pub struct en_passant
{
	ep:Vec<u64>,
	pub row5: u64,
	pub row4: u64,
}
pub enum mode
{
	select,
	moving
}
pub struct GameSession
{
	pub id:u64,
	pub bitboards: [[u64; 7]; 2],
	pub mode:mode,
	pub side_to_move: u8,          // 0 = white, 1 = black (matches new_state_for)
	pub castling: String,          // e.g. "KQkq" or "-"
	pub en_passant: en_passant,   // single-bit bitboard of the target square
	pub halfmove: u32,
	pub fullmove: u32,
}

impl GameSession
{
	pub fn new() -> Self
	{
		GameSession { id:0, bitboards:[[0u64; 7]; 2], mode:mode::select, side_to_move:0, castling:String::from("KQkq"), en_passant:en_passant{ep:Vec::new(),row5:0,row4:0}, halfmove:0, fullmove:1 }
	},
	pub fn select(&mut self,select:u64)
	{
		if self.mode==mode::moving
		{
			self.mode=mode::select;
			return;
		}
	}
	pub fn en_passant_check(&mut self,new_bitboard:[[u64; 7]; 2])
	{
		//check which piece moved 
		let mut pawn_moved_white=new_bitboard[0][Piece::P as usize]&(self.bitboards[0][Piece::P as usize]<<(8*2));
		let mut pawn_moved_black=new_bitboard[1][Piece::P as usize]&(self.bitboards[1][Piece::P as usize]>>(8*2));

		// now check if either of them moved ,it cant be that both of them are non zero
		if pawn_moved_white!=0
		{
			self.en_passant.row4|=pawn_moved_white;
			let temp=self.en_passant.row4;
			while temp!=0
			{
				let square=(1<<temp.trailing_zeros());
				if new_bitboard[1][6] & (square>>1) !=0
				{
					self.en_passant.ep.push(square>>8);
				}
				else if new_bitboard[1][6] & (square<<1) !=0
				{
					self.en_passant.ep.push(square>>8);
				}
				temp&=temp-1;
			}
		}
		else if pawn_moved_black!=0
		{
			self.en_passant.row5|=pawn_moved_black;
			let temp=self.en_passant.row5;
			while temp!=0
			{
				let square=(1<<temp.trailing_zeros());
				if new_bitboard[0][6] & (square>>1) !=0
				{
					self.en_passant.ep.push(square<<8);
				}
				else if new_bitboard[0][6] & (square<<1) !=0
				{
					self.en_passant.ep.push(square<<8);
				}
				temp&=temp-1;
			}
		}
		return 
	}

	const FILE_A: u64 = 0x0101_0101_0101_0101;
const FILE_H: u64 = 0x8080_8080_8080_8080;

pub fn en_passant_select_mode(&mut self, select: u64) -> u64 {
    let mut res: u64 = 0;

    // Guard 1: select must be exactly one square
    if select == 0 || select.count_ones() != 1 {
        return res;
    }

    // Guard 2: no en passant targets available
    if self.en_passant.ep.is_empty() {
        return res;
    }

    // Pick the bitboard for the side to move (0 = white, 1 = black)
    let side = self.side_to_move as usize;
    let pawn_select = self.bitboards[side][Piece::P as usize] & select;

    // Guard 3: the selected square must hold a pawn of the side to move
    if pawn_select == 0 {
        return res;
    }

    // Diagonal capture squares, with edge-file guards to prevent wrap-around
    let capture_squares: u64 = if self.side_to_move == 0 {
        // White moves up the board
        ((pawn_select & !FILE_A) << 7) | ((pawn_select & !FILE_H) << 9)
    } else {
        // Black moves down the board
        ((pawn_select & !FILE_H) >> 7) | ((pawn_select & !FILE_A) >> 9)
    };

    // Collect every en passant square this pawn can capture onto
    for &ep in self.en_passant.ep.iter() {
        res |= ep & capture_squares;
    }

    // Enter moving mode only if at least one en passant square is reachable
    if res != 0 {
        self.mode = mode::moving;
    }

    res
}
}