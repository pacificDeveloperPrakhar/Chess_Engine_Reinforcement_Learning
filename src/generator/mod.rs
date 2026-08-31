pub mod l_shape_moves;
pub mod diagnol_moves;
pub mod horizontal_vertical_moves;
pub mod pawn_moves;


pub fn rank_file_generator(rank:usize,file:usize)->u64
{
    let position:u64=((1<<(rank*8)) >>file);
    return position;
}
// moves generator for all the pieces
pub fn possible_moves(bitboards: [[u64; 7]; 2], piece_position:u64) -> u64 {
    if(bitboards[PieceColor::W as usize][Piece::A as usize] & (piece_position) == 0)
    {
        return 0;
    }
    let mut role=0;
    if(bitboards[PieceColor::B as usize][Piece::A as usize] & (piece_position) == 0)
    {
        role=1;
    }

    let ans=match (piece_position)
    {
         bitboards[role][Piece::P as usize]& piece_position => pawn_moves(bitboards,rank,file),
         bitboards[role][Piece::N as usize]& piece_position => l_squares(bitboards,rank,file),
         bitboards[role][Piece::B as usize]& piece_position => diagnol_moves(bitboards,rank,file),
         bitboards[role][Piece::R as usize]& piece_position => horizontal_vertical_moves(bitboards,rank,file),
         bitboards[role][Piece::Q as usize]& piece_position => diagnol_moves(bitboards,rank,file)|horizontal_vertical_moves(bitboards,rank,file),
         bitboards[role][Piece::K as usize]& piece_position => one_square_move(bitboards,rank,file),
         _ => 0,
    }

    return ans;
}
//check if the king is checked or not for specific side
pub fn is_king_checked(bitboards: [[u64; 7]; 2], side: char) -> u64 {
    let rank;
    let file;
    let king_position =
    bitboards[PieceColor::from(side) as usize][Piece::K as usize];
    (rank, file) = resolve_move(king_position);

    let mut diagnol_attackings: u64 = 0;
    let mut l_shape_attackings: u64 = 0;
    let mut horizontal_vertical_attackings: u64 = 0;
    let mut pawn_attackings: u64 = 0;
    
    // calculate the enemy side bitboard
    let enemy_side: usize = 1 - (PieceColor::from(side) as usize);
    //calculate all the enemies pieces
    let all_enemies = bitboards[enemy_side][Piece::A as usize];

    // now proceed with the square calculation
    l_shape_attackings=l_squares(bitboards,rank,file)& all_enemies;
    // now will be calculating for the horizontal and vertical attacking positions
    horizontal_vertical_attackings = horizontal_vertical_moves(bitboards,rank,file) &all_enemies;
    // now for all the diagonal attacking sides
    diagnol_attackings=diagnol_moves(bitboards,rank,file)&all_enemies;
    // pawn attackings
    pawn_attackings=pawn_moves(bitboards,rank,file)&all_enemies;
    let board=display_bitboard(pawn_attackings);

    //king attacking
    let king_attackings=one_square_move(bitboards,rank,file)&all_enemies& bitboards[enemy_side as usize][Piece::K as usize];
    let mut result=0;
    if pawn_attackings &bitboards[enemy_side][Piece::P as usize]!=0
    {
        result|=(pawn_attackings &bitboards[enemy_side][Piece::P as usize]);
    }
    if horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]) !=0
    {
        result|=(horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]))
    }

    if l_shape_attackings&(bitboards[enemy_side][Piece::N as usize])!=0
    {
        result|=(l_shape_attackings&(bitboards[enemy_side][Piece::N as usize]));
    }
    if diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]) !=0
    {
        result|=(diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]));
    }
    return result;
}