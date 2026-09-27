use crate::generator::{generating_moves_with_king_safety, generate_moves_for_king};
use crate::structures::annotations::Piece;

pub struct Node_Min_Max
{
	pub bitboards:[[u64; 7]; 2],
	pub children:Vec<Node_Min_Max>,
	pub value:f64,
	pub role:u8
}

impl Node_Min_Max
{
	pub fn new_state(bitboards:[[u64; 7]; 2])->Node_Min_Max
	{
		Node_Min_Max{bitboards:bitboards,children:Vec::new(),value:0.0,role:0}
	} 

	pub fn evaluate(&mut self)->f64
	{
		return 0.0;	
	}

	pub fn calculate_children(&mut self)->()
	{
		self.children=Vec::new();
		let player=1-self.role;
		for i in 0..6   // FIX: exclude Piece::A (index 6) — not a real movable piece
		{
			let mut bitboard=self.bitboards[player as usize][i as usize];
			while bitboard!=0
			{
				let piece_index=bitboard.trailing_zeros();
				bitboard&=bitboard-1;

				// FIX: king moves must be filtered against enemy-attacked squares
				let mut m = if i == Piece::K as usize {
					generate_moves_for_king(self.bitboards, 1<<piece_index)
				} else {
					generating_moves_with_king_safety(self.bitboards, 1<<piece_index)
				};

				while m!=0
				{
					let next_move=1<<m.trailing_zeros();
					m=m&m-1;
					let mut piece_bitboard=self.bitboards[player as usize][i as usize];
					piece_bitboard^=1<<piece_index;
					piece_bitboard|=next_move;
					let mut new_bitboards=self.bitboards;
					new_bitboards[player as usize][i as usize]=piece_bitboard;
					// FIX: keep the mover's own aggregate occupancy board in sync
					new_bitboards[player as usize][Piece::A as usize] ^= (1<<piece_index) | next_move;
					// remove the enemy piece if it is present in the next position
					for j in 0..7
					{
						new_bitboards[1-player as usize][j as usize]&=!next_move;
					}
					let mut child=Node_Min_Max::new_state(new_bitboards);
					child.role=player;
					child.value=0.0;
					self.children.push(child);
				}
			}
		}
	}
}