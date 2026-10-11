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
          result[(7 - itera/8)*8 + itera%8]= Piece::as_char(j,i as usize);
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
  let mut result=String::new();
  for rank in (0..8).rev()
  {
    let mut empty=0;
    for file in 0..8
    {
      let bit=1u64<<(rank*8+file);
      let mut found:Option<char>=None;
      for i in 0..2
      {
        for j in 0..6
        {
          if bitboards[i][j]&bit!=0
          {
            found=Some(Piece::as_char(j,i as usize));
          }
        }
      }
      match found
      {
        Some(c)=>
        {
          if empty>0
          {
            result.push_str(&empty.to_string());
            empty=0;
          }
          result.push(c);
        }
        None=>empty+=1,
      }
    }
    if empty>0
    {
      result.push_str(&empty.to_string());
    }
    if rank>0
    {
      result.push('/');
    }
  }
  return result;
}

// FEN NOTATION to bitboard logic

use crate::structures::game_session::GameSession;
//this is matching the enum order of the pieces in the enum
const PIECE_ORDER: &str = "pnbrkq";

// Reading order: the first FEN square (a1) is bit 0, h8 is bit 63.
// This matches how bitboards_to_modified_epd writes square k at character k.
// basically the order is like this
//       a   b   c   d   e   f   g   h
//  8   57  58  59  60  61  62  63  64
//  7   49  50  51  52  53  54  55  56
//  6   41  42  43  44  45  46  47  48
//  5   33  34  35  36  37  38  39  40
//  4   25  26  27  28  29  30  31  32
//  3   17  18  19  20  21  22  23  24
//  2    9  10  11  12  13  14  15  16
//  1    1   2   3   4   5   6   7   8
fn square_bit(rank_from_top: usize, file: usize) -> u64
{
	1u64 << ((7 - rank_from_top) * 8 + file)
}

pub fn fen_to_bitboards(fen: &str) -> Result<GameSession, String>
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
	let en_passant = if ep == "-" { 0 } else
	{
		let b = ep.as_bytes();
		if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1])
		{
			return Err(format!("bad en passant square '{}'", ep));
		}
		let file = (b[0] - b'a') as usize;
		let rank_from_top = 8 - (b[1] - b'0') as usize;
		square_bit(rank_from_top, file)
	};

	
	let mut session = GameSession::new();
	session.bitboards = bitboards;
	session.side_to_move = side_to_move;
	session.castling = castling.to_string();
	session.ep_target = en_passant;
	session.halfmove = halfmove.parse().map_err(|_| "bad halfmove clock")?;
	session.fullmove = fullmove.parse().map_err(|_| "bad fullmove number")?;
	Ok(session)
}