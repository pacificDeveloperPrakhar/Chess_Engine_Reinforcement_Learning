# Results: 2026-11-24

Run on a 2-core cloud machine, finished 24 Nov 2026 about 2:22 AM IST.
Engine built from the current `src/` with `RUSTFLAGS="--cfg USE_CLI" cargo build --release`.
Reference engine: Stockfish 16 (NNUE). Lichess was not reachable, so no Lichess data.

## Files

| File | What it holds |
|---|---|
| `evaluation.csv` | 2,000 positions: my engine vs Stockfish (depth 18), with score, best move, time, RAM, same_move |
| `benchmark.csv` | 1 position at depths 1–9: time and RAM for my engine and Stockfish |
| `playout_stockfish.csv` | 30 games: my engine depth 5 vs Stockfish depth 3 |
| `run_all.py` | the script that produced the three CSVs |
| `run_all.log` | full log of the run |

## evaluation.csv

| My engine depth | Rows | Avg time | Max time | Peak RAM | Same move as Stockfish |
|---|---|---|---|---|---|
| 9 | 5 | 654 s | 1,302 s | 458 MB | 40% |
| 8 | 20 | 50 s | 309 s | 141 MB | 70% |
| 7 | 100 | 9.7 s | 209 s | 72 MB | 48% |
| 5 | 1,875 | 0.3 s | 3.3 s | 5 MB | 39% |

- All 2,000 rows OK, no errors or timeouts.
- Same move as Stockfish overall: 794 / 2,000 (39.7%).
- Score gap from Stockfish: median 47 cp, mean 101 cp.
- Stockfish scores are from White's point of view.

## benchmark.csv (position: `r1bqkbnr/ppp2ppp/2n5/3p4/3P4/5N2/PPP2PPP/RNBQKB1R w KQkq - 2 5`)

| Depth | My engine time | My engine RAM | Stockfish time |
|---|---|---|---|
| 1–5 | 0.05–0.6 s | < 4 MB | < 0.01 s |
| 6 | 5.9 s | 11.5 MB | 0.002 s |
| 7 | 34 s | 37 MB | 0.003 s |
| 8 | 246 s | 141 MB | 0.004 s |
| 9 | 1,280 s | 471 MB | 0.03 s |

Each extra depth costs about 5–7 times more time.

## playout_stockfish.csv

| My engine plays | Wins | Draws | Losses |
|---|---|---|---|
| White (15) | 5 | 7 | 3 |
| Black (15) | 8 | 5 | 2 |
| Total (30) | 13 | 12 | 5 |

- Score 63%. No crashes or illegal moves.
- 11 of 12 draws were threefold repetition (the engine gets no game history, so it can't avoid repeats).
- Stockfish depth 3 is a weak setting; this is not an Elo measurement.

## Notes

- My engine's move format is `from->to`; for same_move, Stockfish's UCI move was converted with the rule "lowest vacated square -> lowest newly occupied square" (white O-O = `e1->f1`).
- Lichess playouts (`playout_lichess.csv`) were not run.
