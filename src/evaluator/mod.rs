pub mod PSQT;
pub mod MIN_MAX;

pub fn evaluate_material_score(bitboards: [[u64; 7]; 2]) -> i32 {
    let mut score: i32 = 0;
    for color in 0..2 {
        for piece in 0..6 {
            let count = bitboards[color][piece].count_ones() as i32;
            let value = PSQT::PIECE_VALUES[piece] as i32;
            score += if color == 0 { count * value } else { -count * value };
        }
    }
    score
}

// TODO: define what "sparse" should measure. The old version always returned 0.
pub fn how_sparse(_bitboards: [[u64; 7]; 2]) -> i32 {
    0
}

// Phase is based on the total number of pieces left on the board (both sides).
// Assumes PSQT::PSQT[0] = opening, [1] = middlegame, [2] = endgame. Tune the thresholds.
pub fn game_progress(bitboards: [[u64; 7]; 2]) -> usize {
    let mut pieces: u32 = 0;
    for color in 0..2 {
        for piece in 0..6 {
            pieces += bitboards[color][piece].count_ones();
        }
    }
    if pieces > 24 {
        0
    } else if pieces > 12 {
        1
    } else {
        2
    }
}

pub fn evaluate_psqt_score(bitboards: [[u64; 7]; 2]) -> f64 {
    let progress = game_progress(bitboards);
    let mut score: i32 = 0;

    for i in 0..2 {
        for j in 0..6 {
            let mut piece = bitboards[i][j];
            while piece != 0 {
                let sq = piece.trailing_zeros() as usize;
                piece &= piece - 1;

                if i == 0 {
                    score += PSQT::PSQT[progress][j][sq] as i32;
                } else {
                    score -= PSQT::PSQT[progress][j][sq ^ 56] as i32;
                }
            }
        }
    }

    score as f64
}

pub fn evaluate_score(bitboards: [[u64; 7]; 2]) -> f64 {
    let mut score = evaluate_material_score(bitboards) as f64;
    score += evaluate_psqt_score(bitboards);
    score
}