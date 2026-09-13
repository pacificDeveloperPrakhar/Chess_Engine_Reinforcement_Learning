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
pub fn is_king_checked(bitboards: [[u64; 7]; 2], side: char) -> (u64,Vec<AttackType>) {
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
    let mut attack_type=Vec::new();
    if pawn_attackings &bitboards[enemy_side][Piece::P as usize]!=0
    {
        result|=(pawn_attackings &bitboards[enemy_side][Piece::P as usize]);
        attack_type.push(AttackType::pawn);
    }
    if horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]) !=0
    {
        result|=(horizontal_vertical_attackings &(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::R as usize]));
        attack_type.push(AttackType::horizontal_vertical);
    }

    if l_shape_attackings&(bitboards[enemy_side][Piece::N as usize])!=0
    {
        result|=(l_shape_attackings&(bitboards[enemy_side][Piece::N as usize]));
        attack_type.push(AttackType::l_shape);
    }
    if diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]) !=0
    {
        result|=(diagnol_attackings&(bitboards[enemy_side][Piece::Q as usize]|bitboards[enemy_side][Piece::B as usize]));
        attack_type.push(AttackType::diagnol);
    }
    if king_attackings!=0
    {
        result|=king_attackings;
        attack_type.push(AttackType::square);
    }
    return (result,attack_type);
}

pub fn generate_moves_for_king(mut bitboards: [[u64; 7]; 2], selected_position: u64) -> u64 {
    let mut king_position = 0u64;
    let mut role = 0usize;

    if bitboards[PieceColor::W as usize][Piece::K as usize] & selected_position != 0 {
        king_position = bitboards[PieceColor::W as usize][Piece::K as usize];
        role = 0;
    } else if bitboards[PieceColor::B as usize][Piece::K as usize] & selected_position != 0 {
        king_position = bitboards[PieceColor::B as usize][Piece::K as usize];
        role = 1;
    }

    if king_position == 0 {
        return 0;
    }

    let possible_moves_king = one_square_move(bitboards, king_position);
    if possible_moves_king == 0 {
        return 0;
    }

    // Remove the king from the board ONCE, for every attack computation below.
    bitboards[role][Piece::K as usize] &= !king_position;
    bitboards[role][Piece::A as usize] &= !king_position;

    let enemy_color = 1 - role;
    let mut attacked = 0u64;

    // Pawns: raw diagonal attack squares only (not the forward push,
    // and NOT conditional on whether something currently sits there).
    {
        let mut pieces = bitboards[enemy_color][Piece::P as usize];
        while pieces != 0 {
            let piece_position = 1u64 << pieces.trailing_zeros();
            if role == 0 {
                attacked |= (piece_position >> 7); // white pawns attack down-left
                attacked |= (piece_position >> 9); // white pawns attack down-right
            } else {
                attacked |= (piece_position << 7) ; // black pawns attack up-right
                attacked |= (piece_position << 9);  // black pawns attack up-left
            }
            pieces &= pieces - 1;
        }
    }

    // Knights, bishops, rooks, queens, king attacking patterns
    for i in 1..6 {
        let mut pieces = bitboards[enemy_color][i as usize];
        while pieces != 0 {
            let piece_position = 1u64 << pieces.trailing_zeros();
            let piece_attacks = match i {
                x if x == Piece::N as usize => l_squares(bitboards, piece_position),
                x if x == Piece::B as usize => diagnol_moves(bitboards, piece_position),
                x if x == Piece::R as usize => horizontal_vertical_moves(bitboards, piece_position),
                x if x == Piece::Q as usize => {
                    diagnol_moves(bitboards, piece_position)
                        | horizontal_vertical_moves(bitboards, piece_position)
                }
                x if x == Piece::K as usize => one_square_move(bitboards, piece_position),
                _ => 0,
            };
            attacked |= piece_attacks;
            pieces &= pieces - 1;
        }
    }

    possible_moves_king & !attacked
}
// this function will be used for the non king positions 
pub fn generating_moves_with_king_safety(mut bitboards: [[u64; 7]; 2], selected_position:u64) -> u64 {
    let color= if bitboards[PieceColor::W as usize][Piece::A as usize] & selected_position != 0 {
        PieceColor::W
    } else {
        PieceColor::B
    };
    let possible_moves_in_free_condition=possible_moves(bitboards,selected_position);
    let mut valid_moves=0;
    // remove the selected position from the bitboard to check for king safety
    for i in 0..7
    {
        bitboards[PieceColor::from(color) as usize][i as usize] &= !selected_position;
    }
    // check the possible moves and see if the king is checked or not
    let (all_checked_positions, attack_types) = is_king_checked(
    bitboards,
    match color {
        PieceColor::W => 'w',
        PieceColor::B => 'B',
    }
    );

    if attack_types.len() > 1 {
        return 0;
    }
    // also take the king position on the bitboard
    let king_position=bitboards[PieceColor::from(color) as usize][Piece::K as usize];
    if all_checked_positions == 0 {
        return possible_moves_in_free_condition;
    }
    let mut moves_checked_positions=0;
    let mut checked_positions=all_checked_positions;
    //iterate through each checked bit and then proces it
    for i in 0..attack_types.len()
    {
    let attack_type=attack_types[i];
    let enemy_color=1-(color as usize);
    moves_checked_positions |=match attack_type
    {
        AttackType::diagnol =>
        {
            diagnol_moves(bitboards,1<<checked_positions.trailing_zeros())&diagnol_moves(bitboards,king_position)
        },
        AttackType::horizontal_vertical =>
        {
            horizontal_vertical_moves(bitboards,1<<checked_positions.trailing_zeros())&horizontal_vertical_moves(bitboards,king_position)
        },
        AttackType::l_shape =>
        {   
            l_squares(bitboards,1<<checked_positions.trailing_zeros())&l_squares(bitboards,king_position)
        },
        AttackType::pawn =>
        {   
            pawn_moves(bitboards,1<<checked_positions.trailing_zeros())&pawn_moves(bitboards,king_position)
        },
        _ =>
        {
            one_square_move(bitboards,1<<checked_positions.trailing_zeros())&one_square_move(bitboards,king_position)
        }
    };
    checked_positions&= checked_positions-(1<<checked_positions.trailing_zeros());
    }
    if (king_position&selected_position)!=0
    {
        return possible_moves_in_free_condition^(moves_checked_positions&possible_moves_in_free_condition)
    }

    return moves_checked_positions&possible_moves_in_free_condition
    
}