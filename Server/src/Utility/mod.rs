pub mod consistent_hash;

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use crate::{CONNECTIONS, CONSISTENT_HASH, MANAGERS};
use crate::Payload::Payload;
use crate::Payload::Request::Request;

pub async fn request_handle(conn_id:u64,buff:Vec<u8>,send:wtransport::SendStream)->anyhow::Result<()>
{
	//check for the connection in the connection hash map
	{
		let connections = CONNECTIONS.get().unwrap().lock().unwrap();
		let _entry = &connections[&conn_id];
	}
	let data=&buff[0..buff.len()-1];
	let route=buff[buff.len()-1];
	// find the id of the manager using the connection id
	let manager_id = CONSISTENT_HASH.get().unwrap().lock().unwrap().get_manager_for_connection(conn_id).unwrap();
	//form the payload
	let pd = Payload
	{
		id:conn_id,
		data:data.to_vec(),
		route,
		send:Some(send)
	};
	// get the manager for the given id
	let tx = {
		let managers = MANAGERS.get().unwrap().lock().unwrap();
		managers[&manager_id].1.clone()
	};
	println!("processing the request");
	// send the payload object to the specific manager
	tx.send(pd).await?;
	return Ok(())
}