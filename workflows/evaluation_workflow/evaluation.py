#!/usr/bin/env python3
"""
Playouts: my engine vs local Stockfish (first half) and Lichess AI (second half).
See workflows/workflow.md, part 2.

    python3 workflows/evaluation_workflow/evaluation.py --games 2
    python3 workflows/evaluation_workflow/evaluation.py --depth 5
    python3 workflows/evaluation_workflow/evaluation.py --depth 7 --games 4 --only stockfish

Rows are appended to data/playout_stockfish.csv / data/playout_lichess.csv.
A game already in the file for the same (sno, depth) is skipped, so a
stopped run continues where it left off and several depths share one file.
"""
import argparse
import csv
import os
import subprocess
import sys
import time

import chess
import chess.engine
import psutil

# ---------------- settings ----------------
CUSTOM_DEPTH = 3
STOCKFISH_DEPTH = 3
LICHESS_LEVEL = 1
N_GAMES = 20
MAX_PLIES = 300
MOVE_TIMEOUT_S = 120
MIN_FREE_RAM_MB = 1000
MAX_LOAD_PER_CORE = 2.0

ENGINE_CMD = "./target/release/chess_engine"
STOCKFISH_CMD = "stockfish"
FEN_CSV = "data/evaluation.csv"
OUT = {"stockfish": "data/playout_stockfish.csv", "lichess": "data/playout_lichess.csv"}
HEADER = ["sno", "stockfish_depth", "custom_engine_depth", "winner", "custom_engine_time_s"]
ME = "my_engine"
# ------------------------------------------


class Stop(Exception):
    """The stop guard fired: end the whole run."""


class EngineError(Exception):
    """My engine crashed / gave no move / gave an illegal move: loss for this game."""


def guard(sno, move_no):
    free_mb = psutil.virtual_memory().available / 2**20
    if free_mb < MIN_FREE_RAM_MB:
        raise Stop(f"free memory {free_mb:.0f} MB < {MIN_FREE_RAM_MB} MB, game {sno}, move {move_no}")
    load = os.getloadavg()[0] / os.cpu_count()
    if load > MAX_LOAD_PER_CORE:
        raise Stop(f"load per core {load:.2f} > {MAX_LOAD_PER_CORE}, game {sno}, move {move_no}")


def load_fens(path, n):
    rows = []
    with open(path) as f:
        for row in csv.DictReader(f):
            try:
                board = chess.Board(row["fen"])
            except ValueError:
                continue
            if board.is_valid() and not board.is_game_over():
                rows.append((int(row["sno"]), board.fen()))
            if len(rows) == n:
                break
    return rows


def engine_string(board, move):
    """My engine's text for a move, e.g. 'e1->f1' for white O-O (lowest left / lowest filled square)."""
    me = board.turn
    before = int(board.occupied_co[me])
    after_board = board.copy(stack=False)
    after_board.push(move)
    after = int(after_board.occupied_co[me])
    return chess.square_name(chess.lsb(before & ~after)) + "->" + chess.square_name(chess.lsb(after & ~before))


def my_engine_move(board, depth, sno, move_no):
    try:
        proc = subprocess.run(
            [ENGINE_CMD, "--eval_fen", board.fen(), "--depth", str(depth)],
            capture_output=True, text=True, timeout=MOVE_TIMEOUT_S,
        )
    except subprocess.TimeoutExpired:  # subprocess.run kills the child on timeout
        raise Stop(f"my engine took more than {MOVE_TIMEOUT_S}s, game {sno}, move {move_no}")
    best = None
    for line in proc.stdout.splitlines():
        if line.startswith("best move: "):
            best = line[len("best move: "):].strip()
    if best is None:
        raise EngineError(f"engine gave no move for {board.fen()}: {proc.stderr.strip()[-300:]}")
    matches = [m for m in board.legal_moves if engine_string(board, m) == best]
    if not matches:
        raise EngineError(f"engine move {best} is not legal in {board.fen()}")
    for m in matches:
        if m.promotion in (None, chess.QUEEN):
            return m
    return matches[0]


def local_result(board):
    outcome = board.outcome(claim_draw=True)
    if outcome is not None:
        if outcome.winner is None:
            return "draw"
        return "white" if outcome.winner == chess.WHITE else "black"
    if len(board.move_stack) >= MAX_PLIES:
        return "draw"
    return None


def timed_move(board, depth, sno, clock):
    move_no = board.fullmove_number
    guard(sno, move_no)
    t = time.perf_counter()
    move = my_engine_move(board, depth, sno, move_no)
    clock[0] += time.perf_counter() - t
    return move


def play_stockfish(sno, fen, my_color, depth, sf, clock):
    board = chess.Board(fen)
    while True:
        result = local_result(board)
        if result:
            return result
        if (board.turn == chess.WHITE) == (my_color == "white"):
            board.push(timed_move(board, depth, sno, clock))
        else:
            guard(sno, board.fullmove_number)
            try:
                board.push(sf.play(board, chess.engine.Limit(depth=STOCKFISH_DEPTH, time=MOVE_TIMEOUT_S)).move)
            except chess.engine.EngineError as e:
                raise Stop(f"stockfish failed ({e}), game {sno}, move {board.fullmove_number}")


def play_lichess(sno, fen, my_color, depth, client, clock):
    game_id = client.challenges.create_ai(level=LICHESS_LEVEL, color=my_color, position=fen)["id"]
    print(f"    https://lichess.org/{game_id}")
    try:
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
            if state["status"] not in ("created", "started"):
                return local_result(board) or state.get("winner", "draw")
            result = local_result(board)
            if result:
                if result == "draw":
                    try:
                        client.board.resign_game(game_id)
                    except Exception:
                        pass
                return result
            if (board.turn == chess.WHITE) == (my_color == "white"):
                client.board.make_move(game_id, timed_move(board, depth, sno, clock).uci())
    except (Stop, EngineError):
        try:
            client.board.resign_game(game_id)
        except Exception:
            pass
        raise
    raise EngineError(f"lichess stream ended early for game {game_id}")


def done_games(path, depth):
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        return {int(r["sno"]) for r in csv.DictReader(f) if r["custom_engine_depth"] == str(depth)}


def append_row(path, row):
    new = not os.path.exists(path)
    with open(path, "a", newline="") as f:
        w = csv.writer(f)
        if new:
            w.writerow(HEADER)
        w.writerow(row)


def main():
    global STOCKFISH_DEPTH, LICHESS_LEVEL
    p = argparse.ArgumentParser()
    p.add_argument("--games", type=int, default=N_GAMES)
    p.add_argument("--depth", type=int, default=CUSTOM_DEPTH)
    p.add_argument("--stockfish_depth", type=int, default=STOCKFISH_DEPTH)
    p.add_argument("--lichess_level", type=int, default=LICHESS_LEVEL)
    p.add_argument("--only", choices=["stockfish", "lichess"], help="play only this half")
    a = p.parse_args()
    STOCKFISH_DEPTH, LICHESS_LEVEL = a.stockfish_depth, a.lichess_level

    games = load_fens(FEN_CSV, a.games)
    n_lichess = a.games // 2 if a.games > 1 else 0
    plan = []
    for i, (sno, fen) in enumerate(games, start=1):
        opp = "lichess" if i > a.games - n_lichess else "stockfish"
        if a.only and opp != a.only:
            continue
        plan.append((i, sno, fen, opp))

    done = {k: done_games(v, a.depth) for k, v in OUT.items()}
    plan = [g for g in plan if g[1] not in done[g[3]]]
    print(f"depth {a.depth}: {len(plan)} games to play")

    client = None
    if any(g[3] == "lichess" for g in plan):
        token = os.environ.get("LICHESS_TOKEN")
        if not token:
            sys.exit("LICHESS_TOKEN is not set (needs board:play)")
        import berserk
        client = berserk.Client(berserk.TokenSession(token))

    sf = chess.engine.SimpleEngine.popen_uci(STOCKFISH_CMD)
    try:
        for i, sno, fen, opp in plan:
            my_color = "white" if i % 2 == 1 else "black"
            print(f"game {sno}: my engine ({my_color}, d{a.depth}) vs {opp}  {fen}", flush=True)
            clock = [0.0]
            try:
                if opp == "lichess":
                    result = play_lichess(sno, fen, my_color, a.depth, client, clock)
                else:
                    result = play_stockfish(sno, fen, my_color, a.depth, sf, clock)
                winner = "draw" if result == "draw" else ME if result == my_color else opp
            except EngineError as e:
                print(f"    ERROR {e}  -> counted as a loss")
                winner = opp
            print(f"    winner: {winner}  (my engine {clock[0]:.2f}s)", flush=True)
            append_row(OUT[opp], [sno, STOCKFISH_DEPTH if opp == "stockfish" else "-",
                                  a.depth, winner, f"{clock[0]:.2f}"])
    except Stop as e:
        print(f"STOPPED: {e}")
        sys.exit(2)
    finally:
        try:
            sf.quit()
        except Exception:
            sf.close()


if __name__ == "__main__":
    main()
