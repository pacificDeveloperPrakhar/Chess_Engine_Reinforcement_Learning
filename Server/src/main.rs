use tokio::task::LocalSet;
use ed25519_dalek::Signer;
use der::Encode;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
pub mod parse_args;
use parse_args::parse_args;
use tokio::task::JoinHandle;
mod connections;
mod log;
mod worker;
mod Payload;
mod Certificates;

// Every connection, shared by all workers. One thread runs them all, so Rc<RefCell<..>> is enough.
pub type Store = Rc<RefCell<connections::Connections>>;
pub static CERT:OnceLock<Certificates::certificate_asn::Certificate> = OnceLock::new();
pub static CONNECTIONS:OnceLock<std::sync::Mutex<Vec<JoinHandle<()>>>> = OnceLock::new();

pub fn main()
{
	dotenv::dotenv().ok();
	parse_args().unwrap();
	// endpoint for getting the certificate from the server and send it to the client
	let certificate_server = std::thread::Builder::new().name(String::from("certificate_endpoint")).spawn(|| {
		certificate_endpoint();
	}).unwrap();
	// creating the seperate 1:1 thread then there we will create the multiple task to handle the conncetion from the client
	let thread = std::thread::Builder::new().name(String::from("gateway")).spawn(|| {
		// this will create the runtime for the gateway
		//enable all means to be able to use all sort of networking and other features that tokio provides
		let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
		LocalSet::new().block_on(&rt,gateway_init());
	}).unwrap();
	// keep main alive until the gateway thread finishes, otherwise the process exits immediately
	thread.join().unwrap();
	// keep main alive until the certificate server thread finishes, otherwise the process exits immediately
	certificate_server.join().unwrap();
}
use ed25519_dalek::{SigningKey, VerifyingKey};
use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey};
use std::fs;

pub fn create_certificate()-> Result<Vec<u8>, Box<dyn std::error::Error>>
{

	let pub_key=fs::read_to_string("public_key.pem").expect("generate the ed25519 keys first");
	let priv_key=fs::read_to_string("private_key.pem").expect("generate the ed25519 keys first");
	let signing_key= SigningKey::from_pkcs8_pem(&priv_key).unwrap();
	let verifying_key= VerifyingKey::from_public_key_pem(&pub_key).unwrap();

	//now we will create the tbs Certifcate
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let tbs = Certificates::certificate_asn::tbsCertificate::new(
        // serial_number
        der::asn1::Int::new(&[0x01]).unwrap(),
        // signature (algorithm used to sign this certificate)
        Certificates::certificate_asn::AlgorithmIdentifier {
			// algorithhm being used is the ed25519 algorithm which is represented by the OID
            algorithm: der::asn1::ObjectIdentifier::new_unwrap("1.3.101.112"),
            parameters: None,
        },
        // issuer (CN=localhost)
        vec![der::asn1::SetOfVec::try_from(vec![Certificates::certificate_asn::field_type_and_value {
            oid: der::asn1::ObjectIdentifier::new_unwrap("2.5.4.3"),
            value: der::asn1::Any::encode_from(&der::asn1::Utf8StringRef::new("localhost").unwrap()).unwrap(),
        }]).unwrap()],
        // validity (now -> now + 14 days)
        Certificates::certificate_asn::Validity {
            not_before: Certificates::certificate_asn::Time::UtcTime(der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(now.as_secs())).unwrap()),
            not_after: Certificates::certificate_asn::Time::UtcTime(der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(now.as_secs() + 14 * 24 * 60 * 60)).unwrap()),
        },
        // subject (CN=localhost, same as issuer since self-signed)
        vec![der::asn1::SetOfVec::try_from(vec![Certificates::certificate_asn::field_type_and_value {
            oid: der::asn1::ObjectIdentifier::new_unwrap("2.5.4.3"),
            value: der::asn1::Any::encode_from(&der::asn1::Utf8StringRef::new("localhost").unwrap()).unwrap(),
        }]).unwrap()],
        // subject_public_key_info (our Ed25519 public key)
        Certificates::certificate_asn::SubjectPublicKeyInfo {
            algorithm: Certificates::certificate_asn::AlgorithmIdentifier {
                algorithm: der::asn1::ObjectIdentifier::new_unwrap("1.3.101.112"),
                parameters: None,
            },
            subject_public_key: der::asn1::BitString::from_bytes(verifying_key.as_bytes()).unwrap(),
        },
        // extensions
        None,
    );

	// now we gonna sign the tbs certificate with the signing key
	let signature = signing_key.sign(&tbs.to_der().unwrap());
	let cert = Certificates::certificate_asn::Certificate {
		tbs_certificate: tbs,
		signature_algorithm: Certificates::certificate_asn::AlgorithmIdentifier {
			algorithm: der::asn1::ObjectIdentifier::new_unwrap("1.3.101.112"),
			parameters: None,
		},
		signature: der::asn1::BitString::from_bytes(&signature.to_bytes()).unwrap(),
	};

	let cert = cert.to_der().unwrap();
	let cert_pem = pem::encode(&pem::Pem::new("CERTIFICATE", cert.clone()));
    fs::write("certificate.pem", &cert_pem).expect("could not write certificate to file");

	use sha2::{Digest, Sha256};
	// this returns the bytes raw
	let hash = Sha256::digest(&cert);
	Ok(hash.to_vec())
}

pub fn certificate_endpoint()
{
	let cert_endpoint=std::net::UdpSocket::bind("0.0.0.0:8082").expect("could not bind to address");
	let mut buf=[0u8;1024];
	// \x1b[1;32m = bold green, \x1b[1;36m = bold cyan, \x1b[0m = reset
	println!("\x1b[1;32m✔ certificate endpoint\x1b[0m listening on port \x1b[1;36m8082\x1b[0m");
	loop
	{
		let (len,src)=cert_endpoint.recv_from(&mut buf).expect("could not receive data");
		println!("data received from {}: {:?}",src,&buf[..len]);

		//get the certificate and send it back to the client
		let cert=fs::read("certificate.pem").expect("could not read the certificate file, please ensure to generate the certificate first");
		cert_endpoint.send_to(&cert,src).expect("could not send data");
	}
}

pub async fn gateway_init()
{
	let private_key=fs::read_to_string("private_key.pem").expect("could not read the private key generte the private key first");

	fs::read_to_string("public_key.pem").expect("could not read the public key generate the public key first");
	let signing_key= ed25519_dalek::SigningKey::from_pkcs8_pem(&private_key).expect("could not decode the private key from the file, please ensure its the correct one");	

	use wtransport::{Endpoint, Identity, ServerConfig};

	let identity = Identity::load_pemfiles("certificate.pem", "private_key.pem").await.unwrap();

	// 4. Configure and start the server
    let config = ServerConfig::builder()
        .with_bind_default(8081)
        .with_identity(identity)
        .build();

    let server = Endpoint::server(config).unwrap();

    loop {
        let incoming = server.accept().await; //will stop over here unless a connection comes
        let request = incoming.await.unwrap();
        let connection = request.accept().await.unwrap();
		let mut connection_handle =tokio::spawn(async move {
			println!("client connected: {}", connection.remote_address());
		});
		CONNECTIONS.get_or_init(|| std::sync::Mutex::new(Vec::new())).lock().unwrap().push(connection_handle);
    }
}