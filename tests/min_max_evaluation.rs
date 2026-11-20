// tests/min_max_evaluation.rs
//
// Tests for the alpha-beta min_max search (src/evaluator/MIN_MAX.rs)
// operating on Node_Min_Max trees (src/structures/state.rs).
#[cfg(test)]
mod tests {
    use chess_engine::evaluator::MIN_MAX::min_max;
    use chess_engine::parser::fen_to_bitboards;
    use chess_engine::structures::annotations::{Piece, PieceColor};
    use chess_engine::structures::state::Node_Min_Max;

    type Boards = [[u64; 7]; 2];

    const MATE: f64 = 1.0e9;
    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    // White rook on d1 can take an undefended black queen on d5.
    const WHITE_WINS_QUEEN: &str = "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1";
    // Colour-flipped mirror of the above: black rook d8 takes white queen d4.
    const BLACK_WINS_QUEEN: &str = "3rk3/8/8/8/3Q4/8/8/4K3 b - - 0 1";
    // Back-rank mate: Ra1-a8#.
    const MATE_IN_ONE: &str = "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1";
    // Position after Ra8#: black to move with no legal moves.
    const BLACK_IS_MATED: &str = "R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1";
    // Black king h8 has no moves but is not in check.
    const STALEMATE: &str = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1";
    // Qxd5 wins a pawn but loses the queen to exd5.
    const POISONED_PAWN: &str = "4k3/8/4p3/3p4/8/8/8/3QK3 w - - 0 1";

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// "e4" -> single-bit mask (a1 = bit 0, h8 = bit 63).
    fn sq(name: &str) -> u64 {
        let b = name.as_bytes();
        let file = (b[0] - b'a') as u32;
        let rank = (b[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
    }

    /// Build a root node from a FEN string (side to move taken from the FEN).
    fn root_from_fen(fen: &str) -> Node_Min_Max {
        let session = fen_to_bitboards(fen).expect("invalid FEN");
        Node_Min_Max::new_state_for(session.bitboards, session.side_to_move)
    }

    /// role for min_max: 1 = white maximizes, -1 = black minimizes.
    fn role_for(node: &Node_Min_Max) -> i8 {
        if node.side_to_move() == 0 { 1 } else { -1 }
    }

    /// Run a full-window search from `root` to `depth`, returning the score.
    fn search(root: &mut Node_Min_Max, depth: usize) -> f64 {
        let role = role_for(root);
        min_max(f64::NEG_INFINITY, f64::INFINITY, role, root, depth)
    }

    /// Square the side `color` moved a piece to, going from `before` to `after`.
    fn moved_to(before: &Boards, after: &Boards, color: PieceColor) -> u64 {
        let c = color as usize;
        after[c][Piece::A as usize] & !before[c][Piece::A as usize]
    }

    /// Destination square of the move min_max chose at the root.
    fn best_destination(root: &Node_Min_Max) -> u64 {
        let best = root.best_child().expect("min_max chose no move");
        let mover = if root.side_to_move() == 0 { PieceColor::W } else { PieceColor::B };
        moved_to(&root.bitboards, &best.bitboards, mover)
    }

    fn is_mate_score(v: f64) -> bool {
        v.abs() >= MATE
    }

    /// Plain minimax with no pruning, using the same leaf / mate rules as min_max.
    /// Used as a reference to check that alpha-beta never changes the result.
    fn plain_minimax(node: &mut Node_Min_Max, depth: usize, role: i8) -> f64 {
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
        let scores = node
            .children
            .iter_mut()
            .map(|c| plain_minimax(c, depth - 1, -role));
        if role == 1 {
            scores.fold(f64::NEG_INFINITY, f64::max)
        } else {
            scores.fold(f64::INFINITY, f64::min)
        }
    }

    // ------------------------------------------------------------------
    // Sanity
    // ------------------------------------------------------------------

    #[test]
    fn depth_zero_returns_static_eval() {
        let mut root = root_from_fen(START);
        let expected = root.evaluate();
        let v = search(&mut root, 0);
        assert_eq!(v, expected);
        assert!(!root.expanded, "depth 0 must not expand the node");
        assert!(root.best_child().is_none());
    }

    #[test]
    fn depth_one_picks_a_best_child() {
        let mut root = root_from_fen(START);
        let v = search(&mut root, 1);
        assert!(root.expanded);
        assert_eq!(root.children.len(), 20, "start position has 20 legal moves");
        assert!(root.best_child().is_some());
        assert_eq!(root.value, v);
    }

    #[test]
    fn start_position_is_roughly_equal() {
        for depth in 1..=3 {
            let mut root = root_from_fen(START);
            let v = search(&mut root, depth);
            assert!(v.abs() < 100.0, "depth {depth}: start position scored {v}");
        }
    }

    #[test]
    fn search_does_not_change_root_board() {
        let mut root = root_from_fen(START);
        let before = root.bitboards;
        search(&mut root, 3);
        assert_eq!(root.bitboards, before);
    }

    #[test]
    fn searching_an_expanded_tree_again_gives_same_result() {
        let mut root = root_from_fen(WHITE_WINS_QUEEN);
        let first = search(&mut root, 3);
        let first_move = best_destination(&root);
        let second = search(&mut root, 3);
        assert_eq!(first, second);
        assert_eq!(best_destination(&root), first_move);
    }

    // ------------------------------------------------------------------
    // Tactics
    // ------------------------------------------------------------------

    #[test]
    fn white_captures_hanging_queen() {
        for depth in 1..=3 {
            let mut root = root_from_fen(WHITE_WINS_QUEEN);
            let v = search(&mut root, depth);
            assert_eq!(best_destination(&root), sq("d5"), "depth {depth}");
            assert!(v > 0.0, "depth {depth}: white should be winning, got {v}");
        }
    }

    #[test]
    fn black_captures_hanging_queen() {
        for depth in 1..=3 {
            let mut root = root_from_fen(BLACK_WINS_QUEEN);
            let v = search(&mut root, depth);
            assert_eq!(best_destination(&root), sq("d4"), "depth {depth}");
            assert!(v < 0.0, "depth {depth}: black should be winning, got {v}");
        }
    }

    #[test]
    fn mirrored_positions_get_opposite_scores() {
        for depth in 1..=3 {
            let w = search(&mut root_from_fen(WHITE_WINS_QUEEN), depth);
            let b = search(&mut root_from_fen(BLACK_WINS_QUEEN), depth);
            assert_eq!(w, -b, "depth {depth}");
        }
    }

    #[test]
    fn finds_mate_in_one() {
        let mut root = root_from_fen(MATE_IN_ONE);
        let v = search(&mut root, 2);
        assert!(is_mate_score(v) && v > 0.0, "expected white mate score, got {v}");
        assert_eq!(best_destination(&root), sq("a8"));
    }

    #[test]
    fn greedy_depth_one_takes_poisoned_pawn() {
        // At depth 1 the engine cannot see the recapture, so it grabs the pawn.
        let mut root = root_from_fen(POISONED_PAWN);
        search(&mut root, 1);
        assert_eq!(best_destination(&root), sq("d5"));
    }

    #[test]
    fn avoids_losing_piece_at_depth_two() {
        let mut root = root_from_fen(POISONED_PAWN);
        let v = search(&mut root, 2);
        assert_ne!(best_destination(&root), sq("d5"), "Qxd5 loses the queen to exd5");
        assert!(v > 0.0, "white is still a queen up, got {v}");
    }

    // ------------------------------------------------------------------
    // Terminal positions
    // ------------------------------------------------------------------

    #[test]
    fn checkmated_side_gets_mate_score() {
        for depth in 1..=3 {
            let mut root = root_from_fen(BLACK_IS_MATED);
            let v = search(&mut root, depth);
            assert!(root.children.is_empty(), "black should have no legal moves");
            assert!(root.best_child().is_none());
            // Black to move and mated -> good for white, offset by remaining depth.
            assert_eq!(v, MATE + depth as f64, "depth {depth}");
        }
    }

    #[test]
    #[ignore = "min_max scores every no-legal-moves position as mate; stalemate detection is not implemented"]
    fn stalemate_is_not_scored_as_mate() {
        let mut root = root_from_fen(STALEMATE);
        let v = search(&mut root, 1);
        assert!(root.children.is_empty(), "black should have no legal moves");
        assert!(!is_mate_score(v), "stalemate must not be scored as mate, got {v}");
    }

    // ------------------------------------------------------------------
    // Alpha-beta consistency
    // ------------------------------------------------------------------

    #[test]
    fn alpha_beta_matches_plain_minimax() {
        let positions = [START, WHITE_WINS_QUEEN, BLACK_WINS_QUEEN, MATE_IN_ONE, POISONED_PAWN];
        for fen in positions {
            for depth in 1..=3 {
                let mut a = root_from_fen(fen);
                let mut b = root_from_fen(fen);
                let role = role_for(&a);
                let pruned = search(&mut a, depth);
                let full = plain_minimax(&mut b, depth, role);
                assert_eq!(pruned, full, "{fen} at depth {depth}");
            }
        }
    }
}
