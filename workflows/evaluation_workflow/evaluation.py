#!/usr/bin/env python3
"""
Play my engine (min_max_enhanced) against Lichess AI and local Stockfish.

First half of the games: my engine vs Stockfish (Limit(depth=DEPTH)), local.
Second half:            my engine vs Lichess AI (level LICHESS_LEVEL), online.
My engine switches between white and black on alternate games.
Every finished game is written to data/match_results.csv right away.

Setup:
    sudo apt install stockfish
    pip install chess berserk
    RUSTFLAGS="--cfg USE_CLI" cargo build --release
    export LICHESS_TOKEN=...      # token with scope board:play

Run (from the project folder):
    python3 workflows/evaluation_workflow/evaluation.py              # N_GAMES games
    python3 workflows/evaluation_workflow/evaluation.py --games 2    # quick test: 1 Lichess + 1 Stockfish
"""
import argparse
import csv
import os
import subprocess
import sys
from collections import Counter

import chess
import chess.engine

# ---------------- config ----------------
DEPTH = 3              # search depth for my engine and for Stockfish (depth 4 takes ~4 min per move)
LICHESS_LEVEL = 1      # Lichess AI level 1..8
N_GAMES = 20           # total games: first half Stockfish, second half Lichess
ENGINE_CMD = "./target/release/chess_engine"
STOCKFISH_CMD = "stockfish"
FEN_CSV = "data/fen_positions.csv"
OUT_CSV = "data/match_results.csv"
MAX_PLIES = 300        # game is a draw after this many half-moves
# ----------------------------------------

ME = "my_engine"


def load_fens(path, n):
    """First n FENs from the csv that are legal and not already finished."""
    fens = []
    with open(path) as f:
        for row in csv.DictReader(f):
            try:
                board = chess.Board(row["fen"])
            except ValueError:
                continue
            if board.is_valid() and not board.is_game_over():
                fens.append(board.fen())
            if len(fens) == n:
                break
    return fens


def engine_string(board, move):
    """The text my engine would print for this move, e.g. 'e1->f1' for white O-O.

    The engine finds its move by comparing its own pieces before and after,
    and takes the lowest square that was left and the lowest square that was filled.
    """
    me = board.turn
    before = int(board.occupied_co[me])
    after_board = board.copy(stack=False)
    after_board.push(move)
    after = int(after_board.occupied_co[me])
    left = before & ~after
    filled = after & ~before
    return chess.square_name(chess.lsb(left)) + "->" + chess.square_name(chess.lsb(filled))


def my_engine_move(board):
    proc = subprocess.run(
        [ENGINE_CMD, "--eval_fen", board.fen(), "--depth", str(DEPTH)],
        capture_output=True, text=True, timeout=1800,
    )
    best = None
    for line in proc.stdout.splitlines():
        if line.startswith("best move: "):
            best = line[len("best move: "):].strip()
    if best is None:
        raise RuntimeError(f"engine gave no move for {board.fen()}: {proc.stderr.strip()[-300:]}")

    matches = [m for m in board.legal_moves if engine_string(board, m) == best]
    if not matches:
        raise RuntimeError(f"engine move {best} is not legal in {board.fen()}")
    # several matches only happen for promotions: the engine promotes to a queen
    for m in matches:
        if m.promotion in (None, chess.QUEEN):
            return m
    return matches[0]


def local_result(board, start_plies):
    """None while the game goes on, else 'white' / 'black' / 'draw'."""
    outcome = board.outcome(claim_draw=True)
    if outcome is not None:
        if outcome.winner is None:
            return "draw"
        return "white" if outcome.winner == chess.WHITE else "black"
    if len(board.move_stack) - start_plies >= MAX_PLIES:
        return "draw"
    return None


def winner_name(result, my_color, opponent):
    if result == "draw":
        return "draw"
    return ME if result == my_color else opponent


# ---------------- Stockfish game ----------------
def play_stockfish(fen, my_color, stockfish):
    board = chess.Board(fen)
    while True:
        result = local_result(board, 0)
        if result:
            return result
        mine = (board.turn == chess.WHITE) == (my_color == "white")
        if mine:
            move = my_engine_move(board)
        else:
            move = stockfish.play(board, chess.engine.Limit(depth=DEPTH)).move
        board.push(move)


# ---------------- Lichess game ----------------
def play_lichess(fen, my_color, client):
    """Start a game against Lichess AI from fen and play it on the Board API."""
    game = client.challenges.create_ai(level=LICHESS_LEVEL, color=my_color, position=fen)
    game_id = game["id"]
    print(f"    lichess game: https://lichess.org/{game_id}")

    for event in client.board.stream_game_state(game_id):
        if event["type"] == "gameFull":
            state = event["state"]
        elif event["type"] == "gameState":
            state = event
        else:
            continue

        board = chess.Board(fen)
        for uci in state["moves"].split():
            board.push_uci(uci)

        # Lichess ended the game (mate, stalemate, resign, draw, time out, ...)
        if state["status"] not in ("created", "started"):
            local = local_result(board, 0)
            if local:
                return local
            if "winner" in state:
                return state["winner"]
            return "draw"

        # our own rules: claimable draw or the ply cap end the game here
        result = local_result(board, 0)
        if result:
            if result == "draw":
                try:
                    client.board.resign_game(game_id)  # close the Lichess game; we record a draw
                except Exception:
                    pass
            return result

        mine = (board.turn == chess.WHITE) == (my_color == "white")
        if mine:
            client.board.make_move(game_id, my_engine_move(board).uci())

    raise RuntimeError(f"lichess stream ended early for game {game_id}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--games", type=int, default=N_GAMES)
    args = parser.parse_args()
    n = args.games

    fens = load_fens(FEN_CSV, n)
    if len(fens) < n:
        sys.exit(f"only found {len(fens)} usable FENs in {FEN_CSV}")

    n_lichess = n // 2 if n > 1 else 0
    opponent_tag = f"lichess_L{LICHESS_LEVEL}"

    client = None
    if n_lichess:
        token = os.environ.get("LICHESS_TOKEN")
        if not token:
            sys.exit("LICHESS_TOKEN is not set (needs scope board:play)")
        import berserk
        client = berserk.Client(berserk.TokenSession(token))

    stockfish = chess.engine.SimpleEngine.popen_uci(STOCKFISH_CMD)
    tally = {opponent_tag: Counter(), "stockfish": Counter()}

    try:
        with open(OUT_CSV, "w", newline="") as out:
            writer = csv.writer(out)
            writer.writerow(["sno", "opponent", "starting_fen", "depth", "role", "winner"])
            out.flush()

            for i, fen in enumerate(fens, start=1):
                my_color = "white" if i % 2 == 1 else "black"
                vs_lichess = i > n - n_lichess  # Stockfish games first, Lichess games after
                opponent = opponent_tag if vs_lichess else "stockfish"
                print(f"game {i}/{n}: my engine ({my_color}) vs {opponent}  {fen}")

                try:
                    if vs_lichess:
                        result = play_lichess(fen, my_color, client)
                    else:
                        result = play_stockfish(fen, my_color, stockfish)
                    winner = winner_name(result, my_color, "lichess" if vs_lichess else "stockfish")
                except RuntimeError as err:
                    # my engine crashed or played an illegal move: it forfeits this game
                    print(f"    ERROR: {err}  -> counted as a loss")
                    winner = "lichess" if vs_lichess else "stockfish"

                print(f"    winner: {winner}")
                writer.writerow([i, opponent, fen, DEPTH, my_color, winner])
                out.flush()

                key = "win" if winner == ME else "draw" if winner == "draw" else "loss"
                tally[opponent][key] += 1
    finally:
        stockfish.quit()

    print("\nopponent       W   D   L   (my engine's point of view)")
    total = Counter()
    for opponent, c in tally.items():
        if sum(c.values()):
            print(f"{opponent:<13} {c['win']:>2}  {c['draw']:>2}  {c['loss']:>2}")
            total.update(c)
    if total["win"] > total["loss"]:
        verdict = "my_engine"
    elif total["loss"] > total["win"]:
        verdict = "the opponents"
    else:
        verdict = "nobody (level)"
    print(f"overall: {total['win']}W {total['draw']}D {total['loss']}L -> winner: {verdict}")


if __name__ == "__main__":
    main()
