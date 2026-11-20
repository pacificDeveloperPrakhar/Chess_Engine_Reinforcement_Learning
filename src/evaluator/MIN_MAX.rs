use crate::structures::game_session::GameSession;
use crate::structures::state::{apply_move, Node_Min_Max};

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
