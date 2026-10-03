use crate::structures::annotations::*;
pub fn bitboards_to_modified_epd(bitboards:[[u64;7];2])->Vec<char>
{
  let mut result=String::from("0000000000000000000000000000000000000000000000000000000000000000");
  let mut result: Vec<char> = result.chars().collect();
  {
    let mut itera=0;
    for i in 0..2
    {
      for j in 0..6
      {
        let board=bitboards[i][j];
        while(itera<64)
        {
        if (board&(1<<itera))!=0
        {
          result[itera]= Piece::as_char(j,i as usize);
        }
         itera+=1;
        }
        itera=0;
      }
    }
  }
 for i in 0..8
 {
  result.insert(i*8+8+i,'/');
 }
 return result;
}

pub fn bitboards_to_fen(bitboards:[[u64;7];2])->String
{
  return String::from("hello");
}

// FEN NOTATION to bitboard logic

use crate::structures::game_session::GameSession;

// Piece order must match your Piece enum indices: P, N, B, R, Q, K
const PIECE_ORDER: &str = "pnbrqk";

// Reading order: the first FEN square (a8) is bit 0, h1 is bit 63.
// This matches how bitboards_to_modified_epd writes square k at character k.
// If your engine uses a1 = bit 0, change this one function to: rank_from_top_inverted
fn square_bit(rank_from_top: usize, file: usize) -> u64
{
	1u64 << (rank_from_top * 8 + file)
}

pub fn fen_to_bitboards(fen: &str) -> Result<FenPosition, String>
{
	let mut fields = fen.split_whitespace();

	let placement = fields.next().ok_or("empty FEN")?;
	let side = fields.next().ok_or("missing side to move")?;
	let castling = fields.next().unwrap_or("-");
	let ep = fields.next().unwrap_or("-");
	let halfmove = fields.next().unwrap_or("0");
	let fullmove = fields.next().unwrap_or("1");

	// 1. Piece placement
	let ranks: Vec<&str> = placement.split('/').collect();
	if ranks.len() != 8
	{
		return Err(format!("expected 8 ranks, found {}", ranks.len()));
	}

	let mut bitboards = [[0u64; 7]; 2];

	for (rank_from_top, rank) in ranks.iter().enumerate()
	{
		let mut file = 0usize;
		for c in rank.chars()
		{
			if let Some(d) = c.to_digit(10)
			{
				if d == 0 || d > 8 { return Err(format!("bad empty-run '{}'", c)); }
				file += d as usize;       // a digit skips that many empty squares
				continue;
			}

			let color = if c.is_ascii_uppercase() { 0 } else { 1 };
			let piece = PIECE_ORDER
				.find(c.to_ascii_lowercase())
				.ok_or(format!("unknown piece '{}'", c))?;

			if file >= 8 { return Err(format!("rank {} overflows 8 files", 8 - rank_from_top)); }
			bitboards[color][piece] |= square_bit(rank_from_top, file);
			file += 1;
		}
		if file != 8
		{
			return Err(format!("rank {} has {} files, expected 8", 8 - rank_from_top, file));
		}
	}

	// Rebuild aggregate occupancy (Piece::A) for each side
	for color in 0..2
	{
		let mut all = 0u64;
		for piece in 0..6 { all |= bitboards[color][piece]; }
		bitboards[color][Piece::A as usize] = all;
	}

	// 2. Side to move
	let side_to_move = match side
	{
		"w" => 0,
		"b" => 1,
		other => return Err(format!("bad side to move '{}'", other)),
	};

	// 3. En passant target square, e.g. "e3"
	let en_passant = if ep == "-" { None } else
	{
		let b = ep.as_bytes();
		if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1])
		{
			return Err(format!("bad en passant square '{}'", ep));
		}
		let file = (b[0] - b'a') as usize;
		let rank_from_top = 8 - (b[1] - b'0') as usize;
		Some(square_bit(rank_from_top, file))
	};

	Ok(GameSession
	{
		bitboards,
		side_to_move,
		castling: castling.to_string(),
		en_passant,
		halfmove: halfmove.parse().map_err(|_| "bad halfmove clock")?,
		fullmove: fullmove.parse().map_err(|_| "bad fullmove number")?,
	})
}