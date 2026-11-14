use crate::Payload::Request::Request;
use crate::Payload::Response::Response;
use chess_engine::structures::pgn_notation_to_bitboards;
use chess_engine::structures::state::Node_Min_Max;
use chess_engine::evaluator::MIN_MAX::min_max;
pub async fn evaluate_state(req::Request::SetFen)->Response::Select
{
	// take the the string of the fen convert to the bitboard
	let bitboards:[[u8;7];2]=pgn_notation_to_bitboards(&SetFen);
	let node_min_max=Node_Min_Max::new(bitboards);
	// now perform the minmax on the given node
	let evaluation_score=node_min_max.evaluate();
	return Response::Select(evaluation_score.to_bits());
}