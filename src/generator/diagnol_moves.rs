use crate::structures::annotations::{Piece, PieceColor};

pub fn diagnol_moves(bitboards: [[u64; 7]; 2], rank: usize, file: usize) -> u64 {
    let mut all_piece = bitboards[PieceColor::W as usize][Piece::A as usize]
        | bitboards[PieceColor::B as usize][Piece::A as usize];
    let position: u64 = 1 << ((rank * 8) as u64 + file as u64);
    let mut diagnol_attackings: u64 = 0;
    all_piece ^= position;

    // upper right
    let mut step_row = 1;
    let mut step_col = 1;
    while (step_row + rank < 8) && (step_col <= file) {
        let pos = (position << (step_row * 8)) >> step_col;
        let pos_with = pos & all_piece;
        if pos_with != 0 {
            diagnol_attackings |= pos_with;
            break;
        }
        diagnol_attackings |= pos;
        step_col += 1;
        step_row += 1;
    }

    // upper left
    step_col = 1;
    step_row = 1;
    while (step_col + file < 8) && (step_row + rank < 8) {
        let pos = (position << (step_row * 8)) << step_col;
        let pos_with = pos & all_piece;
        if pos_with != 0 {
            diagnol_attackings |= pos_with;
            break;
        }
        diagnol_attackings |= pos;
        step_col += 1;
        step_row += 1;
    }

    // bottom right
    step_col = 1;
    step_row = 1;
    while (step_row <= rank) && (step_col <= file) {
        let pos = (position >> (step_row * 8)) >> step_col;
        let pos_with = pos & all_piece;
        if pos_with != 0 {
            diagnol_attackings |= pos_with;
            break;
        }
        diagnol_attackings |= pos;
        step_col += 1;
        step_row += 1;
    }

    // bottom left
    step_col = 1;
    step_row = 1;
    while (file + step_col < 8) && (rank >= step_row) {
        let pos = (position >> (step_row * 8)) << step_col;
        let pos_with = pos & all_piece;
        if pos_with != 0 {
            diagnol_attackings |= pos_with;
            break;
        }
        diagnol_attackings |= pos;
        step_col += 1;
        step_row += 1;
    }

    diagnol_attackings
}