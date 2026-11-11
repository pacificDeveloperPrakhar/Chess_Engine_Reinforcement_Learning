use tokio::task::LocalSet;
use p256::ecdsa::signature::Signer;
use der::Encode;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
mod ManagerRuntime;
pub mod parse_args;
pub mod Utility;
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
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub static CONNECTIONS: OnceLock<std::sync::Mutex<HashMap<u64, JoinHandle<()>>>> = OnceLock::new();
pub static MANAGERS: OnceLock<std::sync::Mutex<HashMap<u64, JoinHandle<()>>>> = OnceLock::new();
static NEXT_ID_SERVER: AtomicU64 = AtomicU64::new(0);
static NEXT_ID_CONNECTION: AtomicU64 = AtomicU64::new(9223372036854775808);

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
	// ======================================================SPAWNING THE MANAGER=============================================================================
	// Creating the Manager runtime for the server to handle the request from the client
	let manager_rt = crate::ManagerRuntime::manager_init();
	// ========================================================================================================================================================
	// keep main alive until the gateway thread finishes, otherwise the process exits immediately
	thread.join().unwrap();
	// keep main alive until the certificate server thread finishes, otherwise the process exits immediately
	certificate_server.join().unwrap();
	// dont have to do the same for the manager runtime as we did the same for the thread of the certificate server and the gateway server as the manager runtime is running in the background and will be terminated when the main thread exits. 
}
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey};
use std::fs;

// OIDs (globally unique names) used in the certificate
// ecdsa-with-SHA256: "signed with ECDSA, message hashed with SHA-256"
const OID_ECDSA_WITH_SHA256: &str = "1.2.840.10045.4.3.2";
// id-ecPublicKey: "this public key is an elliptic curve key"
const OID_EC_PUBLIC_KEY: &str = "1.2.840.10045.2.1";
// prime256v1: "the curve is P-256"
const OID_P256_CURVE: &str = "1.2.840.10045.3.1.7";
// subjectAltName: "the names (DNS names, IPs) this certificate is for"
const OID_SUBJECT_ALT_NAME: &str = "2.5.29.17";

pub fn create_certificate()-> Result<Vec<u8>, Box<dyn std::error::Error>>
{

	let pub_key=fs::read_to_string("public_key.pem").expect("generate the ECDSA P-256 keys first (--generate)");
	let priv_key=fs::read_to_string("private_key.pem").expect("generate the ECDSA P-256 keys first (--generate)");
	let signing_key= SigningKey::from_pkcs8_pem(&priv_key).unwrap();
	let verifying_key= VerifyingKey::from_public_key_pem(&pub_key).unwrap();

	//now we will create the tbs Certifcate
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let tbs = Certificates::certificate_asn::tbsCertificate::new(
        // serial_number
        der::asn1::Int::new(&[0x01]).unwrap(),
        // signature (algorithm used to sign this certificate)
        Certificates::certificate_asn::AlgorithmIdentifier {
			// ECDSA with SHA-256; RFC 5758 says parameters must be absent here
            algorithm: der::asn1::ObjectIdentifier::new_unwrap(OID_ECDSA_WITH_SHA256),
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
        // subject_public_key_info (our ECDSA P-256 public key)
        Certificates::certificate_asn::SubjectPublicKeyInfo {
            algorithm: Certificates::certificate_asn::AlgorithmIdentifier {
                // "elliptic curve key", and the parameter says which curve: P-256
                algorithm: der::asn1::ObjectIdentifier::new_unwrap(OID_EC_PUBLIC_KEY),
                parameters: Some(der::asn1::Any::encode_from(&der::asn1::ObjectIdentifier::new_unwrap(OID_P256_CURVE)).unwrap()),
            },
            // the public key is a point (x, y) on the curve, sent uncompressed:
            // 0x04 + 32 bytes of x + 32 bytes of y = 65 bytes
            subject_public_key: der::asn1::BitString::from_bytes(verifying_key.to_encoded_point(false).as_bytes()).unwrap(),
        },
        // extensions: Chrome refuses a certificate with no extensions at all,
        // so we add subjectAltName (the names this certificate is for)
        Some(vec![Certificates::certificate_asn::Extension {
            exten_id: der::asn1::ObjectIdentifier::new_unwrap(OID_SUBJECT_ALT_NAME),
            critical: false,
            // the extension's value is the DER bytes of SubjectAltName, wrapped in an OCTET STRING
            extn_value: der::asn1::OctetString::new(
                Certificates::certificate_asn::SubjectAltName {
                    dns_name: der::asn1::Ia5String::new("localhost").unwrap(),
                    ip_address: der::asn1::OctetString::new(vec![127, 0, 0, 1]).unwrap(),
                }.to_der().unwrap(),
            ).unwrap(),
        }]),
    );

	// now we gonna sign the tbs certificate with the signing key
	// sign() hashes the bytes with SHA-256 first, then signs the hash with ECDSA
	let signature: Signature = signing_key.sign(&tbs.to_der().unwrap());
	let cert = Certificates::certificate_asn::Certificate {
		tbs_certificate: tbs,
		signature_algorithm: Certificates::certificate_asn::AlgorithmIdentifier {
			algorithm: der::asn1::ObjectIdentifier::new_unwrap(OID_ECDSA_WITH_SHA256),
			parameters: None,
		},
		// an ECDSA signature is two numbers (r, s); X.509 stores them DER-encoded
		signature: der::asn1::BitString::from_bytes(signature.to_der().as_bytes()).unwrap(),
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
	// check the key file holds an ECDSA P-256 key before starting
	let _signing_key= SigningKey::from_pkcs8_pem(&private_key).expect("could not decode the private key as ECDSA P-256, run --generate and --gen_cert again");

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
        let request = match incoming.await
		{
			Ok(request) => request,
			Err(e) => {
				println!("Error accepting connection: {:?}", e);
				continue;
		}
		};
        let connection: wtransport::Connection = request.accept().await.unwrap();
		let id = NEXT_ID_CONNECTION.fetch_add(1, Ordering::Relaxed);
		let connection_handle = tokio::spawn(async move {
    		println!("client connected: {}", connection.remote_address());
    		let connection_close = connection.closed().await;
    		println!("client disconnected: {}", connection.remote_address());
    		println!("connection closed: {:?}", connection_close);
    		// remove this connection's own entry when it closes
    		CONNECTIONS.get_or_init(|| std::sync::Mutex::new(HashMap::new())).lock().unwrap().remove(&id);
		});
		CONNECTIONS.get_or_init(|| std::sync::Mutex::new(HashMap::new())).lock().unwrap().insert(id, connection_handle);
    	}
}