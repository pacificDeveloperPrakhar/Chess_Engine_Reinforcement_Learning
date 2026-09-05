pub mod l_shape_moves;
pub mod diagnol_moves;
pub mod horizontal_vertical_moves;
pub mod pawn_moves;
pub mod one_square_move;
use l_shape_moves::l_squares;
use  diagnol_moves::diagnol_moves;
use horizontal_vertical_moves::horizontal_vertical_moves;
use pawn_moves::pawn_moves;
use one_square_move::one_square_move;

use crate::structures::annotations::{AttackType,PieceColor,Piece};

pub fn rank_file_generator(rank:usize,file:usize)->u64
{
    let position:u64=((1<<(rank*8)) >>file);
    return position;
}
// moves generator for all the pieces
pub fn possible_moves(bitboards: [[u64; 7]; 2], position_selected:u64) -> u64 {
    if(bitboards[PieceColor::W as usize][Piece::A as usize] & (position_selected) == 0)
    {
        return 0;
    }
    let mut role=0;
    if(bitboards[PieceColor::B as usize][Piece::A as usize] & (position_selected) == 0)
    {
        role=1;
    }

    let ans = if bitboards[role][Piece::P as usize] & position_selected != 0 {
        pawn_moves(bitboards,position_selected)
    } else if bitboards[role][Piece::N as usize] & position_selected != 0 {
        l_squares(bitboards,position_selected)
    } else if bitboards[role][Piece::B as usize] & position_selected != 0 {
        diagnol_moves(bitboards,position_selected)
    } else if bitboards[role][Piece::R as usize] & position_selected != 0 {
        horizontal_vertical_moves(bitboards,position_selected)
    } else if bitboards[role][Piece::Q as usize] & position_selected != 0 {
        diagnol_moves(bitboards,position_selected)|horizontal_vertical_moves(bitboards,position_selected)
    } else if bitboards[role][Piece::K as usize] & position_selected != 0 {
        one_square_move(bitboards,position_selected)
    } else {
        0
    };

    return ans;
}
//check if the king is checked or not for specific side
pub fn is_king_checked(bitboards: [[u64; 7]; 2], side: char) -> (u64,AttackType) {
    let position_selected =
    bitboards[PieceColor::from(side) as usize][Piece::K as usize];

    let mut diagnol_attackings: u64 = 0;
    let mut l_shape_attackings: u64 = 0;
    let mut horizontal_vertical_attackings: u64 = 0;
    let mut pawn_attackings: u64 = 0;
    
    // calculate the enemy side bitboard
    let enemy_side: usize = 1 - (PieceColor::from(side) as usize);
    //calculate all the enemies pieces
    let all_enemies = bitboards[enemy_side][Piece::A as usize];

    // now proceed with the square calculation
    l_shape_attackings=l_squares(bitboards,position_selected)& all_enemies;
    // now will be calculating for the horizontal and vertical attacking positions
    horizontal_vertical_attackings = horizontal_vertical_moves(bitboards,position_selected) &all_enemies;
    // now for all the diagonal attacking sides
    diagnol_attackings=diagnol_moves(bitboards,position_selected)&all_enemies;
    // pawn attackings
    pawn_attackings=pawn_moves(bitboards,position_selected)&all_enemies;
    //king attacking
    let king_attackings=one_square_move(bitboards,position_selected)&all_enemies& bitboards[enemy_side as usize][Piece::K as usize];
    let mut result=0;
    let mut attack_type=AttackType::pawn;
    if pawn_attackings &bitboards[enemy_side][Piece::P as usize]!=0
    {
        result|=(pawn_attackings &bitboards[enemy_side][Piece::P as usize]);
        attack_type=AttackType::pawn ;
    }
    if horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]) !=0
    {
        result|=(horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]));
        attack_type=AttackType::horizontal_vertical ;
    }

    if l_shape_attackings&(bitboards[enemy_side][Piece::N as usize])!=0
    {
        result|=(l_shape_attackings&(bitboards[enemy_side][Piece::N as usize]));
        attack_type=AttackType::l_shape ;
    }
    if diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]) !=0
    {
        result|=(diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]));
        attack_type=AttackType::diagnol ;
    }
    if king_attackings!=0
    {
        result|=king_attackings;
        attack_type=AttackType::square ;
    }
    return (result,attack_type);
}

pub fn generating_moves_with_king_safety(mut bitboards: [[u64; 7]; 2], selected_position:u64) -> u64 {
    let color= if bitboards[PieceColor::W as usize][Piece::A as usize] & selected_position != 0 {
        PieceColor::W
    } else {
        PieceColor::B
    };
    let possible_moves_in_free_condition=possible_moves(bitboards,selected_position);
    let mut valid_moves=0;
    // remove the selected position from the bitboard to check for king safety
    bitboards[PieceColor::from(color) as usize][Piece::A as usize] ^= selected_position;
    // check the possible moves and see if the king is checked or not
    let (checked_positions, attack_type) = is_king_checked(
    bitboards,
    match color {
        PieceColor::W => 'w',
        PieceColor::B => 'B',
    },
    );
    // also take the king position on the bitboard
    let king_position=bitboards[PieceColor::from(color) as usize][Piece::K as usize];
    if checked_positions == 0 {
        return possible_moves_in_free_condition;
    }
    let enemy_color=1-(color as usize);
    let moves_checked_positions =match attack_type
    {
        AttackType::diagnol =>
        {
            diagnol_moves(bitboards,checked_positions)&diagnol_moves(bitboards,king_position)
        },
        AttackType::horizontal_vertical =>
        {
            horizontal_vertical_moves(bitboards,checked_positions)&horizontal_vertical_moves(bitboards,king_position)
        },
        AttackType::l_shape =>
        {   
            l_squares(bitboards,checked_positions)&l_squares(bitboards,king_position)
        },
        AttackType::pawn =>
        {   
            pawn_moves(bitboards,checked_positions)&pawn_moves(bitboards,king_position)
        },
        _ =>
        {
            one_square_move(bitboards,checked_positions)&one_square_move(bitboards,king_position)
        }
    };
    return moves_checked_positions&possible_moves_in_free_condition;
}