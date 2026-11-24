import csv, os, re, subprocess, sys, time
import chess, chess.engine, psutil

ENGINE = "/home/claude/work/chess/target/release/chess_engine"
SF_BIN = "/usr/games/stockfish"
SRC = "/home/claude/work/chess/data/evaluation_original.csv"
OUT = "/home/claude/work/out"
os.makedirs(OUT, exist_ok=True)

MOVE_TIMEOUT = {1: 120, 2: 120, 3: 120, 4: 120, 5: 120, 6: 300, 7: 600, 8: 1200, 9: 2400}
MIN_FREE_MB = 1000
MAX_LOAD_PER_CORE = 2.5


def log(*a):
    print(time.strftime("%H:%M:%S"), *a, flush=True)


def guard():
    """Wait while the machine is under pressure; return a reason string if it never recovers."""
    for _ in range(30):
        free = psutil.virtual_memory().available / 1e6
        load = os.getloadavg()[0] / (os.cpu_count() or 1)
        if free >= MIN_FREE_MB and load <= MAX_LOAD_PER_CORE:
            return None
        log(f"guard: free={free:.0f}MB load/core={load:.2f}, waiting")
        time.sleep(10)
    return "machine overloaded"


def run_my_engine(fen, depth):
    """Run my engine once; return (score, move, seconds, peak_ram_mb, status)."""
    t0 = time.time()
    p = subprocess.Popen([ENGINE, "--eval_fen", fen, "--depth", str(depth)],
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    ps = psutil.Process(p.pid)
    peak = 0
    status = "ok"
    while p.poll() is None:
        try:
            peak = max(peak, ps.memory_info().rss)
        except psutil.Error:
            pass
        if time.time() - t0 > MOVE_TIMEOUT[depth]:
            p.kill()
            status = "timeout"
            break
        if psutil.virtual_memory().available / 1e6 < MIN_FREE_MB / 2:
            p.kill()
            status = "low_memory"
            break
        time.sleep(0.05)
    out, _ = p.communicate()
    secs = time.time() - t0
    sc = re.search(r"score:\s*(\S+)", out)
    mv = re.search(r"best move:\s*(\S+)", out)
    if status == "ok" and not (sc and mv):
        status = "no_output"
    return (sc.group(1) if sc else "-", mv.group(1) if mv else "-", secs, peak / 1e6, status)


def sf_open():
    return chess.engine.SimpleEngine.popen_uci(SF_BIN)


def sf_pid(sf):
    try:
        return sf.protocol.transport.get_pid()
    except Exception:
        return None


def score_str(pov):
    m = pov.mate()
    return f"#{m}" if m is not None else str(pov.score())


def sf_analyse(sf, board, depth):
    t0 = time.time()
    info = sf.analyse(board, chess.engine.Limit(depth=depth))
    secs = time.time() - t0
    move = info["pv"][0].uci() if info.get("pv") else "-"
    return score_str(info["score"].white()), move, secs


def to_my_format(board, move):
    """Lowest square my side vacated -> lowest square it newly occupies, e.g. white O-O = e1->f1."""
    side = board.turn
    before = set(sq for sq, pc in board.piece_map().items() if pc.color == side)
    b = board.copy()
    b.push(move)
    after = set(sq for sq, pc in b.piece_map().items() if pc.color == side)
    vac = before - after
    occ = after - before
    if not vac or not occ:
        return None
    return f"{chess.square_name(min(vac))}->{chess.square_name(min(occ))}"


def done_snos(path):
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        return set(r["sno"] for r in csv.DictReader(f))


def append_row(path, header, row):
    new = not os.path.exists(path)
    with open(path, "a", newline="") as f:
        w = csv.writer(f)
        if new:
            w.writerow(header)
        w.writerow(row)
        f.flush()
        os.fsync(f.fileno())


# ---------------- benchmark: 1 position, depths 1..9 ----------------
def benchmark():
    path = f"{OUT}/benchmark.csv"
    header = ["depth", "fen", "my_engine_time_s", "my_engine_peak_ram_mb", "my_engine_score",
              "my_engine_best_move", "my_engine_status", "stockfish_time_s", "stockfish_ram_mb",
              "stockfish_score", "stockfish_best_move", "lichess_time_s"]
    fen = "r1bqkbnr/ppp2ppp/2n5/3p4/3P4/5N2/PPP2PPP/RNBQKB1R w KQkq - 2 5"
    done = done_snos_bench(path)
    sf = sf_open()
    for d in range(1, 10):
        if str(d) in done:
            continue
        if guard():
            log("STOPPED in benchmark"); sys.exit(2)
        sc, mv, secs, ram, st = run_my_engine(fen, d)
        sfsc, sfmv, sfsecs = sf_analyse(sf, chess.Board(fen), d)
        pid = sf_pid(sf)
        sfram = psutil.Process(pid).memory_info().rss / 1e6 if pid else 0
        append_row(path, header, [d, fen, f"{secs:.3f}", f"{ram:.1f}", sc, mv, st,
                                  f"{sfsecs:.3f}", f"{sfram:.1f}", sfsc, sfmv, "n/a"])
        log(f"bench d={d} my={secs:.2f}s {ram:.1f}MB {st}  sf={sfsecs:.3f}s")
    sf.quit()


def done_snos_bench(path):
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        return set(r["depth"] for r in csv.DictReader(f))


# ---------------- playouts: 30 games vs Stockfish ----------------
def playouts():
    path = f"{OUT}/playout_stockfish.csv"
    header = ["sno", "stockfish_depth", "custom_engine_depth", "role", "starting_fen",
              "winner", "plies", "end_reason", "custom_engine_time_s"]
    MY_D, SF_D, N, MAX_PLIES = 5, 3, 30, 300
    rows = list(csv.DictReader(open(SRC)))
    starts = []
    for r in rows:
        b = chess.Board(r["fen"])
        if b.is_valid() and not b.is_game_over():
            starts.append(r["fen"])
        if len(starts) == N:
            break
    done = done_snos(path)
    sf = sf_open()
    for g in range(N):
        sno = g + 1
        if str(sno) in done:
            continue
        fen = starts[g]
        board = chess.Board(fen)
        my_color = chess.WHITE if g % 2 == 0 else chess.BLACK
        my_time = 0.0
        winner = None
        reason = ""
        while True:
            if board.is_game_over(claim_draw=True):
                res = board.result(claim_draw=True)
                if res == "1/2-1/2":
                    winner = "draw"
                else:
                    white_won = res == "1-0"
                    winner = "my_engine" if white_won == (my_color == chess.WHITE) else "stockfish"
                reason = board.outcome(claim_draw=True).termination.name.lower()
                break
            if board.ply() - chess.Board(fen).ply() >= MAX_PLIES:
                winner, reason = "draw", "max_plies"
                break
            if guard():
                log("STOPPED in playouts"); sf.quit(); sys.exit(2)
            if board.turn == my_color:
                _, mv, secs, _, st = run_my_engine(board.fen(), MY_D)
                my_time += secs
                move = None
                if st == "ok":
                    cands = [m for m in board.legal_moves if to_my_format(board, m) == mv]
                    q = [m for m in cands if m.promotion in (None, chess.QUEEN)]
                    move = (q or cands or [None])[0]
                if move is None:
                    winner, reason = "stockfish", f"my_engine_{st if st != 'ok' else 'illegal'}:{mv}"
                    log(f"ERROR game {sno}: {reason} fen={board.fen()}")
                    break
            else:
                move = sf.play(board, chess.engine.Limit(depth=SF_D)).move
            board.push(move)
        plies = len(board.move_stack)
        append_row(path, header, [sno, SF_D, MY_D, "white" if my_color else "black", fen,
                                  winner, plies, reason, f"{my_time:.2f}"])
        log(f"game {sno}: {winner} ({reason}) plies={plies} my_time={my_time:.1f}s")
    sf.quit()


# ---------------- evaluation: 2000 rows ----------------
def evaluation():
    path = f"{OUT}/evaluation.csv"
    header = ["sno", "fen", "role", "depth", "stockfish_score", "stockfish_best_move",
              "stockfish_time_s", "my_engine_score", "my_engine_best_move", "my_engine_time_s",
              "my_engine_peak_ram_mb", "my_engine_status", "same_move"]
    rows = list(csv.DictReader(open(SRC)))
    done = done_snos(path)
    sf = sf_open()
    for i, r in enumerate(rows):
        if r["sno"] in done:
            continue
        d = 9 if i < 5 else 8 if i < 25 else 7 if i < 125 else 5
        if guard():
            log("STOPPED in evaluation"); sf.quit(); sys.exit(2)
        board = chess.Board(r["fen"])
        try:
            sfsc, sfmv, sfsecs = sf_analyse(sf, board, 18)
        except Exception as e:
            log(f"stockfish error sno={r['sno']}: {e}; restarting stockfish")
            try: sf.quit()
            except Exception: pass
            sf = sf_open()
            sfsc, sfmv, sfsecs = "-", "-", 0.0
        sc, mv, secs, ram, st = run_my_engine(r["fen"], d)
        same = "-"
        if sfmv != "-" and st == "ok":
            try:
                same = "true" if to_my_format(board, chess.Move.from_uci(sfmv)) == mv else "false"
            except Exception:
                same = "-"
        append_row(path, header, [r["sno"], r["fen"], r["role"], d, sfsc, sfmv, f"{sfsecs:.2f}",
                                  sc, mv, f"{secs:.2f}", f"{ram:.1f}", st, same])
        log(f"eval sno={r['sno']} d={d} my={sc}/{mv} {secs:.1f}s {st}  sf={sfsc}/{sfmv} {sfsecs:.1f}s same={same}")
    sf.quit()


if __name__ == "__main__":
    for step in sys.argv[1:]:
        {"benchmark": benchmark, "playouts": playouts, "evaluation": evaluation}[step]()
    log("ALL DONE")
