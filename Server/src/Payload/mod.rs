pub mod Request;
pub mod Response;

pub struct Payload
{
	pub id:u64,
	pub data:Vec<u8>,
	pub route:u8,
	pub send:Option<wtransport::SendStream>
}

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

pub async fn  send_resp(send:&mut wtransport::SendStream,resp:&[u8])->anyhow::Result<()>
{
	send.write_all(&[resp.len() as u8]).await?;
	send.write_all(resp).await?;
	Ok(())
}
pub async fn  send_encode_resp(send:&mut wtransport::SendStream,conn_id:u64,resp:Response::Response)->anyhow::Result<()>
{
 //encode the response
 let data = resp.encode();
 let route = match resp
 {
	Response::Response::Fen(_) => 0,
	Response::Response::Move(_) =>1,
	Response::Response::Select(_)=>2
 };
 let route: u8 = route | (1 << 7);
 //form the payload 
 let pd=Payload
 {
	id:conn_id,
	data,
	route,
	send:None,
 };

 let mut frame = vec![route];
 frame.extend_from_slice(&pd.data);
 send.write_all(&[frame.len() as u8]).await?;
 send.write_all(&frame).await?;
 Ok(())
}