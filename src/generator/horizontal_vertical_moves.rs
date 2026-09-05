use crate::structures::annotations::{Piece, PieceColor};
pub fn horizontal_vertical_moves(bitboards:[[u64;7];2],mut position:u64)->u64
{
    let mut horizontal_vertical_attackings: u64 = 0;
    // now will be calculating for the horizontal and vertical attacking positions
    {
        let square = position.trailing_zeros() as u64;
        let rank = square / 8;
        let file = square % 8;

        let mut all_side = bitboards[PieceColor::W as usize][Piece::A as usize]|bitboards[PieceColor::B as usize][Piece::A as usize];
        all_side=all_side^position;
        let mut step = 1;

        // in right direction
        while step<=file {
            let pos_with= (position >> step) ;
            let pos=pos_with& all_side;
            if pos != 0 {
                horizontal_vertical_attackings |= pos;
                break;
            }
            horizontal_vertical_attackings|=pos_with;
            step += 1;
        }

        step = 1;
        // now in the left direction
        while file+step<8 {

            let pos_with = (position << step) ;
            let pos = pos_with & all_side;
            if pos != 0 {
                horizontal_vertical_attackings |= pos;
                break;
            }
            horizontal_vertical_attackings |= pos_with;
            step += 1;
        }

        // now in the vertical upward direction
        step = 1;
        while step + rank < 8 {
            let pos_with=(position << (step * 8));
            let pos = pos_with & all_side;
            if pos != 0 {
                horizontal_vertical_attackings |= pos;
                break;
            }
            horizontal_vertical_attackings|=pos_with;
            step += 1;
        }

        // now for the vertical downward direction
        step = 1;
        while rank>=step{
            let pos_with = (position >> (step * 8)) ;
            let pos = (position >> (step * 8)) & all_side;
            if pos != 0 {
                horizontal_vertical_attackings |= pos;
                break;
            }
            horizontal_vertical_attackings |= pos_with;
            step += 1;
 
        }
    }
    return horizontal_vertical_attackings;
}