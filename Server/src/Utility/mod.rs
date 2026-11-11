pub mod consistent_hash;

use std::collections::HashMap;
use std::sync::Mutex;
use crate::{CONSISTENT_HASH, MANAGERS};
use crate::Payload::Request::Request;

// sends one request from a connection to the manager that owns that connection
pub async fn request_handler(payload: &[u8], connection_id: u64)
{
	// 1. ask the ring which manager owns this connection
	let manager_id = CONSISTENT_HASH
		.get_or_init(|| Mutex::new(consistent_hash::ConsistentHashRing::new(100)))
		.lock().unwrap()
		.get_manager_for_connection(connection_id);

	// 2. decode the bytes into a request
	let request = Request::decode(payload);

	if let (Some(manager_id), Some(request)) = (manager_id, request)
	{
		// 3. copy the manager's sender out of the map, then release the lock before awaiting
		let tx = {
			let managers = MANAGERS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
			let tx = managers.get(&manager_id).map(|(_, tx)| tx.clone());
			tx
		};

		// 4. send the request into that manager's channel
		if let Some(tx) = tx
		{
			let _ = tx.send(request).await;
		}
	}
}
