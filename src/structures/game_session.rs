#![allow(non_camel_case_types)]

use crate::structures::annotations::Piece;
use crate::generator::{generate_moves_for_king, is_king_checked, possible_moves};
type Boards = [[u64; 7]; 2];

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum mode {
    select,
    moving,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum GameStatus {
    Ongoing,
    Check,
    Checkmate,
    Stalemate,
    DrawFiftyMove,
}

pub struct GameSession {
    pub id: u64,
    pub bitboards: Boards,
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
}
