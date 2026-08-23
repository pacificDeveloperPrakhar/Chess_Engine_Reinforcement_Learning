pub mod generator;
pub mod evaluator;
pub mod structures;
pub mod parser;
use generator::l_shape_moves;
fn main() {
    let moves = l_shape_moves::l_squares();
    println!("L-shaped moves from (0, 0): {:?}", moves);
}
