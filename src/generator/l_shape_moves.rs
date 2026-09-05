use crate::structures::annotations::{Piece, PieceColor};
pub fn l_squares(bitboards:[[u64;7];2],mut position:u64)->u64
{
    let mut l_shape_attackings: u64 = 0;
    
    let square = position.trailing_zeros() as u64;
    let rank = square / 8;
    let file = square % 8;

    // calculating for the knight attacking position
    if (rank+2)<8 && file >=1 
    {
        l_shape_attackings|=(position << 2*8 >> 1); 
    }
    if rank>=2 && (rank+1)<8
    {

        l_shape_attackings|=(position >> 2*8 << 1); 
    }
    if rank+2<8 && file+1<8
    {

        l_shape_attackings|=(position << 2*8 << 1); 
    }
    if rank>=1&& file>=2
    {

        l_shape_attackings|=(position >> 1*8 >> 2); 
    }
    if rank>=1 && (file+2) <8
    {

        l_shape_attackings|=(position >> 1*8 << 2); 
    }
    if (rank +1) <8 && (file+2) <8
    {

        l_shape_attackings|=(position << 1*8 << 2);
    }
    if (rank+1)<8 && file >=2
    {

        l_shape_attackings|=(position << 1*8 >> 2);
    }
    if rank>=2 && file>=1
    {

        l_shape_attackings|=(position >> 2*8 >> 1) ;
    }


    return l_shape_attackings|position;

}