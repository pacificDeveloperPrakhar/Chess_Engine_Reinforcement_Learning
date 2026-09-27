// Integration test: put this in tests/moves_generation.rs.
// Requires src/lib.rs to expose `pub mod generator;` and `pub mod structures;`
// (and `pub mod annotations;` inside structures).
//
// Assumptions (change the helpers if yours differ):
//   * `Piece` and `PieceColor` derive Clone + Copy
//   * PieceColor::from(char) accepts 'w' / 'b'  (see WHITE_SIDE / BLACK_SIDE)
//   * bit layout: a1 = bit 0, b1 = bit 1 ... h1 = bit 7, a2 = bit 8 ... h8 = bit 63,
//     white pawns move towards higher bits
//   * `l_squares` etc. return squares that are empty or hold an enemy piece
//
// Tests describe the CORRECT chess behaviour. Some will fail on the current
// code because of real bugs (see the notes in the accompanying message).

#[cfg(test)]
mod tests {
    use chess_engine::generator::{
        generate_moves_for_king, generating_moves_with_king_safety, is_king_checked,
        possible_moves, rank_file_generator,
    };
    use chess_engine::structures::annotations::{AttackType, Piece, PieceColor};

    type Boards = [[u64; 7]; 2];

    const WHITE_SIDE: char = 'w';
    const BLACK_SIDE: char = 'b';

    // ------------------------------------------------------------------ helpers

    /// "e4" -> single-bit u64 (a1 = bit 0).
    fn sq(name: &str) -> u64 {
        let bytes = name.as_bytes();
        let file = (bytes[0] - b'a') as u32;
        let rank = (bytes[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
    }

    fn sqs(names: &[&str]) -> u64 {
        names.iter().fold(0u64, |acc, n| acc | sq(n))
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

    fn w(piece: Piece, square: &str) -> (PieceColor, Piece, &str) {
        (PieceColor::W, piece, square)
    }

    fn b(piece: Piece, square: &str) -> (PieceColor, Piece, &str) {
        (PieceColor::B, piece, square)
    }

    fn board(pieces: &[(PieceColor, Piece, &str)]) -> Boards {
        let mut bb = empty_board();
        for &(color, piece, square) in pieces {
            put(&mut bb, color, piece, square);
        }
        bb
    }

    fn show(bits: u64) -> String {
        let mut s = String::new();
        for rank in (0..8u32).rev() {
            for file in 0..8u32 {
                s.push(if bits & (1u64 << (rank * 8 + file)) != 0 { 'X' } else { '.' });
            }
            s.push('\n');
        }
        s
    }

    fn assert_squares_ctx(ctx: &str, actual: u64, expected: u64) {
        assert_eq!(
            actual, expected,
            "{ctx}\nactual:\n{}\nexpected:\n{}",
            show(actual),
            show(expected)
        );
    }

    fn assert_squares(actual: u64, expected: u64) {
        assert_squares_ctx("", actual, expected);
    }

    fn moves(bb: Boards, from: &str) -> u64 {
        possible_moves(bb, sq(from))
    }

    fn king_moves(bb: Boards, from: &str) -> u64 {
        generate_moves_for_king(bb, sq(from))
    }

    fn safe_moves(bb: Boards, from: &str) -> u64 {
        generating_moves_with_king_safety(bb, sq(from))
    }

    const ROOK_D4: [&str; 14] = [
        "a4", "b4", "c4", "e4", "f4", "g4", "h4", "d1", "d2", "d3", "d5", "d6", "d7", "d8",
    ];
    const BISHOP_D4: [&str; 13] = [
        "a1", "b2", "c3", "e5", "f6", "g7", "h8", "a7", "b6", "c5", "e3", "f2", "g1",
    ];

    // ------------------------------------------------------- rank_file_generator

    #[test]
    fn rank_file_generator_maps_each_square_to_a_unique_bit() {
        let mut seen = 0u64;
        for rank in 0..8usize {
            for file in 0..8usize {
                let bit = rank_file_generator(rank, file);
                assert_eq!(bit.count_ones(), 1, "rank {rank} file {file} gave {bit:#x}");
                assert_eq!(seen & bit, 0, "rank {rank} file {file} duplicates an earlier square");
                seen |= bit;
            }
        }
        assert_eq!(seen, u64::MAX);
    }

    // ------------------------------------------------------------ possible_moves

    #[test]
    fn possible_moves_on_empty_square_is_zero() {
        let bb = board(&[w(Piece::R, "a1")]);
        assert_eq!(moves(bb, "d4"), 0);
    }

    #[test]
    fn knight_in_the_centre_has_eight_moves() {
        let bb = board(&[w(Piece::N, "e4")]);
        assert_squares(
            moves(bb, "e4"),
            sqs(&["c3", "c5", "d2", "d6", "f2", "f6", "g3", "g5"]),
        );
    }

    #[test]
    fn black_knight_in_the_centre_has_eight_moves() {
        let bb = board(&[b(Piece::N, "e4")]);
        assert_squares(
            moves(bb, "e4"),
            sqs(&["c3", "c5", "d2", "d6", "f2", "f6", "g3", "g5"]),
        );
    }

    #[test]
    fn knight_moves_do_not_wrap_around_the_edges() {
        let cases: [(&str, &[&str]); 5] = [
            ("a1", &["b3", "c2"]),
            ("h8", &["f7", "g6"]),
            ("a4", &["b2", "b6", "c3", "c5"]),
            ("h5", &["g3", "g7", "f4", "f6"]),
            ("b1", &["a3", "c3", "d2"]),
        ];
        for (from, targets) in cases {
            let bb = board(&[w(Piece::N, from)]);
            assert_squares_ctx(&format!("knight on {from}"), moves(bb, from), sqs(targets));
        }
    }

    #[test]
    fn knight_cannot_land_on_own_piece_but_can_capture() {
        let bb = board(&[w(Piece::N, "e4"), w(Piece::P, "c3"), b(Piece::P, "f6")]);
        assert_squares(
            moves(bb, "e4"),
            sqs(&["c5", "d2", "d6", "f2", "f6", "g3", "g5"]),
        );
    }

    #[test]
    fn rook_on_empty_board() {
        let bb = board(&[w(Piece::R, "d4")]);
        assert_squares(moves(bb, "d4"), sqs(&ROOK_D4));
    }

    #[test]
    fn rook_in_the_corner_has_fourteen_moves() {
        let bb = board(&[w(Piece::R, "a1")]);
        let m = moves(bb, "a1");
        assert_eq!(m.count_ones(), 14);
        assert_squares(
            m,
            sqs(&[
                "a2", "a3", "a4", "a5", "a6", "a7", "a8", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
            ]),
        );
    }

    #[test]
    fn rook_is_blocked_by_own_piece() {
        let bb = board(&[w(Piece::R, "a1"), w(Piece::N, "a4")]);
        assert_squares(
            moves(bb, "a1"),
            sqs(&["a2", "a3", "b1", "c1", "d1", "e1", "f1", "g1", "h1"]),
        );
    }

    #[test]
    fn rook_captures_first_enemy_piece_and_stops() {
        let bb = board(&[w(Piece::R, "a1"), b(Piece::P, "a4"), b(Piece::P, "a6")]);
        assert_squares(
            moves(bb, "a1"),
            sqs(&["a2", "a3", "a4", "b1", "c1", "d1", "e1", "f1", "g1", "h1"]),
        );
    }

    #[test]
    fn both_colours_can_be_selected_on_the_same_board() {
        let bb = board(&[w(Piece::R, "a1"), b(Piece::R, "a8")]);
        assert_squares_ctx(
            "white rook",
            moves(bb, "a1"),
            sqs(&[
                "a2", "a3", "a4", "a5", "a6", "a7", "a8", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
            ]),
        );
        assert_squares_ctx(
            "black rook",
            moves(bb, "a8"),
            sqs(&[
                "a7", "a6", "a5", "a4", "a3", "a2", "a1", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
            ]),
        );
    }

    #[test]
    fn bishop_on_empty_board() {
        let bb = board(&[w(Piece::B, "d4")]);
        assert_squares(moves(bb, "d4"), sqs(&BISHOP_D4));
    }

    #[test]
    fn bishop_in_the_corner() {
        let bb = board(&[b(Piece::B, "h1")]);
        assert_squares(
            moves(bb, "h1"),
            sqs(&["g2", "f3", "e4", "d5", "c6", "b7", "a8"]),
        );
    }

    #[test]
    fn bishop_captures_and_stops_at_enemy_piece() {
        let bb = board(&[w(Piece::B, "d4"), b(Piece::P, "f6")]);
        assert_squares(
            moves(bb, "d4"),
            sqs(&["a1", "b2", "c3", "e5", "f6", "a7", "b6", "c5", "e3", "f2", "g1"]),
        );
    }

    #[test]
    fn bishop_is_blocked_by_own_piece() {
        let bb = board(&[w(Piece::B, "d4"), w(Piece::P, "f6")]);
        assert_squares(
            moves(bb, "d4"),
            sqs(&["a1", "b2", "c3", "e5", "a7", "b6", "c5", "e3", "f2", "g1"]),
        );
    }

    #[test]
    fn queen_is_rook_plus_bishop() {
        let bb = board(&[w(Piece::Q, "d4")]);
        let m = moves(bb, "d4");
        assert_eq!(m.count_ones(), 27);
        assert_squares(m, sqs(&ROOK_D4) | sqs(&BISHOP_D4));
    }

    #[test]
    fn king_in_the_centre_has_eight_moves() {
        let bb = board(&[w(Piece::K, "e4")]);
        assert_squares(
            moves(bb, "e4"),
            sqs(&["d3", "e3", "f3", "d4", "f4", "d5", "e5", "f5"]),
        );
    }

    #[test]
    fn king_in_the_corner_has_three_moves() {
        let bb = board(&[w(Piece::K, "a1")]);
        assert_squares(moves(bb, "a1"), sqs(&["a2", "b1", "b2"]));
    }

    #[test]
    fn king_boxed_in_by_own_pieces_has_no_moves() {
        let bb = board(&[
            w(Piece::K, "a1"),
            w(Piece::P, "a2"),
            w(Piece::P, "b2"),
            w(Piece::N, "b1"),
        ]);
        assert_eq!(moves(bb, "a1"), 0);
    }

    #[test]
    fn white_pawn_on_start_rank_can_push_one_or_two() {
        let bb = board(&[w(Piece::P, "e2")]);
        assert_squares(moves(bb, "e2"), sqs(&["e3", "e4"]));
    }

    #[test]
    fn white_pawn_off_start_rank_pushes_one() {
        let bb = board(&[w(Piece::P, "e4")]);
        assert_squares(moves(bb, "e4"), sqs(&["e5"]));
    }

    #[test]
    fn black_pawns_move_towards_lower_ranks() {
        let start = board(&[b(Piece::P, "e7")]);
        assert_squares_ctx("start rank", moves(start, "e7"), sqs(&["e6", "e5"]));
        let later = board(&[b(Piece::P, "e5")]);
        assert_squares_ctx("later", moves(later, "e5"), sqs(&["e4"]));
    }

    #[test]
    fn pawn_blocked_directly_ahead_cannot_move() {
        let bb = board(&[w(Piece::P, "e2"), b(Piece::N, "e3")]);
        assert_eq!(moves(bb, "e2"), 0);
    }

    #[test]
    fn pawn_double_push_is_blocked_on_the_second_square() {
        let bb = board(&[w(Piece::P, "e2"), b(Piece::N, "e4")]);
        assert_squares(moves(bb, "e2"), sqs(&["e3"]));
    }

    #[test]
    fn pawn_does_not_capture_straight_ahead() {
        let bb = board(&[w(Piece::P, "e4"), b(Piece::P, "e5")]);
        assert_eq!(moves(bb, "e4"), 0);
    }

    #[test]
    fn white_pawn_captures_diagonally() {
        let bb = board(&[w(Piece::P, "e4"), b(Piece::P, "d5"), b(Piece::P, "f5")]);
        assert_squares(moves(bb, "e4"), sqs(&["e5", "d5", "f5"]));
    }

    #[test]
    fn black_pawn_captures_diagonally() {
        let bb = board(&[b(Piece::P, "e5"), w(Piece::P, "d4"), w(Piece::P, "f4")]);
        assert_squares(moves(bb, "e5"), sqs(&["e4", "d4", "f4"]));
    }

    #[test]
    fn pawn_does_not_capture_own_piece() {
        let bb = board(&[w(Piece::P, "e4"), w(Piece::P, "d5")]);
        assert_squares(moves(bb, "e4"), sqs(&["e5"]));
    }

    #[test]
    fn pawn_captures_do_not_wrap_around_the_board() {
        // a2 pawn must not "capture" on h2, h2 pawn must not "capture" on a4.
        let a_file = board(&[w(Piece::P, "a2"), b(Piece::P, "h2")]);
        assert_squares_ctx("a2", moves(a_file, "a2"), sqs(&["a3", "a4"]));
        let h_file = board(&[w(Piece::P, "h2"), b(Piece::N, "a4")]);
        assert_squares_ctx("h2", moves(h_file, "h2"), sqs(&["h3", "h4"]));
    }

    // ------------------------------------------------------------ is_king_checked

    #[test]
    fn quiet_board_is_not_check() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "a8")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_eq!(attackers, 0);
        assert!(kinds.is_empty());
    }

    #[test]
    fn rook_on_open_file_gives_check() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "e8")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sq("e8"));
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], AttackType::horizontal_vertical));
    }

    #[test]
    fn rook_on_open_rank_gives_check() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "a1")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sq("a1"));
        assert!(matches!(kinds[0], AttackType::horizontal_vertical));
    }

    #[test]
    fn rook_check_is_blocked_by_own_pawn() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::P, "e2"), b(Piece::R, "e8")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_eq!(attackers, 0);
        assert!(kinds.is_empty());
    }

    #[test]
    fn rook_check_is_blocked_by_enemy_piece() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::P, "e5"), b(Piece::R, "e8")]);
        let (attackers, _) = is_king_checked(bb, WHITE_SIDE);
        assert_eq!(attackers, 0);
    }

    #[test]
    fn own_rook_never_checks_own_king() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::R, "e8")]);
        let (attackers, _) = is_king_checked(bb, WHITE_SIDE);
        assert_eq!(attackers, 0);
    }

    #[test]
    fn bishop_on_diagonal_gives_check() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::B, "a5")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sq("a5"));
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], AttackType::diagnol));
    }

    #[test]
    fn bishop_does_not_check_along_a_file() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::B, "e8")]);
        assert_eq!(is_king_checked(bb, WHITE_SIDE).0, 0);
    }

    #[test]
    fn rook_does_not_check_along_a_diagonal() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "a5")]);
        assert_eq!(is_king_checked(bb, WHITE_SIDE).0, 0);
    }

    #[test]
    fn queen_checks_diagonally_and_straight() {
        let diag = board(&[w(Piece::K, "e1"), b(Piece::Q, "h4")]);
        let (attackers, kinds) = is_king_checked(diag, WHITE_SIDE);
        assert_squares_ctx("diagonal", attackers, sq("h4"));
        assert!(matches!(kinds[0], AttackType::diagnol));

        let straight = board(&[w(Piece::K, "e1"), b(Piece::Q, "e5")]);
        let (attackers, kinds) = is_king_checked(straight, WHITE_SIDE);
        assert_squares_ctx("straight", attackers, sq("e5"));
        assert!(matches!(kinds[0], AttackType::horizontal_vertical));
    }

    #[test]
    fn knight_gives_check_and_cannot_be_blocked() {
        let bb = board(&[
            w(Piece::K, "e1"),
            w(Piece::P, "d2"),
            w(Piece::P, "e2"),
            w(Piece::P, "f2"),
            b(Piece::N, "f3"),
        ]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sq("f3"));
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], AttackType::l_shape));
    }

    #[test]
    fn black_pawn_diagonally_in_front_checks_white_king() {
        for pawn in ["d5", "f5"] {
            let bb = board(&[w(Piece::K, "e4"), b(Piece::P, pawn)]);
            let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
            assert_squares_ctx(pawn, attackers, sq(pawn));
            assert!(matches!(kinds[0], AttackType::pawn), "{pawn}");
        }
    }

    #[test]
    fn pawn_straight_ahead_or_behind_does_not_check() {
        let ahead = board(&[w(Piece::K, "e4"), b(Piece::P, "e5")]);
        assert_eq!(is_king_checked(ahead, WHITE_SIDE).0, 0, "pawn ahead");
        let behind = board(&[w(Piece::K, "e4"), b(Piece::P, "d3")]);
        assert_eq!(is_king_checked(behind, WHITE_SIDE).0, 0, "pawn behind");
    }

    #[test]
    fn white_pawn_checks_black_king_from_below() {
        for pawn in ["d4", "f4"] {
            let bb = board(&[b(Piece::K, "e5"), w(Piece::P, pawn)]);
            let (attackers, kinds) = is_king_checked(bb, BLACK_SIDE);
            assert_squares_ctx(pawn, attackers, sq(pawn));
            assert!(matches!(kinds[0], AttackType::pawn), "{pawn}");
        }
        let behind = board(&[b(Piece::K, "e5"), w(Piece::P, "d6")]);
        assert_eq!(is_king_checked(behind, BLACK_SIDE).0, 0, "white pawn above black king");
    }

    #[test]
    fn rook_checks_black_king() {
        let bb = board(&[b(Piece::K, "e8"), w(Piece::R, "e1")]);
        let (attackers, kinds) = is_king_checked(bb, BLACK_SIDE);
        assert_squares(attackers, sq("e1"));
        assert!(matches!(kinds[0], AttackType::horizontal_vertical));
    }

    #[test]
    fn adjacent_enemy_king_is_reported_as_attacker() {
        let bb = board(&[w(Piece::K, "e4"), b(Piece::K, "e5")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sq("e5"));
        assert!(matches!(kinds[0], AttackType::square));
    }

    #[test]
    fn rook_plus_knight_is_a_double_check() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "e8"), b(Piece::N, "f3")]);
        let (attackers, kinds) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sqs(&["e8", "f3"]));
        assert_eq!(kinds.len(), 2);
    }

    #[test]
    fn two_rooks_are_both_reported_as_attackers() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "e8"), b(Piece::R, "a1")]);
        let (attackers, _) = is_king_checked(bb, WHITE_SIDE);
        assert_squares(attackers, sqs(&["e8", "a1"]));
    }

    // ------------------------------------------------------ generate_moves_for_king

    #[test]
    fn lone_king_moves() {
        let corner = board(&[w(Piece::K, "a1")]);
        assert_squares_ctx("corner", king_moves(corner, "a1"), sqs(&["a2", "b1", "b2"]));

        let centre = board(&[w(Piece::K, "e4")]);
        assert_squares_ctx(
            "centre",
            king_moves(centre, "e4"),
            sqs(&["d3", "e3", "f3", "d4", "f4", "d5", "e5", "f5"]),
        );
    }

    #[test]
    fn selecting_a_non_king_returns_zero() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::R, "a1")]);
        assert_eq!(king_moves(bb, "a1"), 0);
        assert_eq!(king_moves(bb, "d4"), 0, "empty square");
    }

    #[test]
    fn king_boxed_in_by_own_pieces_has_no_legal_moves() {
        let bb = board(&[
            w(Piece::K, "a1"),
            w(Piece::P, "a2"),
            w(Piece::P, "b2"),
            w(Piece::N, "b1"),
        ]);
        assert_eq!(king_moves(bb, "a1"), 0);
    }

    #[test]
    fn king_cannot_walk_onto_a_rook_file() {
        let bb = board(&[w(Piece::K, "e4"), b(Piece::R, "d8")]);
        assert_squares(king_moves(bb, "e4"), sqs(&["e3", "e5", "f3", "f4", "f5"]));
    }

    #[test]
    fn king_cannot_step_back_along_the_checking_rank() {
        // rook a1 checks along rank 1; f1 is still attacked once the king moves off e1
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "a1")]);
        assert_squares(king_moves(bb, "e1"), sqs(&["d2", "e2", "f2"]));
    }

    #[test]
    fn king_cannot_step_back_along_the_checking_file() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::R, "e8")]);
        assert_squares(king_moves(bb, "e1"), sqs(&["d1", "d2", "f1", "f2"]));
    }

    #[test]
    fn king_cannot_walk_onto_a_bishop_diagonal() {
        let bb = board(&[w(Piece::K, "e1"), b(Piece::B, "a6")]);
        assert_squares(king_moves(bb, "e1"), sqs(&["d1", "d2", "f2"]));
    }

    #[test]
    fn king_avoids_knight_squares() {
        let bb = board(&[w(Piece::K, "e4"), b(Piece::N, "c6")]);
        assert_squares(
            king_moves(bb, "e4"),
            sqs(&["d3", "e3", "f3", "f4", "d5", "f5"]),
        );
    }

    #[test]
    fn white_king_avoids_black_pawn_attacks_but_not_its_push_square() {
        // black pawn d6 attacks c5 and e5, but not d5
        let bb = board(&[w(Piece::K, "e4"), b(Piece::P, "d6")]);
        assert_squares(
            king_moves(bb, "e4"),
            sqs(&["d3", "e3", "f3", "d4", "f4", "d5", "f5"]),
        );
    }

    #[test]
    fn black_king_avoids_white_pawn_attacks() {
        // white pawn d3 attacks c4 and e4
        let bb = board(&[b(Piece::K, "e5"), w(Piece::P, "d3")]);
        assert_squares(
            king_moves(bb, "e5"),
            sqs(&["d4", "f4", "d5", "f5", "d6", "e6", "f6"]),
        );
    }

    #[test]
    fn pawn_attacks_do_not_wrap_around_the_board() {
        // black pawn a4 attacks only b3, so g2 keeps all eight squares
        let black_pawn = board(&[w(Piece::K, "g2"), b(Piece::P, "a4")]);
        assert_squares_ctx(
            "black pawn a4",
            king_moves(black_pawn, "g2"),
            sqs(&["f1", "f2", "f3", "g1", "g3", "h1", "h2", "h3"]),
        );
        // white pawn h2 attacks only g3, so b4 keeps all eight squares
        let white_pawn = board(&[b(Piece::K, "b4"), w(Piece::P, "h2")]);
        assert_squares_ctx(
            "white pawn h2",
            king_moves(white_pawn, "b4"),
            sqs(&["a3", "a4", "a5", "b3", "b5", "c3", "c4", "c5"]),
        );
    }

    #[test]
    fn kings_cannot_stand_next_to_each_other() {
        let bb = board(&[w(Piece::K, "e4"), b(Piece::K, "e6")]);
        assert_squares(king_moves(bb, "e4"), sqs(&["d3", "e3", "f3", "d4", "f4"]));
    }

    #[test]
    fn king_can_capture_an_undefended_checker() {
        // e3 stays attacked through the king's current square
        let bb = board(&[w(Piece::K, "e4"), b(Piece::R, "e5")]);
        assert_squares(king_moves(bb, "e4"), sqs(&["d3", "f3", "d4", "f4", "e5"]));
    }

    #[test]
    fn king_cannot_capture_a_defended_piece() {
        // bishop e5 is defended by the rook on a5
        let bb = board(&[w(Piece::K, "e4"), b(Piece::B, "e5"), b(Piece::R, "a5")]);
        assert_squares(king_moves(bb, "e4"), sqs(&["d3", "e3", "f3", "f5"]));
    }

    #[test]
    fn black_king_x_ray_through_itself() {
        let bb = board(&[b(Piece::K, "e8"), w(Piece::R, "a8")]);
        assert_squares(king_moves(bb, "e8"), sqs(&["d7", "e7", "f7"]));
    }

    #[test]
    fn king_with_every_square_covered_has_no_moves() {
        let bb = board(&[w(Piece::K, "h1"), b(Piece::Q, "g3")]);
        assert_eq!(king_moves(bb, "h1"), 0);
    }

    // ------------------------------------- generating_moves_with_king_safety

    #[test]
    fn free_piece_keeps_all_its_moves_when_king_is_safe() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::N, "b1"), b(Piece::R, "a8")]);
        assert_squares(safe_moves(bb, "b1"), sqs(&["a3", "c3", "d2"]));
    }

    #[test]
    fn black_piece_keeps_all_its_moves_when_king_is_safe() {
        let bb = board(&[b(Piece::K, "e8"), b(Piece::N, "b8"), w(Piece::R, "a1")]);
        assert_squares(safe_moves(bb, "b8"), sqs(&["a6", "c6", "d7"]));
    }

    #[test]
    fn pinned_knight_cannot_move() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::N, "e2"), b(Piece::R, "e8")]);
        assert_eq!(safe_moves(bb, "e2"), 0);
    }

    #[test]
    fn pinned_rook_moves_along_the_pin_and_can_capture_the_pinner() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::R, "e4"), b(Piece::R, "e8")]);
        assert_squares(
            safe_moves(bb, "e4"),
            sqs(&["e2", "e3", "e5", "e6", "e7", "e8"]),
        );
    }

    #[test]
    fn pinned_bishop_moves_along_the_pin_and_can_capture_the_pinner() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::B, "d2"), b(Piece::B, "a5")]);
        assert_squares(safe_moves(bb, "d2"), sqs(&["c3", "b4", "a5"]));
    }

    #[test]
    fn bishop_pinned_along_a_rank_cannot_move() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::B, "c1"), b(Piece::R, "a1")]);
        assert_eq!(safe_moves(bb, "c1"), 0);
    }

    #[test]
    fn pinned_pawn_can_still_advance_along_the_pin_file() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::P, "e2"), b(Piece::R, "e8")]);
        assert_squares(safe_moves(bb, "e2"), sqs(&["e3", "e4"]));
    }

    #[test]
    fn pawn_pinned_on_a_diagonal_cannot_push() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::P, "d2"), b(Piece::B, "a5")]);
        assert_eq!(safe_moves(bb, "d2"), 0);
    }

    #[test]
    fn in_check_a_piece_may_only_block() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::B, "d2"), b(Piece::R, "e8")]);
        assert_squares(safe_moves(bb, "d2"), sqs(&["e3"]));
    }

    #[test]
    fn in_check_a_piece_that_cannot_block_or_capture_has_no_moves() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::N, "b1"), b(Piece::R, "e8")]);
        assert_eq!(safe_moves(bb, "b1"), 0);
    }

    #[test]
    fn in_check_a_piece_may_capture_the_checking_knight() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::B, "h5"), b(Piece::N, "f3")]);
        assert_squares(safe_moves(bb, "h5"), sqs(&["f3"]));
    }

    #[test]
    fn in_check_a_piece_may_capture_the_checking_pawn() {
        let bb = board(&[w(Piece::K, "e4"), w(Piece::N, "c3"), b(Piece::P, "d5")]);
        assert_squares(safe_moves(bb, "c3"), sqs(&["d5"]));
    }

    #[test]
    fn in_check_a_piece_may_capture_the_checking_rook() {
        let bb = board(&[w(Piece::K, "e1"), w(Piece::R, "a8"), b(Piece::R, "e8")]);
        assert_squares(safe_moves(bb, "a8"), sqs(&["e8"]));
    }

    #[test]
    fn double_check_leaves_no_moves_for_other_pieces() {
        // the bishop could block the rook on e3, but the knight still checks
        let bb = board(&[
            w(Piece::K, "e1"),
            w(Piece::B, "d2"),
            b(Piece::R, "e8"),
            b(Piece::N, "f3"),
        ]);
        assert_eq!(safe_moves(bb, "d2"), 0);
    }

    #[test]
    fn double_check_by_two_rooks_leaves_no_moves_for_other_pieces() {
        // the knight can block either rook (b1/d1 or e2) but never both
        let bb = board(&[
            w(Piece::K, "e1"),
            w(Piece::N, "c3"),
            b(Piece::R, "a1"),
            b(Piece::R, "e8"),
        ]);
        assert_eq!(safe_moves(bb, "c3"), 0);
    }
}