use crate::structures::state::Node_Min_Max;

const MATE: f64 = 1.0e9;

// role: 1 = side to move is maximizing (white), -1 = minimizing (black).
// evaluate() is positive when white is better.
pub fn min_max(mut alpha: f64, mut beta: f64, role: i8, curr: &mut Node_Min_Max, depth: usize) -> f64
{
	if depth == 0
	{
		return curr.evaluate();
	}

	if !curr.expanded
	{
		curr.calculate_children();
	}

	if curr.children.is_empty()
	{

		let d = depth as f64;
		return if role == 1 { -(MATE + d) } else { MATE + d };
	}

	let mut best = if role == 1 { f64::NEG_INFINITY } else { f64::INFINITY };

	for child in curr.children.iter_mut()
	{
		let v = min_max(alpha, beta, -role, child, depth - 1);
		if role == 1
		{
			best = best.max(v);
			alpha = alpha.max(best);
		}
		else
		{
			best = best.min(v);
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