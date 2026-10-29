use crate::Payload::Move;
use crate::Payload::Response::encode_fen;
// based on the payload length the request will be classified
#[derive(Debug, PartialEq)]
pub enum Request
{
    GetFen,//1 byte
    SetFen(String),//4*8*8 bytes
	Move(Move),// 8+8+1 byte
	Select(u64), // 8 byte
}

use std::convert::TryInto;

impl Request
{
	/// Returns None for an unknown length or a malformed payload,
	/// so a bad client frame can never panic a stream task.
	pub fn decode(payload: &[u8]) -> Option<Request>
	{
	match payload.len()
	{
		1 => Some(Request::GetFen),
		8 => Some(Request::Select(u64::from_be_bytes(payload.try_into().ok()?))),
		17 => Move::decode(payload).map(Request::Move),
		32 => decode_fen(payload).map(Request::SetFen),
		_ => None,
	}
}

	/// Inverse of decode. The server never sends requests; the smoke-test client does.
	#[allow(dead_code)]
	pub fn encode(&self) -> Vec<u8>
	{
		match self
		{
			Request::GetFen => vec![0x01],
			Request::Select(square) => square.to_be_bytes().to_vec(),
			Request::Move(mv) => mv.encode(),
			Request::SetFen(fen) => encode_fen(fen),
		}
	}
}

pub(crate) fn decode_fen(data: &[u8]) -> Option<String>
{
	let mut fen = String::new();
	for rank in 0..8
	{
		let mut empty = 0;
		for file in 0..8
		{
			let i = rank * 8 + file;
			let byte = data[i / 2];
			let nibble = if i % 2 == 0 { byte >> 4 } else { byte & 0x0F };
			let piece = nibble >> 1;
			if piece == 0
			{
				empty += 1;
				continue;
			}
			if empty > 0
			{
				fen.push_str(&empty.to_string());
				empty = 0;
			}
			let c = match piece
			{
				1 => 'p',
				2 => 'n',
				3 => 'b',
				4 => 'r',
				5 => 'q',
				6 => 'k',
				_ => return None,
			};
			fen.push(if nibble & 1 == 0 { c.to_ascii_uppercase() } else { c });
		}
		if empty > 0
		{
			fen.push_str(&empty.to_string());
		}
		if rank < 7
		{
			fen.push('/');
		}
	}
	Some(fen)
}
