use crate::evaluator::evaluate_score;
use crate::generator::{generating_moves_with_king_safety, generate_moves_for_king};
use crate::structures::annotations::Piece;

pub struct Node_Min_Max
{
	pub bitboards: [[u64; 7]; 2],
	pub children: Vec<Node_Min_Max>,
	pub value: f64,
	pub role: u8,        // side that just moved; side to move is 1 - role
	pub expanded: bool,  // true once calculate_children has run (even if it found no moves)
	pub next_best_move: Option<usize>, // index into self.children, no borrow needed
}

impl Node_Min_Max
{
	pub fn expand(&mut self, depth: usize)
	{
		if depth == 0
		{
			return;
		}
		if !self.expanded
		{
			self.calculate_children();
		}
		for child in self.children.iter_mut()
		{
			child.expand(depth - 1);
		}
	}

	// White to move
	pub fn new_state(bitboards: [[u64; 7]; 2]) -> Self
	{
		Self::new_state_for(bitboards, 0)
	}

	// side_to_move: 0 = white, 1 = black.
	pub fn new_state_for(bitboards: [[u64; 7]; 2], side_to_move: u8) -> Self
	{
		Node_Min_Max
		{
			bitboards,
			children: Vec::new(),
			value: 0.0,
			role: 1 - side_to_move,
			expanded: false,
			next_best_move: None,
		}
	}

	pub fn side_to_move(&self) -> usize
	{
		1 - self.role as usize
	}

	pub fn evaluate(&self) -> f64
	{
		evaluate_score(self.bitboards)
	}

	// Returns the best child, if min_max has chosen one
	pub fn best_child(&self) -> Option<&Node_Min_Max>
	{
		self.next_best_move.map(|i| &self.children[i])
	}

	pub fn calculate_children(&mut self)
	{
		self.children = Vec::new();
		self.expanded = true;
		self.next_best_move = None;

		let player = self.side_to_move(); // FIX: was self.role (the side that just moved)
		let enemy = 1 - player;

		for i in 0..6 // exclude Piece::A (index 6), the aggregate occupancy board
		{
			let mut bitboard = self.bitboards[player][i];
			while bitboard != 0
			{
				let piece_index = bitboard.trailing_zeros();
				bitboard &= bitboard - 1;
				let from: u64 = 1u64 << piece_index;

				let mut m = if i == Piece::K as usize {
					generate_moves_for_king(self.bitboards, from)
				} else {
					generating_moves_with_king_safety(self.bitboards, from)
				};

				while m != 0
				{
					let next_move: u64 = 1u64 << m.trailing_zeros();
					m &= m - 1;

					let mut new_bitboards = self.bitboards;

					new_bitboards[player][i] &= !from;
					new_bitboards[player][i] |= next_move;

					new_bitboards[player][Piece::A as usize] &= !from;
					new_bitboards[player][Piece::A as usize] |= next_move;

					for j in 0..7
					{
						new_bitboards[enemy][j] &= !next_move;
					}

					// the child's side to move is the enemy
					self.children.push(Self::new_state_for(new_bitboards, enemy as u8));
				}
			}
		}
	}
}