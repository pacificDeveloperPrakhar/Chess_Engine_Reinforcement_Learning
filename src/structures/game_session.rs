#![allow(non_camel_case_types)]

use crate::structures::annotations::Piece;
use crate::generator::{generate_moves_for_king, is_king_checked, possible_moves};

const FILE_A: u64 = 0x0101_0101_0101_0101;
const FILE_H: u64 = 0x8080_8080_8080_8080;
const RANK_1: u64 = 0x0000_0000_0000_00FF;
const RANK_8: u64 = 0xFF00_0000_0000_0000;

const A1: u64 = 1 << 0;
const E1: u64 = 1 << 4;
const H1: u64 = 1 << 7;
const A8: u64 = 1 << 56;
const E8: u64 = 1 << 60;
const H8: u64 = 1 << 63;

const P: usize = Piece::P as usize;
const N: usize = Piece::N as usize;
const B: usize = Piece::B as usize;
const R: usize = Piece::R as usize;
const K: usize = Piece::K as usize;
const Q: usize = Piece::Q as usize;
const ALL: usize = Piece::A as usize;

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

pub struct GamePlay {
    pub moves: u64,
    pub bitboards: Boards,
    pub side_to_move: u8,
    pub status: GameStatus,
}

fn in_check(bb: Boards, side: usize) -> bool {
    if bb[side][K] == 0 {
        return false;
    }
    let (attackers, _) = is_king_checked(bb, if side == 0 { 'w' } else { 'b' });
    attackers != 0
}

fn make_move(mut bb: Boards, side: usize, from: u64, to: u64, ep_target: u64, promo: usize) -> Boards {
    let enemy = 1 - side;
    let is_pawn = bb[side][P] & from != 0;
    let is_king = bb[side][K] & from != 0;
    let ep_capture = is_pawn && ep_target != 0 && to == ep_target;

    let victim = if ep_capture {
        if side == 0 { to >> 8 } else { to << 8 }
    } else {
        to
    };
    for i in 0..7 {
        bb[enemy][i] &= !victim;
    }

    for i in 0..6 {
        if bb[side][i] & from != 0 {
            bb[side][i] = (bb[side][i] & !from) | to;
        }
    }
    bb[side][ALL] = (bb[side][ALL] & !from) | to;

    if is_king {
        let (rook_from, rook_to) = if to == from << 2 {
            (from << 3, from << 1)
        } else if to == from >> 2 {
            (from >> 4, from >> 1)
        } else {
            (0, 0)
        };
        if rook_from != 0 {
            bb[side][R] = (bb[side][R] & !rook_from) | rook_to;
            bb[side][ALL] = (bb[side][ALL] & !rook_from) | rook_to;
        }
    }

    if is_pawn && to & (RANK_1 | RANK_8) != 0 {
        let piece = if promo == N || promo == B || promo == R || promo == Q { promo } else { Q };
        bb[side][P] &= !to;
        bb[side][piece] |= to;
    }
    bb
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

    fn snapshot(&self) -> GamePlay {
        GamePlay {
            moves: self.moves_available,
            bitboards: self.bitboards,
            side_to_move: self.side_to_move,
            status: self.status,
        }
    }

    fn deselect(&mut self) {
        self.mode = mode::select;
        self.moves_available = 0;
        self.selected = 0;
        self.piece_selected = None;
    }

    fn piece_at(&self, side: usize, square: u64) -> Option<Piece> {
        for p in [Piece::P, Piece::N, Piece::B, Piece::R, Piece::Q, Piece::K] {
            if self.bitboards[side][p as usize] & square != 0 {
                return Some(p);
            }
        }
        None
    }

    fn has_right(&self, c: char) -> bool {
        self.castling.contains(c)
    }

    fn remove_right(&mut self, c: char) {
        self.castling.retain(|x| x != c);
        if self.castling.is_empty() {
            self.castling.push('-');
        }
    }

    fn update_castling_rights(&mut self, from: u64, to: u64) {
        let t = from | to;
        if t & E1 != 0 { self.remove_right('K'); self.remove_right('Q'); }
        if t & H1 != 0 { self.remove_right('K'); }
        if t & A1 != 0 { self.remove_right('Q'); }
        if t & E8 != 0 { self.remove_right('k'); self.remove_right('q'); }
        if t & H8 != 0 { self.remove_right('k'); }
        if t & A8 != 0 { self.remove_right('q'); }
    }

    fn castling_moves(&self, king_sq: u64) -> u64 {
        let side = self.side_to_move as usize;
        let (home, ks_char, qs_char) = if side == 0 { (E1, 'K', 'Q') } else { (E8, 'k', 'q') };
        if king_sq != home || in_check(self.bitboards, side) {
            return 0;
        }
        let bb = self.bitboards;
        let occupied = bb[0][ALL] | bb[1][ALL];
        let safe = |sq: u64| !in_check(make_move(bb, side, home, sq, 0, Q), side);
        let mut res = 0u64;

        if self.has_right(ks_char)
            && bb[side][R] & (home << 3) != 0
            && occupied & ((home << 1) | (home << 2)) == 0
            && safe(home << 1)
            && safe(home << 2)
        {
            res |= home << 2;
        }
        if self.has_right(qs_char)
            && bb[side][R] & (home >> 4) != 0
            && occupied & ((home >> 1) | (home >> 2) | (home >> 3)) == 0
            && safe(home >> 1)
            && safe(home >> 2)
        {
            res |= home >> 2;
        }
        res
    }

    fn ep_moves(&self, select: u64) -> u64 {
        if select.count_ones() != 1 || self.ep_target == 0 {
            return 0;
        }
        let side = self.side_to_move as usize;
        let pawn = self.bitboards[side][P] & select;
        if pawn == 0 {
            return 0;
        }
        let captures = if side == 0 {
            ((pawn & !FILE_A) << 7) | ((pawn & !FILE_H) << 9)
        } else {
            ((pawn & !FILE_H) >> 7) | ((pawn & !FILE_A) >> 9)
        };
        self.ep_target & captures
    }

    pub fn legal_moves(&self, sq: u64) -> u64 {
        let side = self.side_to_move as usize;
        if sq.count_ones() != 1 || self.bitboards[side][ALL] & sq == 0 {
            return 0;
        }
        if self.bitboards[side][K] & sq != 0 {
            return generate_moves_for_king(self.bitboards, sq) | self.castling_moves(sq);
        }

        let own = self.bitboards[side][ALL];
        let pseudo = (possible_moves(self.bitboards, sq) & !own) | self.ep_moves(sq);

        let mut legal = 0u64;
        let mut rest = pseudo;
        while rest != 0 {
            let to = rest & rest.wrapping_neg();
            let after = make_move(self.bitboards, side, sq, to, self.ep_target, Q);
            if !in_check(after, side) {
                legal |= to;
            }
            rest &= rest - 1;
        }
        legal
    }

    fn has_any_legal_move(&self) -> bool {
        let side = self.side_to_move as usize;
        let mut pieces = self.bitboards[side][ALL];
        while pieces != 0 {
            let sq = pieces & pieces.wrapping_neg();
            if self.legal_moves(sq) != 0 {
                return true;
            }
            pieces &= pieces - 1;
        }
        false
    }

    fn compute_status(&self) -> GameStatus {
        let side = self.side_to_move as usize;
        let checked = in_check(self.bitboards, side);
        if !self.has_any_legal_move() {
            return if checked { GameStatus::Checkmate } else { GameStatus::Stalemate };
        }
        if self.halfmove >= 100 {
            return GameStatus::DrawFiftyMove;
        }
        if checked { GameStatus::Check } else { GameStatus::Ongoing }
    }

    pub fn play(&mut self, select: u64) -> GamePlay {
        self.play_with_promotion(select, Piece::Q)
    }
    pub fn play_with_promotion(&mut self, select: u64, promo: Piece) -> GamePlay {
        if select.count_ones() != 1 {
            return self.snapshot();
        }
        if matches!(
            self.status,
            GameStatus::Checkmate | GameStatus::Stalemate | GameStatus::DrawFiftyMove
        ) {
            return self.snapshot();
        }
        let side = self.side_to_move as usize;

        match self.mode {
            mode::select => {
                if self.bitboards[side][ALL] & select == 0 {
                    return self.snapshot();
                }
                let moves = self.legal_moves(select);
                if moves != 0 {
                    self.selected = select;
                    self.piece_selected = self.piece_at(side, select);
                    self.moves_available = moves;
                    self.mode = mode::moving;
                }
                self.snapshot()
            }

            mode::moving => {
                if self.moves_available & select != 0 {
                    let from = self.selected;
                    let is_pawn = matches!(self.piece_selected, Some(Piece::P));
                    let is_capture = self.bitboards[1 - side][ALL] & select != 0
                        || (is_pawn && select == self.ep_target);

                    self.bitboards = make_move(
                        self.bitboards, side, from, select, self.ep_target, promo as usize,
                    );
                    self.update_castling_rights(from, select);

                    self.ep_target = if is_pawn && side == 0 && select == from << 16 {
                        from << 8
                    } else if is_pawn && side == 1 && select == from >> 16 {
                        from >> 8
                    } else {
                        0
                    };

                    self.halfmove = if is_pawn || is_capture { 0 } else { self.halfmove + 1 };
                    if side == 1 {
                        self.fullmove += 1;
                    }
                    self.side_to_move = 1 - self.side_to_move;
                    self.deselect();
                    self.status = self.compute_status();
                    return self.snapshot();
                }

                let other_own = self.bitboards[side][ALL] & select != 0 && select != self.selected;
                self.deselect();
                if other_own {
                    return self.play_with_promotion(select, promo);
                }
                self.snapshot()
            }
        }
    }
}