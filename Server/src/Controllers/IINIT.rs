pub async fn(connection_id: u64, request: Request) -> Option<Response> {
	let manager_id = crate::CONSISTENT_HASH_RING
		.get_or_init(|| Mutex::new(ConsistentHashRing::new(3)))
		.lock()
		.unwrap()
		.get_manager_for_connection(connection_id)?;

	let managers = crate::MANAGERS.get_or_init(|| Mutex::new(HashMap::new()));
	let entry = managers.lock().unwrap().get(&manager_id)?.clone();

	let (reply_tx, reply_rx) = oneshot::channel();
	entry.sender.send((connection_id, request, reply_tx)).ok()?;

	reply_rx.await.ok()
}