// ECDSA keys on the P-256 curve (also called secp256r1 or prime256v1).
// Every browser must accept this key type for WebTransport certificate pinning.
use p256::ecdsa::{SigningKey,VerifyingKey};
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey};
use der::pem::LineEnding;
use rand::SeedableRng;
use std::fs;
use std::env;

pub fn generate_keypair() -> Result<(SigningKey, VerifyingKey), Box<dyn std::error::Error>> {
	let seed = env::var("CERT_RAND")?.parse::<u64>()?;
	// private key: a random number between 1 and the size of the curve's group
	let signing_key = SigningKey::random(&mut rand::rngs::StdRng::seed_from_u64(seed));
	// public key: a point on the curve, worked out from the private key
	let verifying_key = *signing_key.verifying_key();
	Ok((signing_key, verifying_key))
}

pub fn save_keypair_to_files(signing_key:&SigningKey, verifying_key:&VerifyingKey) -> Result<(), Box<dyn std::error::Error>> {
	// PKCS#8 PEM for the private key, SubjectPublicKeyInfo PEM for the public key
	let private_key_pem = signing_key.to_pkcs8_pem(LineEnding::LF)?;
	let public_key_pem = verifying_key.to_public_key_pem(LineEnding::LF)?;
	fs::write("private_key.pem", private_key_pem.as_bytes())?;
	fs::write("public_key.pem", public_key_pem.as_bytes())?;
	Ok(())
}
