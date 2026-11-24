# Task: produce three CSV files from chess engine games and benchmarks

Repo: ~/Desktop/Chess_Engine_Reinforcement_Learning
Run commands from the repo folder. Do not create or change any other file.

## Output files (only these three)

1. `data/playout_stockfish.csv`: games of my engine against Stockfish.
   Columns: `sno,stockfish_depth,custom_engine_depth,winner,custom_engine_time_s`

2. `data/playout_lichess.csv`: games of my engine against the Lichess AI.
   Columns: `sno,stockfish_depth,custom_engine_depth,winner,custom_engine_time_s`
   Use `-` for `stockfish_depth`, since Stockfish is not used here.

3. `data/fen_evaluation_compared_with_stockfish.csv`: my engine's move and score compared with Stockfish, for each position in `data/fen_positions.csv` that has both answers filled in.
   Columns: `sno,fen,role,custom_depth,custom_move,custom_value,stockfish_move,stockfish_value,same_move`

## Column rules

- `winner` is one of `my_engine`, `stockfish`, `lichess` or `draw`.
- `custom_engine_time_s` is the total seconds my engine spent on its moves in that game, to 2 decimals.
- A game counts as a loss for my engine if it crashes, gives no move, or gives an illegal move.
- Scores are in centipawns. A mate is written as `#N`.

## Settings

| Setting | Default |
|---|---|
| `CUSTOM_DEPTH` | 3 |
| `STOCKFISH_DEPTH` | 3 |
| `LICHESS_LEVEL` | 1 |
| `N_GAMES` | 20 (half Stockfish, half Lichess) |
| `MAX_PLIES` | 300 (half-moves before a draw) |

## Stop if the computer may hang

Before every move, check:

- free memory is above 1000 MB,
- the 1-minute load per CPU core is below 2.0,
- the engine answers within 120 seconds.

If any check fails, stop the run. Keep the finished games, do not write the unfinished one, print the reason, and exit.

## Run

```bash
python3 workflows/evaluation_workflow/evaluation.py --games 2   # quick test
python3 workflows/evaluation_workflow/evaluation.py             # full run
```

Each file is saved after every finished game. The script needs `LICHESS_TOKEN` (board:play scope) for Lichess games.