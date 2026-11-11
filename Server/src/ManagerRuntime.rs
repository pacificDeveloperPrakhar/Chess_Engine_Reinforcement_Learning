use std::collections::HashMap;
use std::sync::Mutex;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use crate::Payload::Request;

// what MANAGERS stores for each manager: its task handle and the channel to send it requests
pub type ManagerEntry = (JoinHandle<()>, mpsc::Sender<Request::Request>);

pub fn manager_init()-> tokio::runtime::Runtime{
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("manager-worker")
        .enable_all()
        .build()
        .unwrap();
	return runtime;
}

pub fn create_manager(rt: &tokio::runtime::Runtime) -> (JoinHandle<()>, mpsc::Sender<Request::Request>) {
	// buffer size is of 32, so that the manager can handle up to 32 requests concurrently without blocking the sender.
	let (tx, mut rx) = mpsc::channel::<Request::Request>(32);
	let manager_handle: JoinHandle<()> = rt.spawn(async move {
		// ends when every sender is dropped, instead of spinning forever
		while let Some(request) = rx.recv().await {
			tokio::spawn(async move {
				// handle the request here
				println!("manager got request: {:?}", request);
			});
		}
	});

	(manager_handle, tx)
}

// creates a manager and stores its handle and channel in MANAGERS under its id
pub fn add_manager(rt: &tokio::runtime::Runtime, manager_id: u64) {
	let entry: ManagerEntry = create_manager(rt);
	crate::MANAGERS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap()
		.insert(manager_id, entry);
}
