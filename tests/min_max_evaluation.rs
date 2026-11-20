// tests/min_max_evaluation.rs
//
// Tests for the alpha-beta min_max search (src/evaluator/MIN_MAX.rs)
// operating on Node_Min_Max trees (src/structures/state.rs), driven by a
// GameSession that is made / reverted move by move.
#[cfg(test)]
mod tests {
    use chess_engine::evaluator::MIN_MAX::{min_max, MATE};
    use chess_engine::parser::fen_to_bitboards;
    use chess_engine::structures::annotations::{Piece, PieceColor};
    use chess_engine::structures::game_session::GameSession;
    use chess_engine::structures::state::{apply_move, Node_Min_Max};

    type Boards = [[u64; 7]; 2];

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

    /// Session + matching root node for a FEN string.
    fn position(fen: &str) -> (GameSession, Node_Min_Max) {
        let session = fen_to_bitboards(fen).expect("invalid FEN");
        let root = Node_Min_Max::new_state_for(session.bitboards, session.side_to_move);
        (session, root)
    }

    /// role for min_max: 1 = white maximizes, -1 = black minimizes.
    fn role_for(node: &Node_Min_Max) -> i8 {
        if node.side_to_move() == 0 { 1 } else { -1 }
    }

    /// Full-window search from `root` to `depth`, returning the score.
    fn search(session: &mut GameSession, root: &mut Node_Min_Max, depth: usize) -> f64 {
        let role = role_for(root);
        min_max(f64::NEG_INFINITY, f64::INFINITY, role, session, root, depth)
    }

    /// Searches a FEN position; returns (score, root).
    fn search_fen(fen: &str, depth: usize) -> (f64, Node_Min_Max) {
        let (mut session, mut root) = position(fen);
        let v = search(&mut session, &mut root, depth);
        (v, root)
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
        let to = moved_to(&root.bitboards, &best.bitboards, mover);
        assert_eq!(best.mv.map(|m| m.to), Some(to), "child.mv must match its board");
        to
    }

    fn is_mate_score(v: f64) -> bool {
        v.abs() >= MATE
    }

    /// Plain minimax with no pruning, using the same leaf / mate / stalemate
    /// rules as min_max. Reference for checking that alpha-beta never changes
    /// the result.
    fn plain_minimax(session: &mut GameSession, node: &mut Node_Min_Max, depth: usize, role: i8) -> f64 {
        if depth == 0 {
            return node.evaluate();
        }
        if !node.expanded {
            node.calculate_children(session);
        }
        if node.children.is_empty() {
            if !node.is_king_checked {
                return 0.0;
            }
            let d = depth as f64;
            return if role == 1 { -(MATE + d) } else { MATE + d };
        }
        let mut best = if role == 1 { f64::NEG_INFINITY } else { f64::INFINITY };
        for child in node.children.iter_mut() {
            let saved = session.clone();
            apply_move(session, child.mv.unwrap());
            let v = plain_minimax(session, child, depth - 1, -role);
            *session = saved;
            best = if role == 1 { best.max(v) } else { best.min(v) };
        }
        best
    }

    // ------------------------------------------------------------------
    // Sanity
    // ------------------------------------------------------------------

    #[test]
    fn depth_zero_returns_static_eval() {
        let (mut session, mut root) = position(START);
        let expected = root.evaluate();
        let v = search(&mut session, &mut root, 0);
        assert_eq!(v, expected);
        assert!(!root.expanded, "depth 0 must not expand the node");
        assert!(root.best_child().is_none());
    }

    #[test]
    fn depth_one_picks_a_best_child() {
        let (v, root) = search_fen(START, 1);
        assert!(root.expanded);
        assert_eq!(root.children.len(), 20, "start position has 20 legal moves");
        assert!(root.best_child().is_some());
        assert_eq!(root.value, v);
    }

    #[test]
    fn start_position_is_roughly_equal() {
        for depth in 1..=3 {
            let (v, _) = search_fen(START, depth);
            assert!(v.abs() < 100.0, "depth {depth}: start position scored {v}");
        }
    }

    #[test]
    fn search_reverts_the_session() {
        let (mut session, mut root) = position(START);
        let before = session.clone();
        search(&mut session, &mut root, 3);
        assert_eq!(session.bitboards, before.bitboards);
        assert_eq!(session.side_to_move, before.side_to_move);
        assert_eq!(session.castling, before.castling);
        assert_eq!(session.ep_target, before.ep_target);
        assert_eq!(session.halfmove, before.halfmove);
        assert_eq!(session.fullmove, before.fullmove);
        assert_eq!(root.bitboards, before.bitboards);
    }

    #[test]
    fn game_session_evaluate_does_not_touch_the_game() {
        let (session, _) = position(START);
        let root = session.evaluate(2);
        assert!(root.best_child().is_some());
        assert_eq!(session.bitboards, fen_to_bitboards(START).unwrap().bitboards);
    }

    #[test]
    fn game_session_evaluate_matches_min_max() {
        let (session, _) = position(WHITE_WINS_QUEEN);
        let root = session.evaluate(2);
        let (v, direct) = search_fen(WHITE_WINS_QUEEN, 2);
        assert_eq!(root.value, v);
        assert_eq!(best_destination(&root), best_destination(&direct));
    }

    #[test]
    fn searching_an_expanded_tree_again_gives_same_result() {
        let (mut session, mut root) = position(WHITE_WINS_QUEEN);
        let first = search(&mut session, &mut root, 3);
        let first_move = best_destination(&root);
        let second = search(&mut session, &mut root, 3);
        assert_eq!(first, second);
        assert_eq!(best_destination(&root), first_move);
    }

    // ------------------------------------------------------------------
    // Move generation through the session (castling / promotion)
    // ------------------------------------------------------------------

    #[test]
    fn castling_is_generated() {
        let (mut session, mut root) = position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        root.calculate_children(&mut session);
        let king_dests: Vec<u64> = root
            .children
            .iter()
            .filter_map(|c| c.mv)
            .filter(|m| m.from == sq("e1"))
            .map(|m| m.to)
            .collect();
        assert!(king_dests.contains(&sq("g1")), "O-O missing");
        assert!(king_dests.contains(&sq("c1")), "O-O-O missing");
    }

    #[test]
    fn promotion_generates_all_four_pieces() {
        let (mut session, mut root) = position("4k3/P7/8/8/8/8/8/4K3 w - - 0 1");
        root.calculate_children(&mut session);
        let promos: Vec<Piece> = root
            .children
            .iter()
            .filter_map(|c| c.mv)
            .filter(|m| m.from == sq("a7"))
            .map(|m| m.promo)
            .collect();
        assert_eq!(promos.len(), 4);
        for p in [Piece::Q, Piece::R, Piece::B, Piece::N] {
            assert!(promos.contains(&p), "missing promotion to {p:?}");
        }
    }

    // ------------------------------------------------------------------
    // Tactics
    // ------------------------------------------------------------------

    #[test]
    fn white_captures_hanging_queen() {
        for depth in 1..=3 {
            let (v, root) = search_fen(WHITE_WINS_QUEEN, depth);
            assert_eq!(best_destination(&root), sq("d5"), "depth {depth}");
            assert!(v > 0.0, "depth {depth}: white should be winning, got {v}");
        }
    }

    #[test]
    fn black_captures_hanging_queen() {
        for depth in 1..=3 {
            let (v, root) = search_fen(BLACK_WINS_QUEEN, depth);
            assert_eq!(best_destination(&root), sq("d4"), "depth {depth}");
            assert!(v < 0.0, "depth {depth}: black should be winning, got {v}");
        }
    }

    #[test]
    fn mirrored_positions_get_opposite_scores() {
        for depth in 1..=3 {
            let (w, _) = search_fen(WHITE_WINS_QUEEN, depth);
            let (b, _) = search_fen(BLACK_WINS_QUEEN, depth);
            assert_eq!(w, -b, "depth {depth}");
        }
    }

    #[test]
    fn finds_mate_in_one() {
        let (v, root) = search_fen(MATE_IN_ONE, 2);
        assert!(is_mate_score(v) && v > 0.0, "expected white mate score, got {v}");
        assert_eq!(best_destination(&root), sq("a8"));
    }

    #[test]
    fn greedy_depth_one_takes_poisoned_pawn() {
        // At depth 1 the engine cannot see the recapture, so it grabs the pawn.
        let (_, root) = search_fen(POISONED_PAWN, 1);
        assert_eq!(best_destination(&root), sq("d5"));
    }

    #[test]
    fn avoids_losing_piece_at_depth_two() {
        let (v, root) = search_fen(POISONED_PAWN, 2);
        assert_ne!(best_destination(&root), sq("d5"), "Qxd5 loses the queen to exd5");
        assert!(v > 0.0, "white is still a queen up, got {v}");
    }

    // ------------------------------------------------------------------
    // Terminal positions
    // ------------------------------------------------------------------

    #[test]
    fn checkmated_side_gets_mate_score() {
        for depth in 1..=3 {
            let (v, root) = search_fen(BLACK_IS_MATED, depth);
            assert!(root.children.is_empty(), "black should have no legal moves");
            assert!(root.is_king_checked);
            assert!(root.best_child().is_none());
            // Black to move and mated -> good for white, offset by remaining depth.
            assert_eq!(v, MATE + depth as f64, "depth {depth}");
        }
    }

    #[test]
    fn stalemate_is_scored_as_draw() {
        for depth in 1..=3 {
            let (v, root) = search_fen(STALEMATE, depth);
            assert!(root.children.is_empty(), "black should have no legal moves");
            assert!(!root.is_king_checked);
            assert_eq!(v, 0.0, "depth {depth}: stalemate must score 0, got {v}");
        }
    }

    // ------------------------------------------------------------------
    // Alpha-beta consistency
    // ------------------------------------------------------------------

    #[test]
    fn alpha_beta_matches_plain_minimax() {
        let positions = [START, WHITE_WINS_QUEEN, BLACK_WINS_QUEEN, MATE_IN_ONE, POISONED_PAWN];
        for fen in positions {
            for depth in 1..=3 {
                let (pruned, _) = search_fen(fen, depth);
                let (mut session, mut root) = position(fen);
                let role = role_for(&root);
                let full = plain_minimax(&mut session, &mut root, depth, role);
                assert_eq!(pruned, full, "{fen} at depth {depth}");
            }
        }
    }
}
