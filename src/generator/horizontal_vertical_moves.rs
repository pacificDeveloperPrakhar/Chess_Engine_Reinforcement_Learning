use crate::structures::annotations::{Piece, PieceColor};

pub fn horizontal_vertical_moves(bitboards:[[u64;7];2],mut position:u64)->u64
{
    let mut horizontal_vertical_attackings: u64 = 0;
    {
        let square = position.trailing_zeros() as u64;
        let rank = square / 8;
        let file = square % 8;

        let white_all = bitboards[PieceColor::W as usize][Piece::A as usize];
        let black_all = bitboards[PieceColor::B as usize][Piece::A as usize];
        let all_side_raw = white_all | black_all;
        let own_all = if white_all & position != 0 { white_all } else { black_all };
        let all_side = all_side_raw ^ position;

        let mut step = 1;

        // in right direction
        while step<=file {
            let pos_with= position >> step;
            if pos_with & own_all != 0 {
                break; // blocked by own piece: stop, don't add
            }
            horizontal_vertical_attackings |= pos_with;
            if pos_with & all_side != 0 {
                break; // captured enemy piece: add, then stop
            }
            step += 1;
        }

        step = 1;
        // now in the left direction
        while file+step<8 {
            let pos_with = position << step;
            if pos_with & own_all != 0 {
                break;
            }
            horizontal_vertical_attackings |= pos_with;
            if pos_with & all_side != 0 {
                break;
            }
            step += 1;
        }

        // now in the vertical upward direction
        step = 1;
        while step + rank < 8 {
            let pos_with = position << (step * 8);
            if pos_with & own_all != 0 {
                break;
            }
            horizontal_vertical_attackings |= pos_with;
            if pos_with & all_side != 0 {
                break;
            }
            step += 1;
        }

        // now for the vertical downward direction
        step = 1;
        while rank>=step {
            let pos_with = position >> (step * 8);
            if pos_with & own_all != 0 {
                break;
            }
            horizontal_vertical_attackings |= pos_with;
            if pos_with & all_side != 0 {
                break;
            }
            step += 1;
        }
    }
    return horizontal_vertical_attackings;
}

pub fn horizontal_vertical_attacks(bitboards:[[u64;7];2], position:u64) -> u64 {
    let mut result: u64 = 0;
    let square = position.trailing_zeros() as u64;
    let rank = square / 8;
    let file = square % 8;
    let all_side = (bitboards[PieceColor::W as usize][Piece::A as usize]
        | bitboards[PieceColor::B as usize][Piece::A as usize]) ^ position;

    let mut step = 1;
    while step<=file {
        let pos = position >> step;
        result |= pos;
        if pos & all_side != 0 { break; }
        step += 1;
    }
    step = 1;
    while file+step<8 {
        let pos = position << step;
        result |= pos;
        if pos & all_side != 0 { break; }
        step += 1;
    }
    step = 1;
    while step + rank < 8 {
        let pos = position << (step * 8);
        result |= pos;
        if pos & all_side != 0 { break; }
        step += 1;
    }
    step = 1;
    while rank>=step {
        let pos = position >> (step * 8);
        result |= pos;
        if pos & all_side != 0 { break; }
        step += 1;
    }
    result
}