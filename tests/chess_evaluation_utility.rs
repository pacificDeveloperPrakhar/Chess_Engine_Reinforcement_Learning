// tests/evaluate.rs
//
// Adjust this import to your actual module path.
#[cfg(test)]
mod tests {
    use chess_engine::evaluator::{
        evaluate_material_score, evaluate_psqt_score, evaluate_score, game_progress, how_sparse,
    }; // <-- fix path if different
    use chess_engine::structures::annotations::{Piece, PieceColor};

    type Boards = [[u64; 7]; 2];

    fn sq(name: &str) -> u64 {
        let bytes = name.as_bytes();
        let file = (bytes[0] - b'a') as u32;
        let rank = (bytes[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
    }
    fn sq_index(name: &str) -> usize {
        let bytes = name.as_bytes();
        let file = (bytes[0] - b'a') as usize;
        let rank = (bytes[1] - b'1') as usize;
        rank * 8 + file
    }

    fn empty_board() -> Boards {
        [[0u64; 7]; 2]
    }

    fn put(bb: &mut Boards, color: PieceColor, piece: Piece, square: &str) {
        let c = color as usize;
        let s = sq(square);
        bb[c][piece as usize] |= s;
        bb[c][Piece::A as usize] |= s;
    }

    fn board(pieces: &[(PieceColor, Piece, &str)]) -> Boards {
        let mut bb = empty_board();
        for &(color, piece, square) in pieces {
            put(&mut bb, color, piece, square);
        }
        bb
    }

    fn fill_n_dummy_pawns(bb: &mut Boards, color: PieceColor, count: usize) {
        // spread across b2..g2 / b7..g7-ish squares just to control a raw piece count;
        // doesn't need to be a legal position, only used for game_progress's total count.
        let files = ["a", "b", "c", "d", "e", "f", "g", "h"];
        let rank = if color == PieceColor::W { "2" } else { "7" };
        for i in 0..count.min(8) {
            let square: String = format!("{}{}", files[i], rank);
            put(bb, color, Piece::P, &square);
        }
    }

    // ---------------------------------------------------------------- how_sparse

    #[test]
    fn how_sparse_always_returns_zero_currently() {
        // documents the current TODO/no-op state explicitly, so a future
        // implementation change is caught by this test needing an update.
        assert_eq!(how_sparse(empty_board()), 0);
        let full = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::Q, "d8"),
        ]);
        assert_eq!(how_sparse(full), 0);
    }

    // ---------------------------------------------------------------- game_progress

    #[test]
    fn game_progress_full_board_is_opening() {
        let mut bb = empty_board();
        put(&mut bb, PieceColor::W, Piece::K, "e1");
        put(&mut bb, PieceColor::B, Piece::K, "e8");
        fill_n_dummy_pawns(&mut bb, PieceColor::W, 8);
        fill_n_dummy_pawns(&mut bb, PieceColor::B, 8);
        put(&mut bb, PieceColor::W, Piece::Q, "d1");
        put(&mut bb, PieceColor::B, Piece::Q, "d8");
        put(&mut bb, PieceColor::W, Piece::R, "a1");
        put(&mut bb, PieceColor::W, Piece::R, "h1");
        put(&mut bb, PieceColor::B, Piece::R, "a8");
        put(&mut bb, PieceColor::B, Piece::R, "h8");
        put(&mut bb, PieceColor::W, Piece::B, "c1");
        put(&mut bb, PieceColor::B, Piece::B, "c8");
        put(&mut bb, PieceColor::W, Piece::N, "b1");
        put(&mut bb, PieceColor::B, Piece::N, "b8");
        // total pieces = 2 kings + 16 pawns + 2Q + 4R + 2B + 2N = 28 > 24
        assert_eq!(game_progress(bb), 0);
    }

    #[test]
    fn game_progress_reduced_material_is_middlegame() {
        let mut bb = empty_board();
        put(&mut bb, PieceColor::W, Piece::K, "e1");
        put(&mut bb, PieceColor::B, Piece::K, "e8");
        fill_n_dummy_pawns(&mut bb, PieceColor::W, 6);
        fill_n_dummy_pawns(&mut bb, PieceColor::B, 6);
        put(&mut bb, PieceColor::W, Piece::Q, "d1");
        put(&mut bb, PieceColor::B, Piece::Q, "d8");
        put(&mut bb, PieceColor::W, Piece::R, "a1");
        put(&mut bb, PieceColor::B, Piece::R, "a8");
        // total = 2 + 12 + 2 + 2 = 18, which is >12 and <=24
        assert_eq!(game_progress(bb), 1);
    }

    #[test]
    fn game_progress_few_pieces_is_endgame() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::W, Piece::P, "e4"),
        ]);
        // total = 3, well under 12
        assert_eq!(game_progress(bb), 2);
    }

    #[test]
    fn game_progress_boundary_at_twelve_is_middlegame_not_endgame() {
        let mut bb = empty_board();
        put(&mut bb, PieceColor::W, Piece::K, "e1");
        put(&mut bb, PieceColor::B, Piece::K, "e8");
        fill_n_dummy_pawns(&mut bb, PieceColor::W, 6);
        fill_n_dummy_pawns(&mut bb, PieceColor::B, 5);
        // total = 2 + 6 + 5 = 13, just above 12
        assert_eq!(game_progress(bb), 1, "13 pieces should still be middlegame (>12)");
    }

    #[test]
    fn game_progress_boundary_at_twenty_four_is_opening() {
        // build exactly 25 pieces (>24) to hit the opening branch at the boundary
        let mut bb = empty_board();
        put(&mut bb, PieceColor::W, Piece::K, "e1");
        put(&mut bb, PieceColor::B, Piece::K, "e8");
        fill_n_dummy_pawns(&mut bb, PieceColor::W, 8);
        fill_n_dummy_pawns(&mut bb, PieceColor::B, 8);
        put(&mut bb, PieceColor::W, Piece::Q, "d1");
        put(&mut bb, PieceColor::B, Piece::Q, "d8");
        put(&mut bb, PieceColor::W, Piece::R, "a1");
        put(&mut bb, PieceColor::W, Piece::R, "h1");
        put(&mut bb, PieceColor::B, Piece::R, "a8");
        // total = 2 + 16 + 2 + 3 = 23... adjust up by 2 more pieces to clear 24
        put(&mut bb, PieceColor::B, Piece::R, "h8");
        put(&mut bb, PieceColor::W, Piece::B, "c1");
        // total = 2 + 16 + 2 + 4 + 1 = 25
        assert_eq!(game_progress(bb), 0, "25 pieces should be opening (>24)");
    }

    // ---------------------------------------------------------------- evaluate_material_score

    #[test]
    fn material_score_is_zero_for_a_bare_symmetric_position() {
        let bb = board(&[(PieceColor::W, Piece::K, "e1"), (PieceColor::B, Piece::K, "e8")]);
        assert_eq!(evaluate_material_score(bb), 0, "kings alone, symmetric, should net to 0");
    }

    #[test]
    fn material_score_is_zero_when_both_sides_have_identical_material() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::W, Piece::R, "a1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::Q, "d8"),
            (PieceColor::B, Piece::R, "a8"),
        ]);
        assert_eq!(evaluate_material_score(bb), 0);
    }

    #[test]
    fn material_score_is_the_negation_when_colors_are_swapped() {
        let white_up_a_queen = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let black_up_a_queen = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::Q, "d8"),
        ]);
        assert_eq!(
            evaluate_material_score(white_up_a_queen),
            -evaluate_material_score(black_up_a_queen),
            "swapping which side has the extra queen should exactly negate the score"
        );
    }

    #[test]
    fn material_score_two_extra_pawns_is_double_one_extra_pawn() {
        let one_pawn = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::P, "e2"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let two_pawns = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::P, "e2"),
            (PieceColor::W, Piece::P, "d2"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        assert_eq!(
            evaluate_material_score(two_pawns),
            2 * evaluate_material_score(one_pawn),
            "material score must scale linearly with piece count"
        );
    }

    // ---------------------------------------------------------------- evaluate_psqt_score

    #[test]
    fn psqt_score_is_zero_for_kings_alone_symmetric_position() {
        // kings on their mirrored home squares; PSQT[king][e1] should equal PSQT[king][e8^56]=PSQT[king][e1]
        // so black's contribution exactly cancels white's, regardless of actual table values.
        let bb = board(&[(PieceColor::W, Piece::K, "e1"), (PieceColor::B, Piece::K, "e8")]);
        assert_eq!(evaluate_psqt_score(bb), 0.0);
    }

    #[test]
    fn psqt_score_cancels_for_any_piece_on_mirrored_squares() {
        // White knight on e2, black knight on e7 (e7 = mirror of e2 under rank flip).
        // Both use the same underlying table cell (just opposite sign), so regardless
        // of the actual PSQT values, these two contributions must exactly cancel.
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::W, Piece::N, "e2"),
            (PieceColor::B, Piece::N, "e7"),
        ]);
        assert_eq!(
            evaluate_psqt_score(bb),
            0.0,
            "a piece and its color-mirrored counterpart on the mirrored square must cancel exactly"
        );
    }

    #[test]
    fn psqt_score_negates_when_the_same_single_piece_swaps_color_and_mirrors_square() {
        // white knight e2 alone vs. black knight e7 alone: same table cell, opposite sign.
        let white_e2 = board(&[(PieceColor::W, Piece::N, "e2")]);
        let black_e7 = board(&[(PieceColor::B, Piece::N, "e7")]);
        assert_eq!(
            evaluate_psqt_score(white_e2),
            -evaluate_psqt_score(black_e7),
            "white on X and black on mirror(X) should score with opposite sign, same magnitude"
        );
    }

    #[test]
    fn psqt_score_is_zero_on_an_empty_board() {
        assert_eq!(evaluate_psqt_score(empty_board()), 0.0);
    }

    #[test]
    fn psqt_mirror_math_is_self_consistent_for_every_square() {
        // Confirms sq ^ 56 is an involution that correctly pairs every square
        // with its vertical mirror (a1<->a8, e2<->e7, h1<->h8, etc.), independent
        // of table contents — this is what the psqt cancellation tests above rely on.
        for name in [
            "a1", "h1", "a8", "h8", "e1", "e8", "e2", "e7", "d4", "d5",
        ] {
            let idx = sq_index(name);
            let mirrored = idx ^ 56;
            let back = mirrored ^ 56;
            assert_eq!(back, idx, "mirroring twice must return to the original square");
        }
        // spot-check a couple of concrete expected mirror pairs
        assert_eq!(sq_index("e2") ^ 56, sq_index("e7"));
        assert_eq!(sq_index("a1") ^ 56, sq_index("a8"));
        assert_eq!(sq_index("h1") ^ 56, sq_index("h8"));
    }

    // ---------------------------------------------------------------- evaluate_score (combined)

    #[test]
    fn evaluate_score_is_zero_for_a_fully_symmetric_position() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::W, Piece::N, "b1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::Q, "d8"),
            (PieceColor::B, Piece::N, "b8"),
        ]);
        assert_eq!(evaluate_score(bb), 0.0);
    }

    #[test]
    fn evaluate_score_equals_the_sum_of_its_two_components() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::N, "e4"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let expected = evaluate_material_score(bb) as f64 + evaluate_psqt_score(bb);
        assert_eq!(evaluate_score(bb), expected);
    }

    #[test]
    fn evaluate_score_material_dominates_over_position_for_a_full_extra_piece() {
        // ASSUMPTION: piece values in PSQT::PIECE_VALUES are large relative to
        // positional table swings (true for any conventional weighting, e.g.
        // queen ~900 vs typical psqt bonuses in the tens). If this fails, it's
        // worth checking PIECE_VALUES are in a sane, standard range.
        let up_a_queen = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let equal_material = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        assert!(
            evaluate_score(up_a_queen) > evaluate_score(equal_material),
            "being up a whole queen should outweigh any positional difference"
        );
    }
}