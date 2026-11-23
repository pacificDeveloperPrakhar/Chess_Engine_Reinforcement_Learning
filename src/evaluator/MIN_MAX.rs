use crate::structures::game_session::GameSession;
use std::collections::HashMap;

use crate::generator::is_king_checked;
use crate::structures::annotations::Piece;
use crate::structures::game_session::GameStatus;
use crate::structures::state::{apply_move, Move, Node_Min_Max, TTEntry, Zobrist_State, PROMOTIONS, PROMOTION_RANKS};

pub const MATE: f64 = 1.0e9;

// role: 1 = side to move is maximizing (white), -1 = minimizing (black).
// evaluate() is positive when white is better.
pub fn min_max(mut alpha: f64, mut beta: f64, role: i8, session: &mut GameSession, curr: &mut Node_Min_Max, depth: usize) -> f64
{
	if depth == 0
	{
		return curr.evaluate();
	}

	if !curr.expanded
	{
		curr.calculate_children(session);
	}

	if curr.children.is_empty()
	{
		if !curr.is_king_checked
		{
			return 0.0;
		}
		let d = depth as f64;
		return if role == 1 { -(MATE + d) } else { MATE + d };
	}

	let mut best = if role == 1 { f64::NEG_INFINITY } else { f64::INFINITY };
	curr.next_best_move = None;

	for i in 0..curr.children.len()
	{
		let child = &mut curr.children[i];

		let saved = session.clone();
		apply_move(session, child.mv.expect("child without a move"));
		let v = min_max(alpha, beta, -role, session, child, depth - 1);
		*session = saved;

		if role == 1
		{
			if v > best
			{
				best = v;
				curr.next_best_move = Some(i); // only when this child is actually better
			}
			alpha = alpha.max(best);
		}
		else
		{
			if v < best
			{
				best = v;
				curr.next_best_move = Some(i);
			}
			beta = beta.min(best);
		}

		if beta <= alpha
		{
			break;
		}
	}
	curr.value = best;
	best
}

const BY_VALUE: [Piece; 6] = [Piece::P, Piece::N, Piece::B, Piece::R, Piece::Q, Piece::K];

// this function will basically take the board and the side and the square and find what piece is sitting on that
// square
fn piece_on(bb: &[[u64; 7]; 2], side: usize, sq: u64) -> Piece
{
	BY_VALUE.into_iter().find(|p| bb[side][*p as usize] & sq != 0).expect("no piece on square")
}

// priority for searching the board for peices ,like which matters more
// transpostion stored
//killing pieces that are killing most valuable piece starting off from the queen then moing all way to the pawn
// now in killing squares we will first take in consideration for the pawn piece to queen 
// now for the normal moves - that are causing capture of my pieces
// then for the moves that are causing the loss
//quiesence search need to be implemented 
//implementing transposition,quiesence search ,ordering (modified or enhanced version of min max)
pub fn min_max_enhanced(mut alpha: f64, mut beta: f64, role: i8, session: &mut GameSession, curr: &mut Node_Min_Max, depth: usize,map:&mut HashMap<u64,TTEntry>,key:u64) -> f64
{
	// if we have reched the depth of 0 then evaluate the value static evaluation
	if depth == 0
	{
		return curr.evaluate();
	}
	// if the fifty move is reached meaning no pawn moved or no killing was done
	if session.status == GameStatus::DrawFiftyMove
	{
		return 0.0;
	}

	// get the zobrist for key that are like more than 1 or more than that depths
	if curr.mv.is_some()
	{
		if let Some(entry) = Zobrist_State::get(map, key)
		{
			if entry.depth >= depth
			{
				return entry.score;
			}
		}
	}

	let me = session.side_to_move as usize;
	let enemy = 1 - me;
	let bb = session.bitboards;

	let mut legal = [0u64; 64];
	let mut pieces = bb[me][Piece::A as usize];
	// we  are iterating through each piece and then storing it in the legal moves one by one
	// index is what peice it is allocted on the board
	while pieces != 0
	{
		let from = pieces & pieces.wrapping_neg();
		pieces &= pieces - 1;
		legal[from.trailing_zeros() as usize] = session.legal_moves(from);
	}

	let (alpha_start, beta_start) = (alpha, beta);
	let mut best = if role == 1 { f64::NEG_INFINITY } else { f64::INFINITY };
	let mut move_count = 0;
	curr.children.clear();
	curr.next_best_move = None;

	let mut search = |from: u64, to: u64| -> bool
	{

		let piece = piece_on(&bb, me, from);
		// check what promotion is feasible 
		let promos: &[Piece] = if piece == Piece::P && to & PROMOTION_RANKS != 0 { &PROMOTIONS } else { &PROMOTIONS[..1] };
		for &promo in promos
		{
			let mv = Move { from, to, promo };
			let child_key = Zobrist_State::get_from_updated(key, session, me, piece, from, to, Some(promo));

			let saved = session.clone();
			apply_move(session, mv);
			let mut child = Node_Min_Max::new_state_for(session.bitboards, session.side_to_move);
			child.mv = Some(mv);
			let v = min_max_enhanced(alpha, beta, -role, session, &mut child, depth - 1, map, child_key);
			*session = saved;
			move_count += 1;

			let better = if role == 1 { v > best } else { v < best };
			if better
			{
				best = v;
				curr.children = vec![child];
				curr.next_best_move = Some(0);
			}
			if role == 1 { alpha = alpha.max(best); } else { beta = beta.min(best); }
			if beta <= alpha
			{
				return true;
			}
		}
		false
	};

	'search: {
		for victim in BY_VALUE.iter().rev().skip(1)
		{
			let mut targets = bb[enemy][*victim as usize];
			while targets != 0
			{
				let to = targets & targets.wrapping_neg();
				targets &= targets - 1;

				let attackers = Node_Min_Max::attacking_sqaures(to, me, bb);
				for attacker in BY_VALUE
				{
					let mut froms = attackers & bb[me][attacker as usize];
					while froms != 0
					{
						let from = froms & froms.wrapping_neg();
						froms &= froms - 1;
						let i = from.trailing_zeros() as usize;
						if legal[i] & to == 0
						{
							continue;
						}
						legal[i] &= !to;
						if search(from, to) { break 'search; }
					}
				}
			}
		}

		let mut quiet: Vec<(u64, [u64; 3])> = Vec::new();
		let mut pieces = bb[me][Piece::A as usize];
		while pieces != 0
		{
			let from = pieces & pieces.wrapping_neg();
			pieces &= pieces - 1;
			let moves = legal[from.trailing_zeros() as usize];
			if moves == 0
			{
				continue;
			}
			let piece = piece_on(&bb, me, from) as usize;
			let [good, rest] = Node_Min_Max::legal_moves_exclude_loosing_captures(moves, piece, me, bb);
			let losing = Node_Min_Max::legal_loosing_captures(good | rest, from, piece, me, bb);
			quiet.push((from, [good & !losing, rest & !losing, losing]));
		}
		// in the stage 3 i need to implement the quiesence search
		for stage in 0..3
		{
			for &(from, groups) in &quiet
			{
				let mut tos = groups[stage];
				while tos != 0
				{
					let to = tos & tos.wrapping_neg();
					tos &= tos - 1;
					legal[from.trailing_zeros() as usize] &= !to;
					if search(from, to) { break 'search; }
					// do the quiesence search for the loosing capture moves
				}
			}
		}

		for i in 0..64
		{
			let mut tos = legal[i];
			while tos != 0
			{
				let to = tos & tos.wrapping_neg();
				tos &= tos - 1;
				if search(1u64 << i, to) { break 'search; }
			}
		}
	}

	if move_count == 0
	{
		curr.is_king_checked = bb[me][Piece::K as usize] != 0
			&& is_king_checked(bb, if me == 0 { 'w' } else { 'b' }).0 != 0;
		if !curr.is_king_checked
		{
			return 0.0;
		}
		let d = depth as f64;
		return if role == 1 { -(MATE + d) } else { MATE + d };
	}

	curr.value = best;
	if best > alpha_start && best < beta_start
	{
		map.insert(key, TTEntry::new(key, depth, best));
	}
	best
}
