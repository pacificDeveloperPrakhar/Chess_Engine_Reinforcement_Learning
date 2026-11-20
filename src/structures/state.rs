use crate::evaluator::evaluate_score;
use crate::generator::is_king_checked;
use crate::structures::annotations::Piece;
use crate::structures::game_session::{GameSession, GameStatus};

const PROMOTIONS: [Piece; 4] = [Piece::Q, Piece::R, Piece::B, Piece::N];
const PROMOTION_RANKS: u64 = 0xFF00_0000_0000_00FF;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move
{
	pub from: u64,
	pub to: u64,
	pub promo: Piece,
}

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
}

pub fn apply_move(session: &mut GameSession, mv: Move)
{
	session.play_with_promotion(mv.from, mv.promo);
	session.play_with_promotion(mv.to, mv.promo);
}
