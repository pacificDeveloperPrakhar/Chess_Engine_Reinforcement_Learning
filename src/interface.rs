use crate::structures::game_session;
use crate::parser::fen_to_bitboards;
use crate::structures::state::Node_Min_Max;
pub fn min_max_eval(fen:&str,depth:usize)->(f64,String)
{
	let mut session=match fen_to_bitboards(fen)
	{
		Ok(session)=>session,
		Err(_)=>{panic!("unable to parse the fen notation")}
	};

	let min_max_node:Node_Min_Max = session.evaluate(depth);
	
	let initial = session.bitboards[session.side_to_move as usize][6]; 
	
	// extracting the enext best min max node 
	let min_max_node_best = match min_max_node.next_best_move
	{
		Some(idx)=>&min_max_node.children[idx],
		None=>panic!("no best move is present")
	};

	let to = min_max_node_best.bitboards[session.side_to_move as usize][6];

	let moves = get_fen_moved(initial & !to, to & !initial);

	let best_score = min_max_node.value;
	// from that node use the next_best_state
	// ue the bitboards of the game session and of the 
	// min max node get the overall board and pass to the get_fen_moved
	// also get the best value from the min max node
	return (best_score,moves);
}

pub fn get_fen_moved(initial:u64,to:u64)->String
{
	
	// get the  position
	let zeros=initial.trailing_zeros();
	let row:usize=(zeros/8).try_into().unwrap();
	let col:usize=(zeros%8).try_into().unwrap();

	let file: char = match col {
		0 => 'a',
		1 => 'b',
		2 => 'c',
		3 => 'd',
		4 => 'e',
		5 => 'f',
		6 => 'g',
		7 => 'h',
		_ => unreachable!("column out of range: {}", col),
	};
	let rank = row + 1;

	// get the destination position
	let to_zeros=to.trailing_zeros();
	let to_row:usize=(to_zeros/8).try_into().unwrap();
	let to_col:usize=(to_zeros%8).try_into().unwrap();

	let to_file: char = match to_col {
		0 => 'a',
		1 => 'b',
		2 => 'c',
		3 => 'd',
		4 => 'e',
		5 => 'f',
		6 => 'g',
		7 => 'h',
		_ => unreachable!("column out of range: {}", to_col),
	};
	let to_rank = to_row + 1;

	return format!("{file}{rank}->{to_file}{to_rank}");
}