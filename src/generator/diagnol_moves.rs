use crate::structures::annotations::{Piece, PieceColor};

pub fn diagnol_moves(bitboards: [[u64; 7]; 2], mut position: u64) -> u64 {
    let white_all = bitboards[PieceColor::W as usize][Piece::A as usize];
    let black_all = bitboards[PieceColor::B as usize][Piece::A as usize];
    let all_piece_raw = white_all | black_all;

    // which side owns the piece on `position`
    let own_all = if white_all & position != 0 { white_all } else { black_all };

    let mut diagnol_attackings: u64 = 0;
    let all_piece = all_piece_raw ^ position; // remove source square, keep both colors for blocking checks

    let square = position.trailing_zeros() as u64;
    let rank = square / 8;
    let file = square % 8;

    // upper right
    let mut step_row = 1;
    let mut step_col = 1;
    while (step_row + rank < 8) && (step_col <= file) {
        let pos = (position << (step_row * 8)) >> step_col;
        if pos & own_all != 0 {
            break; // blocked by own piece: stop, don't add
        }
        diagnol_attackings |= pos;
        if pos & all_piece != 0 {
            break; // captured an enemy piece: add it, then stop
        }
        step_col += 1;
        step_row += 1;
    }

    // upper left
    step_col = 1;
    step_row = 1;
    while (step_col + file < 8) && (step_row + rank < 8) {
        let pos = (position << (step_row * 8)) << step_col;
        if pos & own_all != 0 {
            break;
        }
        diagnol_attackings |= pos;
        if pos & all_piece != 0 {
            break;
        }
        step_col += 1;
        step_row += 1;
    }

    // bottom right
    step_col = 1;
    step_row = 1;
    while (step_row <= rank) && (step_col <= file) {
        let pos = (position >> (step_row * 8)) >> step_col;
        if pos & own_all != 0 {
            break;
        }
        diagnol_attackings |= pos;
        if pos & all_piece != 0 {
            break;
        }
        step_col += 1;
        step_row += 1;
    }

    // bottom left
    step_col = 1;
    step_row = 1;
    while (file + step_col < 8) && (rank >= step_row) {
        let pos = (position >> (step_row * 8)) << step_col;
        if pos & own_all != 0 {
            break;
        }
        diagnol_attackings |= pos;
        if pos & all_piece != 0 {
            break;
        }
        step_col += 1;
        step_row += 1;
    }

    diagnol_attackings
}
pub fn diagnol_attacks(bitboards: [[u64; 7]; 2], position: u64) -> u64 {
    let all_piece = (bitboards[PieceColor::W as usize][Piece::A as usize]
        | bitboards[PieceColor::B as usize][Piece::A as usize]) ^ position;

    let mut result: u64 = 0;
    let square = position.trailing_zeros() as u64;
    let rank = square / 8;
    let file = square % 8;

    let mut step_row = 1;
    let mut step_col = 1;
    while (step_row + rank < 8) && (step_col <= file) {
        let pos = (position << (step_row * 8)) >> step_col;
        result |= pos;
        if pos & all_piece != 0 { break; }
        step_col += 1; step_row += 1;
    }
    step_col = 1; step_row = 1;
    while (step_col + file < 8) && (step_row + rank < 8) {
        let pos = (position << (step_row * 8)) << step_col;
        result |= pos;
        if pos & all_piece != 0 { break; }
        step_col += 1; step_row += 1;
    }
    step_col = 1; step_row = 1;
    while (step_row <= rank) && (step_col <= file) {
        let pos = (position >> (step_row * 8)) >> step_col;
        result |= pos;
        if pos & all_piece != 0 { break; }
        step_col += 1; step_row += 1;
    }
    step_col = 1; step_row = 1;
    while (file + step_col < 8) && (rank >= step_row) {
        let pos = (position >> (step_row * 8)) << step_col;
        result |= pos;
        if pos & all_piece != 0 { break; }
        step_col += 1; step_row += 1;
    }
    result
}