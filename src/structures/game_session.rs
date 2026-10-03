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
}