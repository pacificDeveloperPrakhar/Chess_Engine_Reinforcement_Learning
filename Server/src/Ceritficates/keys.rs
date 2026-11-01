use ed25519_dalek::{SigningKey,VerifyingKey};
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use std::fs;
use std::env;

pub fn generate_keypair() -> Result<(SigningKey, VerifyingKey), Box<dyn std::error::Error>> {
	let seed = env::var("CERT_RAND").unwrap()?.parse::<u64>().unwrap()?;
	let signing_key = SigningKey::generate(&mut rand::rngs::StdRng::seed_from_u64(seed));
	let verifying_key = signing_key.verifying_key();
	Ok((signing_key, verifying_key))
}

pub fn save_keypair_to_files(signing_key:&SigningKey, verifying_key:&VerifyingKey) -> Result<(), Box<dyn std::error::Error>> {
	let private_key_pem = signing_key.to_pkcs8_pem()?;
	let public_key_pem = verifying_key.to_public_key_pem()?;
	fs.write("private_key.pem", private_key_pem.as_bytes())?;
	fs.write("public_key.pem", public_key_pem.as_bytes())?;
	Ok(())`
}