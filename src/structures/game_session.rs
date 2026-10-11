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
    pub ep_target: u64,             //the square available for the en pssant destination,will be calculating victim with this field only
    pub halfmove: u32,              // counts plies (single moves by either side) since the last capture or pawn move. It exists for the fifty-move rule. It resets to 0 on any capture or pawn move, and otherwise goes up by 1 after every move by either side.
    pub fullmove: u32,           // is the move counter for the game record. It starts at 1 and goes up by 1 after Black moves. It never resets, and it has no effect on the rules.
    pub piece_selected: Option<Piece>, 
    pub selected: u64,
    pub moves_available: u64,   //moves available for the selected piece 
    pub status: GameStatus,
}

impl GameSession {
    pub fn new() -> Self {
        GameSession {
            id: 0,
            bitboards: [[0u64; 7]; 2],
            mode: mode::select,
            side_to_move: 0,
            castling: String::from("KQkq"),
            ep_target: 0,
            halfmove: 0,
            fullmove: 1,
            piece_selected: None,
            selected: 0,
            moves_available: 0,
            status: GameStatus::Ongoing,
        }
    }
}

const P: usize = Piece::P as usize;
const N: usize = Piece::N as usize;
const B: usize = Piece::B as usize;
const R: usize = Piece::R as usize;
const K: usize = Piece::K as usize;
const Q: usize = Piece::Q as usize;
const ALL: usize = Piece::A as usize;

const RANK_1: u64 = 0x0000_0000_0000_00FF;
const RANK_8: u64 = 0xFF00_0000_0000_0000;

// while making the move ,i have validated and generated the move already so now we are just moving the piece

pub fn make_move(mut bb:Boards,side:usize,from:u64,to:u64,ep_target:u64,promoted_piece:Option<Piece>)->Boards{
    let enemy_side = 1-side;
    let is_pawn = bb[side][P] & from != 0;
    let is_king = bb[side][K] & from != 0;

    let is_ep_capture=is_pawn && ep_target!=0 && to==ep_target;

    // this will just give u the victim square for any piece about to be captured
    // if there is no piece to be capture then it will just give he empty square
    let victim_square = if is_ep_capture {
      {
        if side==0{
            ep_target>>8
        }
        else {
            ep_target<<8
        }
      }
    }
    else
    {
        to
    }

    // remove the victim piece from the bitboard
    for i in 0..7 {
        bb[enemy_side][i] &= !victim_square;
    }
    //now remove and add the piece in the bit board
    for i in 0..7 {
        match bb[side][i] & from {
            0 => continue,
            _ => {
                bb[side][i] &= !from|to;
                break;
            }
        }
    }

    // now for the castling moves
    let (rook_src_sqr, rook_dest_sqr) = if is_king 
    {
        if from == to >> 2
        {
            (from << 3, from << 1)
        }
        else if from == to << 2
        {
            (from >> 4,from >> 1)
        }
        else
        {
            (0,0)
        }
    }

    if rook_src_sqr != 0
    {
        bb[side][R] &= !rook_src_sqr|rook_dest_sqr;
    }

    // now for the promotion moves

    if is_pawn && (to & RANK_1 != 0 || to & RANK_8 != 0) {
     
        if let Some(promoted_piece) = promoted_piece 
        {
            bb[side][P] &= !to;
            bb[side][promoted_piece as usize] |= to;
        }
        else
        {
            bb[side][P] &= !to;
            bb[side][Q] |= to;
        }
    }
    bb
}