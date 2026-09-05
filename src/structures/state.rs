use crate::generator::generating_moves_with_king_safety;

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
		// i will have a method to which will take the current node and evaluate the
		// board then attacch the value to the node and return the value of the node
		return 0.0;	
	}
	pub fn calculate_children(&mut self)->()
	{
		self.children=Vec::new();
		let player=1-self.role;
		for i in 0..7
		{
			let mut bitboard=self.bitboards[player as usize][i as usize];
			while bitboard!=0
			{
				let piece_index=bitboard.trailing_zeros();
				bitboard&=bitboard-1;
				let mut m=generating_moves_with_king_safety(self.bitboards,1<<piece_index);
				while m!=0
				{
					let next_move=1<<m.trailing_zeros();
					m=m&m-1;
					let mut bitboard=self.bitboards[player as usize][i as usize];
					// remove the piece from the current position
					bitboard^=(1<<piece_index);
					// move it to the next position
					bitboard|=next_move;
					let mut new_bitboards=self.bitboards;
					new_bitboards[player as usize][i as usize]=bitboard;
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