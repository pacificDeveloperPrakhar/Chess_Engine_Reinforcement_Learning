use std::collections::BTreeMap;
pub struct ServerShards
{
	ring:BTreeMap<u64, Server>,

}