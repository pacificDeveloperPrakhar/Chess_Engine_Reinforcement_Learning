use std::env;
use anyhow::Result;
use crate::Ceritficates::keys;
pub fn parse_args() -> Result<()>
{
	let args: Vec<String> = env::args().collect();
	for arg in args.iter()
	{
		match arg.as_str()
		{
			"--help" => {
				help();
				std::process::exit(0);
			},
			"--version" => {
				println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
				std::process::exit(0);
			},
			"--generate" => {
				let (signing_key, verifying_key) = keys::generate_keypair().unwrap();
				keys::save_keypair_to_files(&signing_key, &verifying_key).unwrap();
				std::process::exit(0);
			},
			_ => {}
		}
	}
	return Ok(());
}

// help flag this will display the help message and exit the program and how to use the program
pub fn help()
{
	println!("Usage: {} [OPTIONS]", env!("CARGO_PKG_NAME"));
	println!("Options:");
	println!("  --help     Display this help message");
	println!("  --version  Display the version information");
	println!("  --generate Generate keypair files");
}
