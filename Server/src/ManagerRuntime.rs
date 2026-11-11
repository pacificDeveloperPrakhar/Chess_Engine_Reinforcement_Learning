use tokio::runtime::Builder;
use tokio::sync::mpsc;
use crate::Payload::Request;

pub fn manager_init()-> tokio::runtime::Runtime{
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("manager-worker")
        .enable_all()
        .build()
        .unwrap();
	return runtime;
}

pub fn create_manager(rt: tokio::runtime::Runtime) -> tokio::task::JoinHandle<()> {
	// buffer size is of 32, so that the manager can handle up to 32 requests concurrently without blocking the sender.
	let (tx, mut rx) = mpsc::channel::<Request::Request>(32);
	let manager_handle: tokio::task::JoinHandle<()> = rt.spawn(async move {
	loop {
			match rx.recv().await {
       			Some(payload) => { 
					tokio::spawn(async move {
				// handle the request here
					});
				 }
       			None => { /* all senders are gone, stop the loop */ }
   			}
			
		 }
	});

	manager_handle
}