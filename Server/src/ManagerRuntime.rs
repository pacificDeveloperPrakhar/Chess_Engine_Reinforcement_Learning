use std::collections::HashMap;
use std::sync::Mutex;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use crate::Payload::Request::Request;
use crate::Payload::Response::Response;

// one unit of work for a manager: which connection asked, the request,
// and a one-time channel to send the answer back on
pub type Job = (u64, Request, oneshot::Sender<Response>);

// what MANAGERS stores for each manager: its task handle and the channel to send it jobs
pub type ManagerEntry = (JoinHandle<()>, mpsc::Sender<Job>);

pub fn manager_init()-> tokio::runtime::Runtime{
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("manager-worker")
        .enable_all()
        .build()
        .unwrap();
	return runtime;
}

pub fn create_manager(rt: &tokio::runtime::Runtime,gateway_rt:tokio::runtime::Runtime) -> ManagerEntry {
	// buffer size is of 32, so that the manager can hold up to 32 waiting jobs before the sender has to wait.
	let (tx, mut rx) = mpsc::channel::<Job>(32);
	let manager_handle: JoinHandle<()> = rt.spawn(async move {
		// ends when every sender is dropped, instead of spinning forever
		while let Some((connection_id, request, reply_tx)) = rx.recv().await {
			tokio::spawn(async move {
				let response = handle_request(request);
				// the receiver may already be gone (client left); nothing to do then
				let _ = reply_tx.send(response);

				// hand a task to the gateway runtime: it looks the connection up by id and messages the client
				gateway_rt.spawn(async move {
					notify_client(connection_id).await;
				});
			});
		}
	});

	(manager_handle, tx)
}

// same answers as worker.rs handle_request.
// TODO: there is no per-connection board here yet, so GetFen always answers the start position
// and SetFen only echoes the board back instead of saving it.
fn handle_request(request: Request) -> Response {
	match request {
		Request::GetFen => Response::Fen(crate::connections::START_FEN.to_string()),
		Request::SetFen(fen) => Response::Fen(fen),
		Request::Select(square) => Response::Select(square),
		Request::Move(mv) => Response::Move(mv),
	}
}

// runs on the gateway runtime: finds the connection by id and sends it a short message.
// It is sent as a datagram (a one-off message), so a client that does not read datagrams just drops it.
async fn notify_client(connection_id: u64) {
	// clone the connection out of the map, then release the lock before using it
	let connection = {
		let connections = crate::CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
		connections.get(&connection_id).map(|(_, conn)| conn.clone())
	};
	match connection {
		Some(conn) => {
			if let Err(e) = conn.send_datagram(b"received message") {
				println!("could not notify {}: {:?}", connection_id, e);
			}
		}
		None => println!("connection {} already gone; nothing to notify", connection_id),
	}
}

// creates a manager and stores its handle and channel in MANAGERS under its id
pub fn add_manager(rt: &tokio::runtime::Runtime, manager_id: u64, gateway_rt: tokio::runtime::Runtime) {
	let entry: ManagerEntry = create_manager(rt,gateway_rt);
	crate::MANAGERS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap()
		.insert(manager_id, entry);
}
