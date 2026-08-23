use crate::structures::annotations::{Piece, PieceColor};
pub fn one_square_move(bitboards:[[u64;7];2],rank:usize,file:usize)->u64
{
    let position =1 <<((rank*8) as u64 + file as u64);
    let mut  result:u64=0;
    if (rank+1)<8
    {
        result|=(position <<1*8 );
		// for the right up
        if file >=1
        {
            result|=(position<<(1*8)>>1);
        }
		// for the left up
        if file+1 <8
        {
            result|=(position<<(1*8)<<1);
        }
    }
    if rank>=1
    {   
	// for the bottom direction
        result|=(position >> 1*8 );
		// for the right bottom
        if file >=1
        {
            result|=(position>>(1*8)>>1);
        }
		// for the left bottom
        if file+1 <8
        {
            result|=(position >>(1*8)<<1);
            result|=(position<<1);
        }
    }
	// for left
    if file>=1
    {
        result|=(position>>1);
    }
	// for the left
    if file+1 <8
    {
        result|=(position<<1);
    }
    
    return result;
}