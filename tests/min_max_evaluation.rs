// tests/min_max_evaluation.rs

// interface.rs is not part of the library, so include it here directly.
use chess_engine::{parser, structures};
#[path = "../src/interface.rs"]
mod interface;

#[test]
fn min_max_eval_for_each_fen() {
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1",
        "3rk3/8/8/8/3Q4/8/8/4K3 b - - 0 1",
        "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1",
        "4k3/8/4p3/3p4/8/8/8/3QK3 w - - 0 1",
    ];

    for fen in fens {
        let (score, best_move) = interface::min_max_eval(fen, 3);
        println!("{fen} -> best move {best_move}, score {score}");
    }
}
