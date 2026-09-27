pub mod generator;
pub mod evaluator;
pub mod structures;
pub mod parser;

use crate::structures::annotations::{PieceColor,Piece};
use generator::l_shape_moves;

fn main() {
    let mut bitboards: [[u64; 7]; 2] = [[0; 7]; 2];

    // ---------- White pieces (rank 1 & 2) ----------
    bitboards[PieceColor::W as usize][Piece::P as usize] = 0x000000000000FF00; // pawns  - rank 2
    bitboards[PieceColor::W as usize][Piece::R as usize] = 0x0000000000000081; // rooks  - a1, h1
    bitboards[PieceColor::W as usize][Piece::N as usize] = 0x0000000000000042; // knights- b1, g1
    bitboards[PieceColor::W as usize][Piece::B as usize] = 0x0000000000000024; // bishops- c1, f1
    bitboards[PieceColor::W as usize][Piece::K as usize] = 0x0000000000000010; // king   - e1
    bitboards[PieceColor::W as usize][Piece::Q as usize] = 0x0000000000000008; // queen  - d1

    // ---------- Black pieces (rank 7 & 8) ----------
    bitboards[PieceColor::B as usize][Piece::P as usize] = 0x00FF000000000000; // pawns  - rank 7
    bitboards[PieceColor::B as usize][Piece::R as usize] = 0x8100000000000000; // rooks  - a8, h8
    bitboards[PieceColor::B as usize][Piece::N as usize] = 0x4200000000000000; // knights- b8, g8
    bitboards[PieceColor::B as usize][Piece::B as usize] = 0x2400000000000000; // bishops- c8, f8
    bitboards[PieceColor::B as usize][Piece::K as usize] = 0x1000000000000000; // king   - e8
    bitboards[PieceColor::B as usize][Piece::Q as usize] = 0x0800000000000000; // queen  - d8

    // ---------- Aggregate occupancy per color ----------
    for color in 0..2 {
        let mut all = 0u64;
        for piece in 0..6 {
            all |= bitboards[color][piece];
        }
        bitboards[color][Piece::A as usize] = all;
    }

    println!("{:?}", bitboards);
	println!("{}", structures::get_pgn_notation_from_bitboards(bitboards));
	let position_selected = 1u64 << 12; // Example: selecting the piece at d2 (rank 2, file 4)
	let possible_moves = generator::possible_moves(bitboards, position_selected);
	println!("Possible moves for the selected piece: {:#018x}", possible_moves);

	let king_checked = generator::is_king_checked(bitboards, 'W');
	println!("Is White King checked? {}", king_checked.0 != 0);
}

