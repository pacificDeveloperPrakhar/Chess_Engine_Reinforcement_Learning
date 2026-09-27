// tests/node_min_max.rs
//
// Adjust this import to your actual module path for Node_Min_Max.
#[cfg(test)]
mod tests {
    use chess_engine::structures::annotations::{Piece, PieceColor};
	use chess_engine::structures::state::*;

    type Boards = [[u64; 7]; 2];

    fn sq(name: &str) -> u64 {
        let bytes = name.as_bytes();
        let file = (bytes[0] - b'a') as u32;
        let rank = (bytes[1] - b'1') as u32;
        1u64 << (rank * 8 + file)
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

    // ---------------------------------------------------------------------
    // Bug 2: moving a piece must update its own side's aggregate `A` board
    // ---------------------------------------------------------------------
    #[test]
    fn moving_a_piece_updates_the_aggregate_occupancy_board() {
        // lone white knight on b1, kings far apart and safe, nothing else on board
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::N, "b1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);

        let mut node = Node_Min_Max::new_state(bb);
        node.role = 1; // so player = 1 - 1 = 0 = White to move
        node.calculate_children();

        // find a child where the knight moved from b1 to c3
        let target = sq("c3");
        let origin = sq("b1");

        let child = node
            .children
            .iter()
            .find(|c| c.bitboards[0][Piece::N as usize] & target != 0)
            .expect("expected a child with the knight on c3");

        // piece-specific board reflects the move
        assert_eq!(child.bitboards[0][Piece::N as usize] & origin, 0, "knight board still has b1 set");
        assert_ne!(child.bitboards[0][Piece::N as usize] & target, 0, "knight board missing c3");

        // aggregate board must be kept in sync (this is the bug being tested)
        assert_eq!(child.bitboards[0][Piece::A as usize] & origin, 0, "A board still has stale bit at b1");
        assert_ne!(child.bitboards[0][Piece::A as usize] & target, 0, "A board missing new bit at c3");

        // A board should still also contain the untouched king
        assert_ne!(child.bitboards[0][Piece::A as usize] & sq("e1"), 0, "A board lost the king");
    }

    // ---------------------------------------------------------------------
    // Bug 1: Piece::A must never be walked as if it were a movable piece
    // ---------------------------------------------------------------------
    #[test]
    fn children_count_matches_real_pieces_only_not_the_aggregate_board() {
        // white: king e1 (5 free moves off the back rank) + knight b1 (3 free moves)
        // black: lone king far away, uninvolved
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::N, "b1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);

        let mut node = Node_Min_Max::new_state(bb);
        node.role = 1; // player = 0 = White
        node.calculate_children();

        // king e1 with nothing nearby: d1,d2,e2,f1,f2 = 5 moves
        // knight b1: a3,c3,d2 = 3 moves
        // total = 8, if Piece::A were (incorrectly) also iterated as a "piece"
        // sitting on every occupied square, this count would be inflated / corrupted.
        assert_eq!(node.children.len(), 8, "unexpected number of children: {}", node.children.len());

        // every child must have exactly the same number of white pieces as the parent (2):
        // if Piece::A were treated as a piece and moved via new_bitboards[player][6],
        // this would corrupt piece counts.
        for child in &node.children {
            let white_pieces = child.bitboards[0][Piece::A as usize].count_ones();
            assert_eq!(white_pieces, 2, "white piece count corrupted in a child board");
        }
    }

    // ---------------------------------------------------------------------
    // Bug 3: king must not be allowed to move into an attacked square
    // ---------------------------------------------------------------------
    #[test]
    fn king_does_not_move_into_a_square_controlled_by_the_enemy() {
        // white king e4, not currently in check.
        // black rook on d8 controls the whole d-file, so d3, d4, d5 are all unsafe.
        let bb = board(&[
            (PieceColor::W, Piece::K, "e4"),
            (PieceColor::B, Piece::R, "d8"),
            (PieceColor::B, Piece::K, "h8"),
        ]);

        let mut node = Node_Min_Max::new_state(bb);
        node.role = 1; // player = 0 = White
        node.calculate_children();

        // none of the resulting child boards should have the white king anywhere on the d-file
        for child in &node.children {
            let king_bb = child.bitboards[0][Piece::K as usize];
            for file_d_square in ["d1", "d2", "d3", "d4", "d5", "d6", "d7", "d8"] {
                assert_eq!(
                    king_bb & sq(file_d_square),
                    0,
                    "king illegally moved to {file_d_square}, which is attacked by the rook"
                );
            }
        }

        // and the king should have exactly the 5 legal squares (matches king_cannot_walk_onto_a_rook_file)
        assert_eq!(node.children.len(), 5, "expected exactly 5 legal king moves");
    }

    // ---------------------------------------------------------------------
    // Capturing must clear the captured piece from both its own board and A
    // ---------------------------------------------------------------------
    #[test]
    fn capturing_clears_piece_board_and_aggregate_board() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::W, Piece::R, "a1"),
            (PieceColor::B, Piece::P, "a4"),
            (PieceColor::B, Piece::K, "h8"),
        ]);

        let mut node = Node_Min_Max::new_state(bb);
        node.role = 1; // player = 0 = White
        node.calculate_children();

        let target = sq("a4");
        let child = node
            .children
            .iter()
            .find(|c| c.bitboards[0][Piece::R as usize] & target != 0)
            .expect("expected a child where the rook captured on a4");

        assert_eq!(child.bitboards[1][Piece::P as usize] & target, 0, "captured pawn still on its own board");
        assert_eq!(child.bitboards[1][Piece::A as usize] & target, 0, "captured pawn still on black's A board");
    }

    // ---------------------------------------------------------------------
    // child.role bookkeeping sanity check
    // ---------------------------------------------------------------------
    #[test]
    fn children_are_tagged_with_the_side_that_moved() {
        let bb = board(&[
            (PieceColor::W, Piece::K, "e1"),
            (PieceColor::B, Piece::K, "e8"),
        ]);

        let mut node = Node_Min_Max::new_state(bb);
        node.role = 1; // player = 0
        node.calculate_children();

        assert!(!node.children.is_empty());
        for child in &node.children {
            assert_eq!(child.role, 0, "child.role should equal the side that just moved");
        }
    }
}