use crate::Payload::Move;
use crate::Payload::Request::decode_fen;

#[derive(Debug, PartialEq)]
pub enum Response
{
	Fen(String),//need to convert to 4*8*8 bytes
	Move(Move),// need to convert to 8+8+1 byte
	Select(u64), //need to convert to// 8 byte
}

impl Response
{
	pub fn encode(&self) -> Vec<u8>
	{
		match self
		{
			Response::Fen(fen) => encode_fen(fen),
			Response::Move(mv) => mv.encode(),
			Response::Select(sel) => sel.to_be_bytes().to_vec(),
		}
	}

	/// Inverse of encode, picked by payload length like Request::decode.
	/// The server never reads responses; the smoke-test client does.
	#[allow(dead_code)]
	pub fn decode(payload: &[u8]) -> Option<Response>
	{
		match payload.len()
		{
			8 => Some(Response::Select(u64::from_be_bytes(payload.try_into().ok()?))),
			17 => Move::decode(payload).map(Response::Move),
			32 => decode_fen(payload).map(Response::Fen),
			_ => None,
		}
	}
}

pub(crate) fn encode_fen(fen: &str) -> Vec<u8>
{
	let mut squares = [0u8; 64];
	let mut i = 0;
	for c in fen.split(' ').next().unwrap().chars()
	{
		match c
		{
			'/' => {},
			'1'..='8' => i += c.to_digit(10).unwrap() as usize,
			_ => {
				let piece = match c.to_ascii_lowercase()
				{
					'p' => 1,
					'n' => 2,
					'b' => 3,
					'r' => 4,
					'q' => 5,
					'k' => 6,
					_ => panic!("Invalid piece"),
				};
				let color = if c.is_ascii_uppercase() { 0 } else { 1 };
				squares[i] = (piece << 1) | color;
				i += 1;
			},
		}
	}
	squares.chunks(2).map(|p| (p[0] << 4) | p[1]).collect()
}

