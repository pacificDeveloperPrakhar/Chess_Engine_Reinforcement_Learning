// tests/min_max.rs
//
// Adjust these import paths — min_max's module location wasn't given explicitly.
#[cfg(test)]
mod tests {
    use chess_engine::structures::state::Node_Min_Max;
    use chess_engine::evaluator::MIN_MAX::min_max;
    use chess_engine::generator::is_king_checked;
    use chess_engine::structures::annotations::{Piece, PieceColor};

    type Boards = [[u64; 7]; 2];

    fn sq(name: &str) -> u64 {
        let bytes = name.as_bytes();
        let file = (bytes[0] - b'a') as u32;
        let rank = (bytes[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
    }

    fn empty_board() -> Boards { [[0u64; 7]; 2] }

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

    // ------------------------------------------------------------------
    // Reference full-width minimax, mirroring min_max's OWN terminal
    // formula exactly (including the stalemate/checkmate conflation),
    // so this checks only that alpha-beta pruning preserves the value —
    // it is NOT a correctness oracle for the stalemate bug below.
    // ------------------------------------------------------------------
    const MATE: f64 = 1.0e9;

    fn reference_full_width(role: i8, node: &mut Node_Min_Max, depth: usize) -> f64 {
        if depth == 0 {
            return node.evaluate();
        }
        if !node.expanded {
            node.calculate_children();
        }
        if node.children.is_empty() {
            let d = depth as f64;
            return if role == 1 { -(MATE + d) } else { MATE + d };
        }
        let mut best = if role == 1 { f64::NEG_INFINITY } else { f64::INFINITY };
        for child in node.children.iter_mut() {
            let v = reference_full_width(-role, child, depth - 1);
            if role == 1 { best = best.max(v); } else { best = best.min(v); }
        }
        best
    }

    // ------------------------------------------------------------------
    // Depth 0: pure leaf evaluation, no search
    // ------------------------------------------------------------------
    #[test]
    fn depth_zero_returns_static_evaluation_directly() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 0);
        let expected = node.evaluate();
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, 1, &mut node, 0);
        assert_eq!(v, expected);
    }

    // ------------------------------------------------------------------
    // THE BUG: stalemate must be a draw (0.0), not a mate score.
    //
    // Classic K+Q stalemate: White K f7, White Q g6, Black K h8, black to
    // move. Black's king has no legal moves (g8/g7/h7 all controlled by
    // the queen and/or king), but black is NOT in check.
    // ------------------------------------------------------------------
    #[test]
    fn stalemate_position_has_no_legal_black_moves() {
        // sanity-check the position itself before trusting the min_max result
        let bb = board(&[
            (PieceColor::W, Piece::K, "f7"),
            (PieceColor::W, Piece::Q, "g6"),
            (PieceColor::B, Piece::K, "h8"),
        ]);
        let (attackers, _) = is_king_checked(bb, 'b');
        assert_eq!(attackers, 0, "black king must NOT be in check — this must be stalemate, not checkmate");

        let mut node = Node_Min_Max::new_state_for(bb, 1); // black to move
        node.calculate_children();
        assert!(node.children.is_empty(), "black should have zero legal moves in this position");
    }

    #[test]
    fn stalemate_should_evaluate_as_a_draw_not_a_mate() {
        // EXPECTED (correct chess rules): stalemate is scored 0.0.
        // CURRENT BEHAVIOR (bug): min_max cannot distinguish stalemate
        // from checkmate and will return a huge score instead, because
        // it never calls is_king_checked before treating an empty
        // children list as "mate". This test is expected to FAIL against
        // the current implementation — that failure IS the bug report.
        let bb = board(&[
            (PieceColor::W, Piece::K, "f7"),
            (PieceColor::W, Piece::Q, "g6"),
            (PieceColor::B, Piece::K, "h8"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 1); // black to move, role = -1
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, -1, &mut node, 1);
        assert_eq!(v, 0.0, "stalemate must evaluate as a draw, got {v} instead (mate-vs-stalemate bug)");
    }

    // ------------------------------------------------------------------
    // Contrast case: genuine checkmate SHOULD score as an extreme value.
    // White K f6, White Q g7, Black K h8, black to move, in check,
    // zero escape squares — real back-rank-style mate.
    // ------------------------------------------------------------------
    #[test]
    fn checkmate_position_has_black_in_check_with_no_legal_moves() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "f6"),
            (PieceColor::W, Piece::Q, "g7"),
            (PieceColor::B, Piece::K, "h8"),
        ]);
        let (attackers, _) = is_king_checked(bb, 'b');
        assert_ne!(attackers, 0, "black king must be in check for this to be checkmate");

        let mut node = Node_Min_Max::new_state_for(bb, 1);
        node.calculate_children();
        assert!(node.children.is_empty(), "black should have zero legal replies to checkmate");
    }

    #[test]
    fn checkmate_scores_as_an_extreme_value_favoring_white() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "f6"),
            (PieceColor::W, Piece::Q, "g7"),
            (PieceColor::B, Piece::K, "h8"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 1); // black to move, role = -1
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, -1, &mut node, 1);
        assert!(v > 1.0e8, "checkmate favoring white should be a huge positive score, got {v}");
    }

    // ------------------------------------------------------------------
    // Role sign convention: white (role=1) should find and prefer a free
    // capture; the resulting score should be clearly positive.
    // ------------------------------------------------------------------
    #[test]
    fn white_to_move_finds_a_free_capture_and_scores_positively() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "a8"),
            (PieceColor::B, Piece::K, "h1"),
            (PieceColor::B, Piece::R, "a1"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 0); // white to move, role = 1
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, 1, &mut node, 1);
        assert!(v > 0.0, "white should find the free rook capture and score positively, got {v}");
    }

    #[test]
    fn black_to_move_finds_a_free_capture_and_scores_negatively() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "h1"),
            (PieceColor::W, Piece::R, "a1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::Q, "a8"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 1); // black to move, role = -1
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, -1, &mut node, 1);
        assert!(v < 0.0, "black should find the free rook capture and score negatively, got {v}");
    }

    // ------------------------------------------------------------------
    // Alpha-beta pruning must not change the returned value versus a
    // full-width search of the identical tree (same terminal formula).
    // ------------------------------------------------------------------
    #[test]
    fn alpha_beta_result_matches_full_width_search_depth_one() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::N, "b1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::N, "b8"),
        ]);
        let mut node_ab = Node_Min_Max::new_state_for(bb, 0);
        let mut node_full = Node_Min_Max::new_state_for(bb, 0);

        let ab_value = min_max(f64::NEG_INFINITY, f64::INFINITY, 1, &mut node_ab, 1);
        let full_value = reference_full_width(1, &mut node_full, 1);

        assert_eq!(ab_value, full_value, "alpha-beta pruning must not change the minimax value");
    }

    #[test]
    fn alpha_beta_result_matches_full_width_search_depth_two() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::N, "b1"),
            (PieceColor::B, Piece::K, "e8"),
            (PieceColor::B, Piece::N, "b8"),
        ]);
        let mut node_ab = Node_Min_Max::new_state_for(bb, 0);
        let mut node_full = Node_Min_Max::new_state_for(bb, 0);

        let ab_value = min_max(f64::NEG_INFINITY, f64::INFINITY, 1, &mut node_ab, 2);
        let full_value = reference_full_width(1, &mut node_full, 2);

        assert_eq!(ab_value, full_value, "alpha-beta pruning must not change the minimax value at depth 2");
    }

    // ------------------------------------------------------------------
    // curr.value should be left holding the value that was returned.
    // ------------------------------------------------------------------
    #[test]
    fn root_value_field_is_set_to_the_returned_score() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::Q, "d1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);
        let mut node = Node_Min_Max::new_state_for(bb, 0);
        let v = min_max(f64::NEG_INFINITY, f64::INFINITY, 1, &mut node, 1);
        assert_eq!(node.value, v);
    }
}