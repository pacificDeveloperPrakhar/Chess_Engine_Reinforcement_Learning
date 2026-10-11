#![allow(non_camel_case_types)]

use std::convert::From;

/// Material values (centipawn-style). Rust enums cannot repeat a discriminant,
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PieceValue {
    P = 100,
    N = 320,
    B = 330,
    R = 500,
    Q = 900,
    K = 20000,
}

impl From<usize> for PieceValue {
    fn from(c: usize) -> Self {
        match c {
            0 => PieceValue::P,
            1 => PieceValue::N,
            2 => PieceValue::B,
            3 => PieceValue::R,
            4 => PieceValue::K,
            5 => PieceValue::Q,
            _ => PieceValue::P,
        }
    }
}

// second index in the bitboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Piece {
    P = 0, // pawns
    N = 1, // knights
    B = 2, // bishops
    R = 3, // rooks
    K = 4, // king
    Q = 5, // queen
    A = 6, // all pieces of one colour
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PieceColor {
    W = 0,
    B = 1,
}

impl From<char> for Piece {
    fn from(c: char) -> Self {
        Piece::from_char_opt(c).unwrap_or(Piece::A)
    }
}

impl Piece {
    pub fn from_char_opt(c: char) -> Option<Piece> {
        match c {
            'Q' | 'q' => Some(Piece::Q),
            'K' | 'k' => Some(Piece::K),
            'B' | 'b' => Some(Piece::B),
            'R' | 'r' => Some(Piece::R),
            'P' | 'p' => Some(Piece::P),
            'N' | 'n' => Some(Piece::N),
            _ => None,
        }
    }


    pub fn as_char(piece: usize, side: usize) -> char {
        let c = match piece {
            0 => 'p',
            1 => 'n',
            2 => 'b',
            3 => 'r',
            4 => 'k',
            5 => 'q',
            _ => 'p',
        };
        if side == 0 {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }

    pub fn value(self) -> PieceValue {
        PieceValue::from(self as usize)
    }
}

impl From<char> for PieceColor {
    fn from(c: char) -> Self {
        PieceColor::from_char_opt(c).unwrap_or(PieceColor::W)
    }
}

impl PieceColor {

    pub fn from_char_opt(c: char) -> Option<PieceColor> {
        match c {
            'W' | 'w' => Some(PieceColor::W),
            'B' | 'b' => Some(PieceColor::B),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    diagnol,
    horizontal_vertical,
    pawn,
    square,
    l_shape,
}