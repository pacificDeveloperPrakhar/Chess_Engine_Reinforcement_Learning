use chess_engine::structures::annotations::{Piece, PieceColor};
use chess_engine::structures::{get_pgn_notation_from_bitboards, pgn_notation_to_bitboards};

// Helper: build an empty board
fn empty_bitboards() -> [[u64; 7]; 2] {
    [[0; 7]; 2]
}

// ---------- ENCODE TESTS (bitboards -> FEN string) ----------

#[test]
fn encode_empty_board() {
    let bitboards = empty_bitboards();
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, "8/8/8/8/8/8/8/8");
}

#[test]
fn encode_starting_position() {
    let mut bitboards = empty_bitboards();

    bitboards[PieceColor::W as usize][Piece::R as usize] = (1u64 << 0) | (1u64 << 7);
    bitboards[PieceColor::W as usize][Piece::N as usize] = (1u64 << 1) | (1u64 << 6);
    bitboards[PieceColor::W as usize][Piece::B as usize] = (1u64 << 2) | (1u64 << 5);
    bitboards[PieceColor::W as usize][Piece::Q as usize] = 1u64 << 3;
    bitboards[PieceColor::W as usize][Piece::K as usize] = 1u64 << 4;
    bitboards[PieceColor::W as usize][Piece::P as usize] = 0xFFu64 << 8;

    bitboards[PieceColor::B as usize][Piece::R as usize] = (1u64 << 56) | (1u64 << 63);
    bitboards[PieceColor::B as usize][Piece::N as usize] = (1u64 << 57) | (1u64 << 62);
    bitboards[PieceColor::B as usize][Piece::B as usize] = (1u64 << 58) | (1u64 << 61);
    bitboards[PieceColor::B as usize][Piece::Q as usize] = 1u64 << 59;
    bitboards[PieceColor::B as usize][Piece::K as usize] = 1u64 << 60;
    bitboards[PieceColor::B as usize][Piece::P as usize] = 0xFFu64 << 48;

    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
}

#[test]
fn encode_single_white_king() {
    let mut bitboards = empty_bitboards();
    bitboards[PieceColor::W as usize][Piece::K as usize] = 1u64 << 4;
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, "8/8/8/8/8/8/8/4K3");
}

#[test]
fn encode_single_black_pawn() {
    let mut bitboards = empty_bitboards();
    bitboards[PieceColor::B as usize][Piece::P as usize] = 1u64 << 56;
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, "p7/8/8/8/8/8/8/8");
}

#[test]
fn encode_mixed_rank() {
    let mut bitboards = empty_bitboards();
    bitboards[PieceColor::W as usize][Piece::R as usize] = 1u64 << 24;
    bitboards[PieceColor::B as usize][Piece::B as usize] = 1u64 << 27;
    bitboards[PieceColor::W as usize][Piece::Q as usize] = 1u64 << 30;
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, "8/8/8/8/R2b2Q1/8/8/8");
}

// ---------- DECODE TESTS (FEN string -> bitboards) ----------

#[test]
fn decode_empty_board() {
    let bitboards = pgn_notation_to_bitboards("8/8/8/8/8/8/8/8");
    assert_eq!(bitboards, empty_bitboards());
}

#[test]
fn decode_single_white_king() {
    let bitboards = pgn_notation_to_bitboards("8/8/8/8/8/8/8/4K3");
    assert_eq!(bitboards[PieceColor::W as usize][Piece::K as usize], 1u64 << 4);
    assert_eq!(bitboards[PieceColor::W as usize][Piece::K as usize].count_ones(), 1);
}

#[test]
fn decode_single_black_pawn() {
    let bitboards = pgn_notation_to_bitboards("p7/8/8/8/8/8/8/8");
    assert_eq!(bitboards[PieceColor::B as usize][Piece::P as usize], 1u64 << 56);
}

#[test]
fn decode_starting_position() {
    let bitboards = pgn_notation_to_bitboards("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
    assert_eq!(bitboards[PieceColor::W as usize][Piece::P as usize], 0xFFu64 << 8);
    assert_eq!(bitboards[PieceColor::B as usize][Piece::P as usize], 0xFFu64 << 48);
    assert_eq!(bitboards[PieceColor::W as usize][Piece::R as usize], (1u64 << 0) | (1u64 << 7));
    assert_eq!(bitboards[PieceColor::B as usize][Piece::K as usize], 1u64 << 60);
    assert_eq!(bitboards[PieceColor::W as usize][Piece::K as usize], 1u64 << 4);
}

#[test]
fn decode_all_pieces_bitboard_matches_union() {
    let bitboards = pgn_notation_to_bitboards("8/8/8/8/8/8/8/RN6");
    let expected_all = (1u64 << 0) | (1u64 << 1);
    assert_eq!(bitboards[PieceColor::W as usize][Piece::A as usize], expected_all);
}

// ---------- ROUND-TRIP TESTS ----------

#[test]
fn round_trip_starting_position() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
    let bitboards = pgn_notation_to_bitboards(fen);
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, fen);
}

#[test]
fn round_trip_sparse_endgame_position() {
    let fen = "4k3/8/8/8/8/8/4P3/4K3";
    let bitboards = pgn_notation_to_bitboards(fen);
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, fen);
}

#[test]
fn round_trip_empty_board() {
    let fen = "8/8/8/8/8/8/8/8";
    let bitboards = pgn_notation_to_bitboards(fen);
    let result = get_pgn_notation_from_bitboards(bitboards);
    assert_eq!(result, fen);
}