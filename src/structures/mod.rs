pub mod state;
pub mod annotations;
use crate::structures::annotations::{Piece, PieceColor};
pub fn get_pgn_notation_from_bitboards(bitboards: [[u64; 7]; 2]) -> String {
    // Step 1: build a flat 64-square board, no slashes yet.
    let mut board = ['\0'; 64];

    for i in 0..2 {
        for j in 0..6 {
            let mut piece = bitboards[i][j];
            let piece_char = match j {
                0 => 'P',
                1 => 'N',
                2 => 'B',
                3 => 'R',
                4 => 'K',
                5 => 'Q',
                _ => '0',
            };
            while piece != 0 {
                let square = piece.trailing_zeros();
                let rank = square / 8;
                let file = square % 8;
                let c = if i == 0 {
                    piece_char.to_ascii_uppercase()
                } else {
                    piece_char.to_ascii_lowercase()
                };
                // flip rank so index 0 = a8 (top-left), matching FEN's top-to-bottom order
                board[((7 - rank) * 8 + file) as usize] = c;
                piece &= piece - 1;
            }
        }
    }

    // Step 2: serialize to FEN-style string, inserting '/' every 8 squares
    // and run-length-encoding empty squares.
    let mut result = String::with_capacity(71);
    let mut count = 0;

    for (idx, &c) in board.iter().enumerate() {
        if idx != 0 && idx % 8 == 0 {
            if count > 0 {
                result.push_str(&count.to_string());
                count = 0;
            }
            result.push('/');
        }
        if c != '\0' {
            if count > 0 {
                result.push_str(&count.to_string());
                count = 0;
            }
            result.push(c);
        } else {
            count += 1;
        }
    }
    if count > 0 {
        result.push_str(&count.to_string());
    }

    result
}

pub fn pgn_notation_to_bitboards(pgn: &str) -> [[u64; 7]; 2] {
    let mut bitboards: [[u64; 7]; 2] = [[0; 7]; 2];
    let mut fen_idx: u32 = 0; // 0..63, in FEN's top-left-to-bottom-right order

    for c in pgn.chars() {
        if c == '/' {
            continue;
        }
        if let Some(empty_squares) = c.to_digit(10) {
            fen_idx += empty_squares;
            continue;
        }

        let piece_color = if c.is_uppercase() { PieceColor::W } else { PieceColor::B };
        let piece_type = match c.to_ascii_lowercase() {
            'p' => Piece::P,
            'n' => Piece::N,
            'b' => Piece::B,
            'r' => Piece::R,
            'q' => Piece::Q,
            'k' => Piece::K,
            _ => { fen_idx += 1; continue; }
        };

        // convert FEN order (rank 7 first) back to bottom-up square index
        let rank_from_top = fen_idx / 8;
        let file = fen_idx % 8;
        let actual_rank = 7 - rank_from_top;
        let square = actual_rank * 8 + file;

        bitboards[piece_color as usize][piece_type as usize] |= 1u64 << square;
        bitboards[piece_color as usize][Piece::A as usize] |= 1u64 << square;

        fen_idx += 1;
    }

    bitboards
}

pub fn get_legal_possible_moves_notation_for_rendering(
    bitboards: [[u64; 7]; 2],
    all_moves: u64,
    selected_position: u64,
) -> String {
    // Determine which color is moving, based on which side's piece sits on selected_position.
    let moving_color = if bitboards[PieceColor::W as usize][Piece::A as usize] & selected_position != 0 {
        PieceColor::W
    } else {
        PieceColor::B
    };
    let enemy_color = match moving_color {
        PieceColor::W => PieceColor::B,
        PieceColor::B => PieceColor::W,
    };

    // Step 1: place the real board, exactly as in get_pgn_notation_from_bitboards.
    let mut board = ['\0'; 64];
    for i in 0..2 {
        for j in 0..6 {
            let mut piece = bitboards[i][j];
            let piece_char = match j {
                0 => 'P',
                1 => 'N',
                2 => 'B',
                3 => 'R',
                4 => 'K',
                5 => 'Q',
                _ => '0',
            };
            while piece != 0 {
                let square = piece.trailing_zeros();
                let rank = square / 8;
                let file = square % 8;
                let c = if i == 0 {
                    piece_char.to_ascii_uppercase()
                } else {
                    piece_char.to_ascii_lowercase()
                };
                board[((7 - rank) * 8 + file) as usize] = c;
                piece &= piece - 1;
            }
        }
    }

    // Step 2: overlay every move square from `all_moves`.
    // '-' if it lands on an enemy piece (capture), '+' if it's an empty destination.
    let mut moves = all_moves;
    while moves != 0 {
        let square = moves.trailing_zeros();
        let rank = square / 8;
        let file = square % 8;
        let idx = ((7 - rank) * 8 + file) as usize;

        let is_capture = bitboards[enemy_color as usize][Piece::A as usize] & (1u64 << square) != 0;
        board[idx] = if is_capture { '-' } else { '+' };

        moves &= moves - 1;
    }

    // Step 3: serialize with slashes + run-length-encoded empty squares (same as before).
    let mut result = String::with_capacity(80);
    let mut count = 0;

    for (idx, &c) in board.iter().enumerate() {
        if idx != 0 && idx % 8 == 0 {
            if count > 0 {
                result.push_str(&count.to_string());
                count = 0;
            }
            result.push('/');
        }
        if c != '\0' {
            if count > 0 {
                result.push_str(&count.to_string());
                count = 0;
            }
            result.push(c);
        } else {
            count += 1;
        }
    }
    if count > 0 {
        result.push_str(&count.to_string());
    }

    result
}