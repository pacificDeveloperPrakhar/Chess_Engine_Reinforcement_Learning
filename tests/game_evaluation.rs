// tests/game_session.rs
//
// Adjust this import path to wherever GameSession lives in your crate.
#[cfg(test)]
mod tests {
    use chess_engine::structures::game_session::{GamePlay, GameSession, GameStatus, mode as Mode};
    use chess_engine::structures::annotations::Piece;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------
    const P: usize = Piece::P as usize;
    const N: usize = Piece::N as usize;
    const B: usize = Piece::B as usize;
    const R: usize = Piece::R as usize;
    const Q: usize = Piece::Q as usize;
    const K: usize = Piece::K as usize;
    const ALL: usize = Piece::A as usize;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn sq(name: &str) -> u64 {
        let b = name.as_bytes();
        let file = (b[0] - b'a') as u32;
        let rank = (b[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
    }

    fn mask(names: &[&str]) -> u64 {
        names.iter().fold(0u64, |acc, n| acc | sq(n))
    }

    fn from_fen(fen: &str) -> GameSession {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut s = GameSession::new();
        let mut bb = [[0u64; 7]; 2];

        for (i, row) in parts[0].split('/').enumerate() {
            let rank = 7 - i as u32;
            let mut file = 0u32;
            for ch in row.chars() {
                if let Some(d) = ch.to_digit(10) {
                    file += d;
                    continue;
                }
                let color = if ch.is_ascii_uppercase() { 0 } else { 1 };
                let piece = match ch.to_ascii_lowercase() {
                    'p' => P, 'n' => N, 'b' => B, 'r' => R, 'q' => Q, 'k' => K,
                    _ => panic!("bad fen char {ch}"),
                };
                let bit = 1u64 << (rank * 8 + file);
                bb[color][piece] |= bit;
                bb[color][ALL] |= bit;
                file += 1;
            }
        }
        s.bitboards = bb;
        s.side_to_move = if parts[1] == "w" { 0 } else { 1 };
        s.castling = parts[2].to_string();
        s.ep_target = if parts[3] == "-" { 0 } else { sq(parts[3]) };
        s.halfmove = parts[4].parse().unwrap();
        s.fullmove = parts[5].parse().unwrap();
        s
    }

    /// Compare the full game state against the state described by a FEN.
    fn assert_state(s: &GameSession, fen: &str) {
        let e = from_fen(fen);
        assert_eq!(s.bitboards, e.bitboards, "bitboards differ from expected FEN: {fen}");
        assert_eq!(s.side_to_move, e.side_to_move, "side_to_move");
        assert_eq!(s.castling, e.castling, "castling rights");
        assert_eq!(s.ep_target, e.ep_target, "en-passant target");
        assert_eq!(s.halfmove, e.halfmove, "halfmove clock");
        assert_eq!(s.fullmove, e.fullmove, "fullmove number");
        // ALL board must always equal union of the six piece boards
        for c in 0..2 {
            let union = (0..6).fold(0u64, |a, p| a | s.bitboards[c][p]);
            assert_eq!(s.bitboards[c][ALL], union, "ALL bitboard inconsistent for side {c}");
        }
    }

    fn assert_idle(s: &GameSession) {
        assert_eq!(s.mode, Mode::select);
        assert_eq!(s.selected, 0);
        assert_eq!(s.moves_available, 0);
        assert!(s.piece_selected.is_none());
    }

    /// Click `from`, verify selection + legal moves, click `to`, return result.
    fn step(
        s: &mut GameSession,
        from: &str,
        piece: Piece,
        expected_legal: &[&str],
        to: &str,
    ) -> GamePlay {
        let side_before = s.side_to_move;

        let r1 = s.play(sq(from));
        assert_eq!(s.mode, Mode::moving, "selecting {from} should enter moving mode");
        assert_eq!(s.selected, sq(from));
        assert_eq!(s.piece_selected, Some(piece));
        assert_eq!(s.moves_available, mask(expected_legal), "legal moves from {from}");
        assert_eq!(r1.moves, mask(expected_legal), "snapshot moves from {from}");
        assert_eq!(r1.side_to_move, side_before, "side must not change on selection");

        let r2 = s.play(sq(to));
        assert_idle(s);
        assert_eq!(r2.side_to_move, 1 - side_before, "side must flip after a move");
        assert_eq!(r2.moves, 0);
        r2
    }

    // ------------------------------------------------------------------
    // Main scenario: start FEN, W, B, W, B — verified at every step
    // 1. e4 e5 2. Nf3 Nc6
    // ------------------------------------------------------------------
    #[test]
    fn start_position_four_ply_sequence() {
        let mut s = from_fen(START);
        assert_state(&s, START);
        assert_eq!(s.status, GameStatus::Ongoing);
        assert_idle(&s);

        // --- White: e2-e4 ---
        let r = step(&mut s, "e2", Piece::P, &["e3", "e4"], "e4");
        assert_state(&s, "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert_eq!(r.status, GameStatus::Ongoing);
        assert_eq!(r.bitboards, s.bitboards);

        // --- Black: e7-e5 ---
        let r = step(&mut s, "e7", Piece::P, &["e6", "e5"], "e5");
        assert_state(&s, "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2");
        assert_eq!(r.status, GameStatus::Ongoing);

        // --- White: Ng1-f3 ---
        let r = step(&mut s, "g1", Piece::N, &["e2", "f3", "h3"], "f3");
        assert_state(&s, "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2");
        assert_eq!(r.status, GameStatus::Ongoing);

        // --- Black: Nb8-c6 ---
        let r = step(&mut s, "b8", Piece::N, &["a6", "c6"], "c6");
        assert_state(&s, "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3");
        assert_eq!(r.status, GameStatus::Ongoing);
    }

    // ------------------------------------------------------------------
    // Selection edge cases from the start position
    // ------------------------------------------------------------------
    #[test]
    fn selecting_empty_square_does_nothing() {
        let mut s = from_fen(START);
        let r = s.play(sq("e5"));
        assert_idle(&s);
        assert_eq!(r.moves, 0);
        assert_state(&s, START);
    }

    #[test]
    fn selecting_enemy_piece_does_nothing() {
        let mut s = from_fen(START);
        let r = s.play(sq("e7"));
        assert_idle(&s);
        assert_eq!(r.moves, 0);
        assert_state(&s, START);
    }

    #[test]
    fn selecting_blocked_piece_does_not_enter_moving_mode() {
        let mut s = from_fen(START);
        s.play(sq("a1")); // rook boxed in
        assert_idle(&s);
        s.play(sq("d1")); // queen boxed in
        assert_idle(&s);
    }

    #[test]
    fn multi_bit_selection_is_ignored() {
        let mut s = from_fen(START);
        s.play(sq("e2") | sq("d2"));
        assert_idle(&s);
        assert_state(&s, START);
    }

    #[test]
    fn clicking_illegal_target_deselects() {
        let mut s = from_fen(START);
        s.play(sq("e2"));
        assert_eq!(s.mode, Mode::moving);
        s.play(sq("e5")); // not legal, not own piece
        assert_idle(&s);
        assert_state(&s, START);
    }

    #[test]
    fn clicking_another_own_piece_switches_selection() {
        let mut s = from_fen(START);
        s.play(sq("e2"));
        s.play(sq("d2"));
        assert_eq!(s.mode, Mode::moving);
        assert_eq!(s.selected, sq("d2"));
        assert_eq!(s.piece_selected, Some(Piece::P));
        assert_eq!(s.moves_available, mask(&["d3", "d4"]));
        assert_state(&s, START); // nothing moved
    }

    #[test]
    fn clicking_same_piece_twice_deselects() {
        let mut s = from_fen(START);
        s.play(sq("e2"));
        s.play(sq("e2"));
        // second click on the selected square: not a legal target, and
        // `select != self.selected` is false, so it just deselects.
        assert_idle(&s);
    }

    // ------------------------------------------------------------------
    // Fool's mate: 1. f3 e5 2. g4 Qh4#  (checkmate detection)
    // ------------------------------------------------------------------
    #[test]
    fn fools_mate_sequence_ends_in_checkmate() {
        let mut s = from_fen(START);

        step(&mut s, "f2", Piece::P, &["f3", "f4"], "f3");
        assert_state(&s, "rnbqkbnr/pppppppp/8/8/8/5P2/PPPPP1PP/RNBQKBNR b KQkq - 0 1");

        step(&mut s, "e7", Piece::P, &["e6", "e5"], "e5");
        assert_state(&s, "rnbqkbnr/pppp1ppp/8/4p3/8/5P2/PPPPP1PP/RNBQKBNR w KQkq e6 0 2");

        step(&mut s, "g2", Piece::P, &["g3", "g4"], "g4");
        assert_state(&s, "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2");
        assert_eq!(s.status, GameStatus::Ongoing);

        // Queen d8 can go e7, f6, g5, h4
        let r = step(&mut s, "d8", Piece::Q, &["e7", "f6", "g5", "h4"], "h4");
        assert_state(&s, "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
        assert_eq!(r.status, GameStatus::Checkmate);
        assert_eq!(s.status, GameStatus::Checkmate);

        // No further play allowed after mate
        let before = s.bitboards;
        let r = s.play(sq("e2"));
        assert_eq!(s.bitboards, before);
        assert_eq!(r.status, GameStatus::Checkmate);
        assert_idle(&s);
    }

    // ------------------------------------------------------------------
    // Castling: both sides, rights removal, rook relocation
    // ------------------------------------------------------------------
    #[test]
    fn castling_white_kingside_then_black_queenside() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let mut s = from_fen(fen);

        // White king: 5 normal squares + both castles
        let r = step(
            &mut s, "e1", Piece::K,
            &["d1", "d2", "e2", "f1", "f2", "c1", "g1"], "g1",
        );
        assert_state(&s, "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1");
        assert_eq!(r.status, GameStatus::Ongoing);

        // Black king: white rook on f1 now attacks f8, so O-O is illegal,
        // O-O-O is legal.
        s.play(sq("e8"));
        assert_eq!(s.mode, Mode::moving);
        let m = s.moves_available;
        assert_ne!(m & sq("c8"), 0, "queenside castle must be available");
        assert_eq!(m & sq("g8"), 0, "kingside castle passes through attacked f8");
        assert_ne!(m & sq("d8"), 0);
        assert_ne!(m & sq("d7"), 0);
        assert_ne!(m & sq("e7"), 0);
        s.play(sq("c8"));
        assert_idle(&s);
        assert_state(&s, "2kr3r/8/8/8/8/8/8/R4RK1 w - - 2 2");
    }

    #[test]
    fn castling_blocked_through_attacked_square() {
        // Black rook on f2 attacks f1; king not in check.
        let s = from_fen("4k3/8/8/8/8/8/5r2/R3K2R w KQ - 0 1");
        let m = s.legal_moves(sq("e1"));
        assert_eq!(m & sq("g1"), 0, "cannot castle kingside through f1");
        assert_ne!(m & sq("c1"), 0, "queenside castle is still legal");
    }

    #[test]
    fn castling_not_allowed_while_in_check() {
        let s = from_fen("4r1k1/8/8/8/8/8/8/R3K2R w KQ - 0 1");
        let m = s.legal_moves(sq("e1"));
        assert_eq!(m & sq("g1"), 0);
        assert_eq!(m & sq("c1"), 0);
    }

    #[test]
    fn castling_not_allowed_without_right() {
        let s = from_fen("4k3/8/8/8/8/8/8/R3K2R w - - 0 1");
        let m = s.legal_moves(sq("e1"));
        assert_eq!(m & sq("g1"), 0);
        assert_eq!(m & sq("c1"), 0);
    }

    #[test]
    fn rook_move_removes_only_its_castling_right() {
        let mut s = from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        step(&mut s, "h1", Piece::R, &[
            "g1", "f1", "h2", "h3", "h4", "h5", "h6", "h7", "h8",
        ], "h2");
        assert_state(&s, "r3k2r/8/8/8/8/8/7R/R3K3 b Qkq - 1 1");
    }

    #[test]
    fn capturing_rook_on_home_square_removes_right() {
        let mut s = from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        // Rh1xh8 captures the black king-side rook
        step(&mut s, "h1", Piece::R, &[
            "g1", "f1", "h2", "h3", "h4", "h5", "h6", "h7", "h8",
        ], "h8");
        assert_eq!(s.castling, "Qq");
        assert_eq!(s.halfmove, 0, "capture resets halfmove clock");
    }

    // ------------------------------------------------------------------
    // En passant
    // ------------------------------------------------------------------
    #[test]
    fn en_passant_capture_removes_the_pawn() {
        let mut s = from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
        let r = step(&mut s, "e5", Piece::P, &["e6", "d6"], "d6");
        assert_state(&s, "4k3/8/3P4/8/8/8/8/4K3 b - - 0 1");
        assert_eq!(r.status, GameStatus::Ongoing);
        assert_eq!(s.bitboards[1][P], 0, "black pawn must be removed");
    }

    #[test]
    fn en_passant_target_set_after_double_push_only() {
        let mut s = from_fen(START);
        step(&mut s, "a2", Piece::P, &["a3", "a4"], "a3");
        assert_eq!(s.ep_target, 0, "single push must not set ep target");
        step(&mut s, "a7", Piece::P, &["a6", "a5"], "a5");
        assert_eq!(s.ep_target, sq("a6"), "black double push sets ep target behind pawn");
    }

    #[test]
    fn en_passant_expires_after_one_move() {
        let mut s = from_fen("4k3/3p4/8/4P3/8/8/8/4K3 b - - 0 1");
        step(&mut s, "d7", Piece::P, &["d6", "d5"], "d5");
        assert_eq!(s.ep_target, sq("d6"));
        // White declines to capture and plays a king move
        s.play(sq("e1"));
        let to = if s.moves_available & sq("e2") != 0 { "e2" } else { "d1" };
        s.play(sq(to));
        assert_eq!(s.ep_target, 0);
    }

    // ------------------------------------------------------------------
    // Pins and check evasion
    // ------------------------------------------------------------------
    #[test]
    fn pinned_knight_has_no_legal_moves_and_cannot_be_selected() {
        let mut s = from_fen("4r1k1/8/8/8/8/8/4N3/4K3 w - - 0 1");
        assert_eq!(s.legal_moves(sq("e2")), 0);
        s.play(sq("e2"));
        assert_idle(&s);
    }

    #[test]
    fn piece_in_check_may_only_block() {
        // Black rook e8 checks king e1; the bishop can only block on e3.
        let s = from_fen("4r1k1/8/8/8/8/8/3B4/4K3 w - - 0 1");
        assert_eq!(s.legal_moves(sq("d2")), mask(&["e3"]));
    }

    #[test]
    fn pinned_rook_can_only_move_along_pin_line() {
        let s = from_fen("4r1k1/8/8/8/8/8/4R3/4K3 w - - 0 1");
        let m = s.legal_moves(sq("e2"));
        assert_eq!(m, mask(&["e3", "e4", "e5", "e6", "e7", "e8"]));
    }

    #[test]
    fn legal_moves_empty_for_wrong_side_or_empty_square() {
        let s = from_fen(START);
        assert_eq!(s.legal_moves(sq("e7")), 0, "black piece, white to move");
        assert_eq!(s.legal_moves(sq("e4")), 0, "empty square");
        assert_eq!(s.legal_moves(sq("e2") | sq("d2")), 0, "multi-bit input");
    }

    // ------------------------------------------------------------------
    // Promotion
    // ------------------------------------------------------------------
    #[test]
    fn promotion_to_knight() {
        let mut s = from_fen("8/P6k/8/8/8/8/8/K7 w - - 0 1");
        s.play(sq("a7"));
        assert_eq!(s.moves_available, mask(&["a8"]));
        s.play_with_promotion(sq("a8"), Piece::N);
        assert_state(&s, "N7/7k/8/8/8/8/8/K7 b - - 0 1");
    }

    #[test]
    fn promotion_defaults_to_queen() {
        let mut s = from_fen("8/P6k/8/8/8/8/8/K7 w - - 0 1");
        s.play(sq("a7"));
        s.play(sq("a8"));
        assert_state(&s, "Q7/7k/8/8/8/8/8/K7 b - - 0 1");
    }

    #[test]
    fn promotion_with_capture() {
        let mut s = from_fen("1r5k/P7/8/8/8/8/8/K7 w - - 0 1");
        s.play(sq("a7"));
        assert_eq!(s.moves_available, mask(&["a8", "b8"]));
        s.play_with_promotion(sq("b8"), Piece::R);
        assert_eq!(s.bitboards[1][R], 0, "captured rook removed");
        assert_ne!(s.bitboards[0][R] & sq("b8"), 0, "promoted rook present");
        assert_eq!(s.bitboards[0][P], 0);
    }

    // ------------------------------------------------------------------
    // Stalemate / fifty-move / clocks
    // ------------------------------------------------------------------
    #[test]
    fn queen_move_creates_stalemate() {
        let mut s = from_fen("7k/5K2/8/8/8/8/8/6Q1 w - - 0 1");
        s.play(sq("g1"));
        assert_ne!(s.moves_available & sq("g6"), 0);
        let r = s.play(sq("g6"));
        assert_eq!(r.status, GameStatus::Stalemate, "must be stalemate, not checkmate");
        assert_eq!(s.status, GameStatus::Stalemate);
        assert_eq!(s.side_to_move, 1);
    }

    #[test]
    fn fifty_move_rule_triggers_at_halfmove_100() {
        let mut s = from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 99 60");
        s.play(sq("a1"));
        let r = s.play(sq("a2"));
        assert_eq!(s.halfmove, 100);
        assert_eq!(r.status, GameStatus::DrawFiftyMove);
    }

    #[test]
    fn check_status_is_reported() {
        let mut s = from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1");
        s.play(sq("a1"));
        let r = s.play(sq("a8"));
        assert_eq!(r.status, GameStatus::Check);
    }

    #[test]
    fn halfmove_resets_on_pawn_move_and_capture_and_increments_otherwise() {
        let mut s = from_fen("4k3/8/8/8/8/8/P7/R3K3 w - - 5 10");
        step(&mut s, "a1", Piece::R, &["b1", "c1", "d1"], "d1");
        assert_eq!(s.halfmove, 6);
        assert_eq!(s.fullmove, 10, "fullmove only increments after black");
    }

    #[test]
    fn fullmove_increments_only_after_black_moves() {
        let mut s = from_fen(START);
        step(&mut s, "e2", Piece::P, &["e3", "e4"], "e4");
        assert_eq!(s.fullmove, 1);
        step(&mut s, "e7", Piece::P, &["e6", "e5"], "e5");
        assert_eq!(s.fullmove, 2);
    }

    #[test]
    fn capture_removes_victim_and_resets_halfmove() {
        let mut s = from_fen("4k3/8/8/3p4/4P3/8/8/4K3 w - - 7 1");
        step(&mut s, "e4", Piece::P, &["e5", "d5"], "d5");
        assert_state(&s, "4k3/8/8/3P4/8/8/8/4K3 b - - 0 1");
    }
}