use crate::Payload::Request::Request;
use crate::Payload::Response::Response;
pub async fn create_state(connection_id: u64, request: Request) -> Option<Response> {
	if let Request::SetFen(fen) = request {;
		let mut states=&STATE.write().await;
		(*states).insert(connection_id, std::sync::Mutex::new(crate::structers::game_session::GameSession::new()));
		Some(Response::Fen(fen))
	} else {
		None
	}
}

