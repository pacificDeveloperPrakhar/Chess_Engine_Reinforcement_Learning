use tokio::task::LocalSet;
use ed25519_dalek::Signer;
use der::Encode;
use std::cell::RefCell;
use std::rc::Rc;
pub mod parse_args;
use parse_args::parse_args;
mod connections;
mod log;
mod worker;
mod Payload;
mod Ceritficates;

// Every connection, shared by all workers. One thread runs them all, so Rc<RefCell<..>> is enough.
pub type Store = Rc<RefCell<connections::Connections>>;

pub fn main()
{
	dotenv::dotenv().ok();
	parse_args().unwrap();
	// creating the seperate 1:1 thread then there we will create the multiple task to handle the conncetion from the client
	let thread = std::thread::Builder::new().name(String::from("gateway")).spawn(|| {
		// this will create the runtime for the gateway
		//enable all means to be able to use all sort of networking and other features that tokio provides
		let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
		LocalSet::new().block_on(&rt,gateway_init());
	}).unwrap();
	// keep main alive until the gateway thread finishes, otherwise the process exits immediately
	thread.join().unwrap();
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
    let tbs = Ceritficates::certificate_asn::tbsCertificate::new(
        // serial_number
        der::asn1::Int::new(&[0x01]).unwrap(),
        // signature (algorithm used to sign this certificate)
        Ceritficates::certificate_asn::AlgorithmIdentifier {
			// algorithhm being used is the ed25519 algorithm which is represented by the OID
            algorithm: der::asn1::ObjectIdentifier::new_unwrap("1.3.101.112"),
            parameters: None,
        },
        // issuer (CN=localhost)
        vec![der::asn1::SetOfVec::try_from(vec![Ceritficates::certificate_asn::field_type_and_value {
            oid: der::asn1::ObjectIdentifier::new_unwrap("2.5.4.3"),
            value: der::asn1::Any::encode_from(&der::asn1::Utf8StringRef::new("localhost").unwrap()).unwrap(),
        }]).unwrap()],
        // validity (now -> now + 14 days)
        Ceritficates::certificate_asn::Validity {
            not_before: Ceritficates::certificate_asn::Time::UtcTime(der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(now.as_secs())).unwrap()),
            not_after: Ceritficates::certificate_asn::Time::UtcTime(der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(now.as_secs() + 14 * 24 * 60 * 60)).unwrap()),
        },
        // subject (CN=localhost, same as issuer since self-signed)
        vec![der::asn1::SetOfVec::try_from(vec![Ceritficates::certificate_asn::field_type_and_value {
            oid: der::asn1::ObjectIdentifier::new_unwrap("2.5.4.3"),
            value: der::asn1::Any::encode_from(&der::asn1::Utf8StringRef::new("localhost").unwrap()).unwrap(),
        }]).unwrap()],
        // subject_public_key_info (our Ed25519 public key)
        Ceritficates::certificate_asn::SubjectPublicKeyInfo {
            algorithm: Ceritficates::certificate_asn::AlgorithmIdentifier {
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
	let cert = Ceritficates::certificate_asn::Certificate {
		tbs_certificate: tbs,
		signature_algorithm: Ceritficates::certificate_asn::AlgorithmIdentifier {
			algorithm: der::asn1::ObjectIdentifier::new_unwrap("1.3.101.112"),
			parameters: None,
		},
		signature: der::asn1::BitString::from_bytes(&signature.to_bytes()).unwrap(),
	};

	let cert = cert.to_der().unwrap();

	use sha2::{Digest, Sha256};
	// this returns the bytes raw
	let hash = Sha256::digest(&cert);
	Ok(hash.to_vec())
}

pub fn certificate_endpoint()
{
	let cert_endpoint=std::net::UdpSocket::bind("0.0.0.0:8082").expect("could not bind to address");
	let mut buf=[0u8;1024];
	loop
	{
		let (len,src)=cert_endpoint.recv_from(&mut buf).expect("could not receive data");
		println!("data received from {}: {:?}",src,&buf[..len]);

		//get the certificate and send it back to the client
		let cert=create_certificate().expect("could not create certificate");
		cert_endpoint.send_to(&cert,src).expect("could not send data");
	}
}

pub async fn gateway_init()
{
}