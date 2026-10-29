pub mod Request;
pub mod Response;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move
{
	pub from : u64,
	pub to : u64,
	pub role:u8,
}

impl Move
{
	// 8 + 8 + 1 bytes, big-endian
	pub fn encode(&self) -> Vec<u8>
	{
		let mut payload = Vec::with_capacity(17);
		payload.extend_from_slice(&self.from.to_be_bytes());
		payload.extend_from_slice(&self.to.to_be_bytes());
		payload.push(self.role);
		payload
	}

	pub fn decode(payload: &[u8]) -> Option<Move>
	{
		if payload.len() != 17
		{
			return None;
		}
		Some(Move {
			from: u64::from_be_bytes(payload[0..8].try_into().ok()?),
			to: u64::from_be_bytes(payload[8..16].try_into().ok()?),
			role: payload[16],
		})
	}
}
