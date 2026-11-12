pub mod consistent_hash;

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use crate::{CONSISTENT_HASH, MANAGERS};
use crate::Payload::Request::Request;

// sends one request from a connection to the manager that owns that connection,
// waits for the answer, and returns it as bytes (Response::encode).
// An empty Vec means "could not handle it", the same as worker.rs, and the client shows that as an error.
pub async fn request_handler(payload: &[u8], connection_id: u64) -> Vec<u8>
{
	// 1. decode the bytes into a request
	let Some(request) = Request::decode(payload) else {
		println!("undecodable payload ({} bytes): {:02x?}", payload.len(), payload);
		return Vec::new();
	};

	// 2. ask the ring which manager owns this connection
	let manager_id = CONSISTENT_HASH
		.get_or_init(|| Mutex::new(consistent_hash::ConsistentHashRing::new(100)))
		.lock().unwrap()
		.get_manager_for_connection(connection_id);
	let Some(manager_id) = manager_id else {
		println!("no manager on the ring yet");
		return Vec::new();
	};

	// 3. copy the manager's sender out of the map, then release the lock before awaiting
	let tx = {
		let managers = MANAGERS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
		managers.get(&manager_id).map(|(_, tx)| tx.clone())
	};
	let Some(tx) = tx else {
		println!("manager {} is on the ring but not in MANAGERS", manager_id);
		return Vec::new();
	};

	// 4. send the request with a one-time reply channel, then wait for the manager's answer
	let (reply_tx, reply_rx) = oneshot::channel();
	if tx.send((connection_id, request, reply_tx)).await.is_err() {
		return Vec::new(); // manager task has stopped
	}
	match reply_rx.await {
		Ok(response) => response.encode(),
		Err(_) => Vec::new(), // manager dropped the job without answering
	}
}
