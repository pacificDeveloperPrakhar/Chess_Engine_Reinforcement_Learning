pub mod PSQT;
pub mod MIN_MAX;


pub fn evaluate_material_score(bitboards: [[u64; 7]; 2]) -> i16 {
    let mut score: i16 = 0;
    for color in 0..2 {
        for piece in 0..6 {
            let piece_count = bitboards[color][piece].count_ones() as i16;
            let value = piece_count * PSQT::PIECE_VALUES[piece];
            score += if color == 0 { value } else { -value };
        }
    }
    score
}

pub fn how_sparse(bitboards: [[u64; 7]; 2]) -> i16 {
    let mut score: i16 = 0;
    for color in 0..2 {
        for piece in 0..6 {
            let mut bytes = bitboards[color][piece].to_be_bytes();
            for i in 0..8 {
                bytes[i] = !bytes[i];
            }
            let flipped = u64::from_be_bytes(bytes);
            score += (flipped & bitboards[color][piece]).count_ones() as i16;
        }
    }
    score
}

// Phase from total piece count: 0 = opening, 1 = middlegame, 2 = endgame.
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
    // PSQT has two phases per piece: [0] = middlegame, [1] = endgame.
    let phase = if game_progress(bitboards) == 2 { 1 } else { 0 };
    let mut score: i32 = 0;

    for color in 0..2 {
        for piece_idx in 0..6 {
            let mut bb = bitboards[color][piece_idx];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;

                if color == 0 {
                    score += PSQT::PSQT[piece_idx][phase][sq ^ 56] as i32;
                } else {
                    score -= PSQT::PSQT[piece_idx][phase][sq] as i32;
                }
            }
        }
    }

    score as f64
}

pub fn evaluate_score(bitboards: [[u64; 7]; 2]) -> f64 {
    evaluate_material_score(bitboards) as f64 + evaluate_psqt_score(bitboards)
}