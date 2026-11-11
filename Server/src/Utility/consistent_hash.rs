use std::collections::BTreeMap;
use std::collections::HashMap;

pub struct Manager
{
	pub id: u64,
	pub ip: String,
	pub port: u16,
}

pub struct ConsistentHashRing
{
	ring: BTreeMap<u64, u64>,
	duplicacy_factor: u64,
	server_hash: HashMap<u64,Manager>,
	connection_hash: HashMap<u64, Connection>,
}