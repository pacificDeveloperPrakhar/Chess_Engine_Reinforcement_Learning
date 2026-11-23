# Work Progress — Chess Engine (Reinforcement Learning)

A day-by-day log of what I worked on in this project.

## 2026-11-21 (Sat)
- Min-max search with alpha-beta pruning (`src/evaluator/MIN_MAX.rs`).
- Board score = material + piece-square tables (`src/evaluator/PSQT.rs`).

## 2026-11-22 (Sun)
- Benchmark command-line tool (`src/utils/engine_cli.rs`, build with `RUSTFLAGS="--cfg USE_CLI"`): fills `data/fen_positions.csv` with Lichess cloud-eval (online), Stockfish (local) and my engine's move + score, side by side.
- Browser game against my engine (`play_vs_engine.py`).
- Compared my engine with Lichess / Stockfish: results were off. Found the cause: the **horizon effect** (the search stops in the middle of a capture and trusts a wrong score).
- Planned three fixes: quiescence search, transposition table, and a lighter min-max that doesn't keep the whole tree.

## 2026-11-23 (Mon)
- Built `min_max_enhanced` (`src/evaluator/MIN_MAX.rs`, `src/structures/state.rs`), now used by `GameSession::evaluate`:
  - **Transposition table** with **Zobrist hashing** (`Zobrist_State`, `TTEntry`): repeated positions are reused when the saved depth is deep enough; only exact scores are saved.
  - **Move ordering**: captures first (most valuable victim, cheapest attacker), then quiet moves to good squares, other quiet moves, and moves where the piece can be taken.
  - Keeps **only the best child** instead of the whole tree.
- Benchmark old vs new (10 positions): same scores in 51/51 tests, about 21x faster at depth 5 (1,689 ms → 78 ms), memory 1,069 MB → 0.36 MB.
  - [Benchmark summary (CSV)](Documentation_Resources/minmax_benchmark_summary.csv)
  - [Every benchmark run (CSV)](Documentation_Resources/minmax_benchmark_runs.csv)

## Next
- [ ] Quiescence search at depth 0 of `min_max_enhanced` (fix for the horizon effect).
- [ ] Re-run `data/fen_positions.csv` with `--engine_eval` and compare with Stockfish again.
- [ ] Check the queen/king table order in `evaluate_psqt_score` (`src/evaluator/mod.rs`).

## Guides
- **2026-11-23 — Making my chess engine search faster: transposition table, move ordering and a lighter min-max** — what I built, why the engine was weak (horizon effect), how `min_max_enhanced` works step by step, and the benchmark results. [PDF](Documentation_Resources/2026-11-23-minmax-transposition-move-ordering.pdf)
