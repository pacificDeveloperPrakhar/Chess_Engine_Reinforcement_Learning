use crate::structures::annotations::{Piece, PieceColor};

pub fn pawn_moves(bitboards: [[u64; 7]; 2], mut position:u64) -> u64 {
    let white_pieces = bitboards[PieceColor::W as usize][Piece::A as usize];
    let black_pieces = bitboards[PieceColor::B as usize][Piece::A as usize];

    let white_start: u64 = 65280;
    let black_start: u64 = 71776119061217280;

    let mut pawn_moves: u64 = 0;
    let all_pieces = white_pieces | black_pieces;

    let square = position.trailing_zeros() as u64;
    let rank = square / 8;
    let file = square % 8;

    if (white_pieces & position & white_start) != 0 {
        let mut step = 1;
        while step <= 2 {
            let target = position << (step * 8);
            if target & all_pieces != 0 {
                break;
            }
            pawn_moves |= target;
            step += 1;
        }
    } else if (black_pieces & position & black_start) != 0 {
        let mut step = 1;
        while step <= 2 {
            let target = position >> (step * 8);
            if target & all_pieces != 0 {
                break;
            }
            pawn_moves |= target;
            step += 1;
        }
    }

    {
        let mut pawn_attack_moves = 0;

        // white
        if position & white_pieces != 0 {
            if rank + 1 < 8 && file >= 1 {
                pawn_attack_moves |= ((position << 8) >> 1) & black_pieces;
            }
            if rank + 1 < 8 && file + 1 < 8 {
                pawn_attack_moves |= ((position << 8) << 1) & black_pieces;
            }
            if rank + 1 < 8 && ((position << 8) & all_pieces) == 0 {
                pawn_moves |= position << 8;
            }
        }

        // black
        if position & black_pieces != 0 {
            if rank >= 1 && file >= 1 {
                pawn_attack_moves |= ((position >> 8) >> 1) & white_pieces;
            }
            if rank >= 1 && file + 1 < 8 {
                pawn_attack_moves |= ((position >> 8) << 1) & white_pieces;
            }
            if rank >= 1 && ((position >> 8) & all_pieces) == 0 {
                pawn_moves |= position >> 8;
            }
        }

        pawn_moves |= pawn_attack_moves;
    }

    pawn_moves
}