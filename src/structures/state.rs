use std::collections::HashMap;

use crate::evaluator::{evaluate_score, game_progress};
use crate::evaluator::PSQT::{PIECE_VALUES, PSQT};
use crate::generator::is_king_checked;
use crate::generator::diagnol_moves::diagnol_attacks;
use crate::generator::horizontal_vertical_moves::horizontal_vertical_attacks;
use crate::generator::l_shape_moves::l_squares;
use crate::generator::one_square_move::one_square_move;
use crate::structures::annotations::Piece;
use crate::structures::game_session::{mode, GameSession, GameStatus};
// the piece that we are promoting to 
pub const PROMOTIONS: [Piece; 4] = [Piece::Q, Piece::R, Piece::B, Piece::N];
// the ranks   that we are targetting for
pub const PROMOTION_RANKS: u64 = 0xFF00_0000_0000_00FF;

#[derive(Clone, Copy, Debug, PartialEq)]
// this will be used to get the promotions
pub struct Move
{
	pub from: u64,
	pub to: u64,
	pub promo: Piece,
}
// ===============================================================================================
const SIDE_KEY: usize = 768;
const CASTLE_KEY: usize = 769;
const EP_KEY: usize = 773;

const KEYS: [u64; 781] = {
	let mut keys = [0u64; 781];
	let mut s: u64 = 2026;
	let mut i = 0;
	while i < 781
	{
		s = s.wrapping_add(0x9E3779B97F4A7C15);
		let mut z = s;
		z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
		keys[i] = z ^ (z >> 31);
		i += 1;
	}
	keys
};

fn piece_key(side: usize, piece: usize, square: usize) -> u64
{
	KEYS[(side * 6 + piece) * 64 + square]
}

fn castling_key(castling: &str) -> u64
{
	let mut h = 0;
	for (i, c) in ['K', 'Q', 'k', 'q'].iter().enumerate()
	{
		if castling.contains(*c)
		{
			h ^= KEYS[CASTLE_KEY + i];
		}
	}
	h
}

fn ep_key(ep_target: u64) -> u64
{
	if ep_target == 0 { 0 } else { KEYS[EP_KEY + (ep_target.trailing_zeros() % 8) as usize] }
}

//extract from the game session
pub struct Zobrist_State
{
    pub bitboards: [[u64; 7]; 2],
    pub mode: mode,
    pub side_to_move: u8,          // 0 = white, 1 = black (matches new_state_for)
    pub castling: String,          // e.g. "KQkq" or "-"
    pub ep_target: u64,
    pub halfmove: u32,
    pub fullmove: u32,
    pub piece_selected: Option<Piece>,
    pub selected: u64,
    pub moves_available: u64,
    pub status: GameStatus,
	pub depth:usize
}

impl Zobrist_State
{
	pub fn from_session(session: &GameSession, depth: usize) -> Zobrist_State
	{
		Zobrist_State
		{
			bitboards: session.bitboards,
			mode: session.mode,
			side_to_move: session.side_to_move,
			castling: session.castling.clone(),
			ep_target: session.ep_target,
			halfmove: session.halfmove,
			fullmove: session.fullmove,
			piece_selected: session.piece_selected,
			selected: session.selected,
			moves_available: session.moves_available,
			status: session.status,
			depth,
		}
	}

	pub fn hash(state:&Zobrist_State)->u64
	{
		let mut h = 0;
		for side in 0..2
		{
			for piece in 0..6
			{
				let mut bb = state.bitboards[side][piece];
				while bb != 0
				{
					let square = bb.trailing_zeros() as usize;
					h ^= piece_key(side, piece, square);
					bb &= bb - 1;
				}
			}
		}
		if state.side_to_move == 1
		{
			h ^= KEYS[SIDE_KEY];
		}
		h ^= castling_key(&state.castling);
		h ^= ep_key(state.ep_target);
		h
	}

	// create zobrist hash from scratch and then add it to the hashmap 
	//if depth give is none then its 0
	pub fn new(session:&GameSession,map:&mut HashMap<u64,TTEntry>,score:f64,depth:usize)->u64
	{
		let state = Zobrist_State::from_session(session, depth);
		let id = Self::hash(&state);
		Self::store(map, id, depth, score);
		id
	}

	pub fn update(map:&mut HashMap<u64,TTEntry>,id:u64,session:&GameSession,color:usize,piece:Piece,from:u64,to:u64,promo:Option<Piece>,depth:usize,score:f64)->u64
	{
		let new_id = Self::get_from_updated(id, session, color, piece, from, to, promo);
		Self::store(map, new_id, depth, score);
		new_id
	}

	// this will be used to get the TTEntry based on the id newly created from the scratch
	pub fn get(map:&HashMap<u64,TTEntry>,id:u64)->Option<TTEntry>
	{
		map.get(&id).copied()
	}

	pub fn get_from_updated(id:u64,session:&GameSession,color:usize,piece:Piece,from:u64,to:u64,promo:Option<Piece>)->u64
	{
		let bb = &session.bitboards;
		let enemy = 1 - color;
		let sq = |b: u64| b.trailing_zeros() as usize;
		let mut h = id;

		h ^= piece_key(color, piece as usize, sq(from));

		let is_pawn = piece == Piece::P;
		let landing = if is_pawn && to & PROMOTION_RANKS != 0
		{
			match promo { Some(p @ (Piece::N | Piece::B | Piece::R | Piece::Q)) => p, _ => Piece::Q }
		}
		else
		{
			piece
		};
		h ^= piece_key(color, landing as usize, sq(to));

		let ep_capture = is_pawn && session.ep_target != 0 && to == session.ep_target;
		let victim = if ep_capture { if color == 0 { to >> 8 } else { to << 8 } } else { to };
		for p in 0..6
		{
			if bb[enemy][p] & victim != 0
			{
				h ^= piece_key(enemy, p, sq(victim));
			}
		}

		if piece == Piece::K
		{
			let (rook_from, rook_to) =
				if to == from << 2 { (from << 3, from << 1) }
				else if to == from >> 2 { (from >> 4, from >> 1) }
				else { (0, 0) };
			if rook_from != 0
			{
				h ^= piece_key(color, Piece::R as usize, sq(rook_from));
				h ^= piece_key(color, Piece::R as usize, sq(rook_to));
			}
		}

		h ^= KEYS[SIDE_KEY];

		let touched = from | to;
		let lost = [
			('K', 0x10 | 0x80),
			('Q', 0x10 | 0x01),
			('k', (1u64 << 60) | (1u64 << 63)),
			('q', (1u64 << 60) | (1u64 << 56)),
		];
		for (i, (right, squares)) in lost.iter().enumerate()
		{
			if session.castling.contains(*right) && touched & squares != 0
			{
				h ^= KEYS[CASTLE_KEY + i];
			}
		}

		h ^= ep_key(session.ep_target);
		let new_ep = if is_pawn && color == 0 && to == from << 16 { from << 8 }
			else if is_pawn && color == 1 && to == from >> 16 { from >> 8 }
			else { 0 };
		h ^= ep_key(new_ep);

		h
	}

	fn store(map:&mut HashMap<u64,TTEntry>,id:u64,depth:usize,score:f64)
	{
		if let Some(old) = map.get(&id)
		{
			if old.depth > depth
			{
				return;
			}
		}
		map.insert(id, TTEntry::new(id, depth, score));
	}
}


#[derive(Clone, Copy, Debug)]
pub struct TTEntry
{
	pub full_hash:u64,
	pub depth:usize,
	pub score:f64,
}
impl TTEntry
{
	pub fn new(full_hash:u64,depth:usize,score:f64)->TTEntry
	{
		TTEntry
		{
			full_hash,
			depth,
			score
		}
	}
}
// ===============================================================================================
pub struct Node_Min_Max
{
	pub bitboards: [[u64; 7]; 2],
	pub children: Vec<Node_Min_Max>,
	pub value: f64,
	pub role: u8,                      // side that just moved; side to move is 1 - role
	pub expanded: bool,                // true once calculate_children has run (even if it found no moves)
	pub next_best_move: Option<usize>, // index into self.children, no borrow needed
	pub is_king_checked: bool,
	pub mv: Option<Move>,
	// pub legal_moves_count:usize //initially will be 0 only then going forward while calculating the moves it will go on calculating
}

impl Node_Min_Max
{
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
			is_king_checked: false,
			mv: None,
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

	pub fn expand(&mut self, session: &mut GameSession, depth: usize)
	{
		if depth == 0
		{
			return;
		}
		if !self.expanded
		{
			self.calculate_children(session);
		}
		for child in self.children.iter_mut()
		{
			let saved = session.clone();
			apply_move(session, child.mv.expect("child without a move"));
			child.expand(session, depth - 1);
			*session = saved;
		}
	}

	pub fn calculate_children(&mut self, session: &mut GameSession) -> bool
	{
		self.children = Vec::new();
		self.expanded = true;
		self.next_best_move = None;

		let player = self.side_to_move();
		self.is_king_checked = self.bitboards[player][Piece::K as usize] != 0
			&& is_king_checked(self.bitboards, if player == 0 { 'w' } else { 'b' }).0 != 0;

		if matches!(
			session.status,
			GameStatus::Checkmate | GameStatus::Stalemate | GameStatus::DrawFiftyMove
		)
		{
			return self.is_king_checked;
		}

		let mut pieces = self.bitboards[player][Piece::A as usize];
		while pieces != 0
		{
			let from = pieces & pieces.wrapping_neg();
			pieces &= pieces - 1;

			let is_pawn = self.bitboards[player][Piece::P as usize] & from != 0;
			let mut moves = session.legal_moves(from);

			while moves != 0
			{
				let to = moves & moves.wrapping_neg();
				moves &= moves - 1;

				let promos: &[Piece] = if is_pawn && to & PROMOTION_RANKS != 0 { &PROMOTIONS } else { &PROMOTIONS[..1] };

				for &promo in promos
				{
					let mv = Move { from, to, promo };
					let saved = session.clone();

					apply_move(session, mv);
					let mut child = Self::new_state_for(session.bitboards, session.side_to_move);
					child.mv = Some(mv);
					self.children.push(child);

					*session = saved;
				}
			}
		}

		self.is_king_checked
	}

	// sorted already based on the priority
	// target is basically the enemey side piece
	pub fn attacking_sqaures(target:u64,role:usize,bitboards:[[u64;7];2])->u64
	{
		const FILE_A: u64 = 0x0101_0101_0101_0101;
		const FILE_H: u64 = 0x8080_8080_8080_8080;
		let mine = &bitboards[role];

		let diag = diagnol_attacks(bitboards, target);
		let line = horizontal_vertical_attacks(bitboards, target);

		let pawn_from = if role == 0
		{
			((target >> 7) & !FILE_A) | ((target >> 9) & !FILE_H)
		}
		else
		{
			((target << 7) & !FILE_H) | ((target << 9) & !FILE_A)
		};

		//start with pawn
		// then go on to from least to most valuable piece
		(pawn_from & mine[Piece::P as usize])
			| (l_squares(bitboards, target) & mine[Piece::N as usize])
			| (diag & mine[Piece::B as usize])
			| (line & mine[Piece::R as usize])
			| ((diag | line) & mine[Piece::Q as usize])
			| (one_square_move(bitboards, target) & mine[Piece::K as usize])
	}

	// all the moves that are available excluding the that are capturing the enemy piece
	// first element all the good one
	// second element all the remaining one
	pub fn legal_moves_exclude_loosing_captures(moves:u64,piece:usize,role:usize,bitboards:[[u64;7];2])->[u64;2]
	{
		let quiet = moves & !bitboards[1 - role][Piece::A as usize];

		let phase = if game_progress(bitboards) == 2 { 1 } else { 0 };
		let row = match piece { 4 => 5, 5 => 4, p => p };
		let value = |sq: usize| PSQT[row][phase][if role == 0 { sq ^ 56 } else { sq }] as i32;

		let (mut sum, mut count) = (0, 0);
		let mut rest = quiet;
		while rest != 0
		{
			sum += value(rest.trailing_zeros() as usize);
			count += 1;
			rest &= rest - 1;
		}

		let mut good = 0;
		let mut rest = quiet;
		while rest != 0
		{
			let to = rest & rest.wrapping_neg();
			if value(to.trailing_zeros() as usize) * count >= sum
			{
				good |= to;
			}
			rest &= rest - 1;
		}
		[good, quiet & !good]
	}

	// after getting the first and second element from the above function get those value
	// add into the one number
	// then pass it to this one
	pub fn legal_loosing_captures(legal_moves:u64,from:u64,piece:usize,role:usize,bitboards:[[u64;7];2])->u64
	{
		let enemy = 1 - role;
		let mut bb = bitboards;
		for i in 0..7
		{
			bb[role][i] &= !from;
		}
		let my_value = PIECE_VALUES[piece];

		let mut losing = 0;
		let mut rest = legal_moves;
		while rest != 0
		{
			let to = rest & rest.wrapping_neg();
			rest &= rest - 1;

			// now for every move get the attacking enemy squares
			let attackers = Self::attacking_sqaures(to, enemy, bb);
			if attackers == 0
			{
				continue;
			}
			let defended = Self::attacking_sqaures(to, role, bb) != 0;
			let cheaper = (0..6).any(|p| attackers & bb[enemy][p] != 0 && PIECE_VALUES[p] < my_value);
			if cheaper || !defended
			{
				losing |= to;
			}
		}
		losing
	}
	// after having the value from the legal loosing captures i will simply take the value
	// and remove it from the result of both from the above functio
}

pub fn apply_move(session: &mut GameSession, mv: Move)
{
	session.play_with_promotion(mv.from, mv.promo);
	session.play_with_promotion(mv.to, mv.promo);
}
