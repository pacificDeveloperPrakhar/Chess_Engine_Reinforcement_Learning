#!/usr/bin/env python3
"""
Simple human vs engine chess UI for testing.

You play White (click a piece, then click where it goes).
Your Rust engine plays Black. Each engine move is calculated by running:
    ./target/release/chess_engine --eval_fen "<fen>" --depth <N>

Setup:
    pip install python-chess
Run (from the project folder):
    python3 play_vs_engine.py
Then open http://127.0.0.1:8000 in your browser.

Optional: set the engine path with the CHESS_ENGINE environment variable.
"""
import json
import os
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import chess

ENGINE = os.environ.get("CHESS_ENGINE", "./target/release/chess_engine")
PORT = 8000

lock = threading.Lock()
game = {"board": chess.Board(), "depth": 3}


def ask_engine(fen, depth):
    """Run the Rust engine CLI on one position and read its answer."""
    proc = subprocess.run(
        [ENGINE, "--eval_fen", fen, "--depth", str(depth)],
        capture_output=True, text=True, timeout=600,
    )
    if proc.returncode != 0:
        raise RuntimeError("engine failed: " + (proc.stderr.strip() or "no error text"))

    score, best = None, None
    for line in proc.stdout.splitlines():
        if line.startswith("score: "):
            score = line[len("score: "):].strip()
        elif line.startswith("best move: "):
            best = line[len("best move: "):].strip()
    if best is None:
        raise RuntimeError("engine gave no best move. Output was: " + proc.stdout.strip())
    # The engine writes "f1->b5"; standard notation is "f1b5".
    return score, best.replace("->", "")


def to_legal_move(board, uci):
    """Turn the engine's move into a move that is legal on this board."""
    for candidate in (uci, uci + "q"):  # second try adds queen promotion
        try:
            move = chess.Move.from_uci(candidate)
        except ValueError:
            continue
        if move in board.legal_moves:
            return move
    raise RuntimeError("engine suggested an illegal move: " + uci)


def game_result(board):
    if board.is_checkmate():
        return "Checkmate - " + ("you win!" if board.turn == chess.BLACK else "the engine wins.")
    if board.is_game_over():
        return "Draw."
    return None


def state_json(extra=None):
    board = game["board"]
    result = game_result(board)
    data = {
        "fen": board.fen(),
        "legal": [m.uci() for m in board.legal_moves],
        "over": result is not None,
        "status": result or ("Your move (White)" if board.turn == chess.WHITE else "Engine is thinking..."),
        "depth": game["depth"],
        "engine_move": None,
        "score": None,
        "last": [],
        "error": None,
    }
    if extra:
        data.update(extra)
    return data


class Handler(BaseHTTPRequestHandler):
    def _send(self, code, body, content_type="application/json"):
        raw = body.encode("utf-8") if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self):
        if self.path == "/":
            self._send(200, PAGE, "text/html; charset=utf-8")
        elif self.path == "/api/state":
            with lock:
                self._send(200, json.dumps(state_json()))
        else:
            self._send(404, json.dumps({"error": "not found"}))

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(length) or b"{}")

        with lock:
            if self.path == "/api/new":
                try:
                    depth = int(body.get("depth", 3))
                    if not 1 <= depth <= 8:
                        raise ValueError
                except ValueError:
                    self._send(200, json.dumps(state_json({"error": "depth must be a whole number from 1 to 8"})))
                    return
                game["board"] = chess.Board()
                game["depth"] = depth
                self._send(200, json.dumps(state_json()))

            elif self.path == "/api/move":
                board = game["board"]
                try:
                    if board.is_game_over():
                        raise RuntimeError("the game is over, start a new game")
                    move = chess.Move.from_uci(body.get("move", ""))
                    if move not in board.legal_moves:
                        raise RuntimeError("illegal move")

                    board.push(move)
                    extra = {"last": [move.uci()]}
                    if not board.is_game_over():
                        score, best = ask_engine(board.fen(), game["depth"])
                        engine_move = to_legal_move(board, best)
                        board.push(engine_move)
                        extra.update({
                            "engine_move": engine_move.uci(),
                            "score": score,
                            "last": [move.uci(), engine_move.uci()],
                        })
                    self._send(200, json.dumps(state_json(extra)))
                except (RuntimeError, ValueError) as err:
                    self._send(200, json.dumps(state_json({"error": str(err)})))
            else:
                self._send(404, json.dumps({"error": "not found"}))

    def log_message(self, fmt, *args):
        pass  # keep the terminal quiet


PAGE = """<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Human vs Engine</title>
<style>
  body { font-family: sans-serif; display: flex; gap: 28px; padding: 20px; align-items: flex-start; }
  #board { display: grid; grid-template-columns: repeat(8, 62px); grid-template-rows: repeat(8, 62px);
           border: 2px solid #333; }
  .sq { display: flex; align-items: center; justify-content: center; font-size: 46px; cursor: pointer;
        user-select: none; position: relative; }
  .light { background: #f0d9b5; }
  .dark { background: #b58863; }
  .last { background: #f6f669 !important; }
  .sel { outline: 4px solid #4a90e2; outline-offset: -4px; }
  .dot::after { content: ""; width: 18px; height: 18px; border-radius: 50%; background: rgba(0,0,0,.3); position: absolute; }
  .white { color: #fff; text-shadow: 0 0 2px #000, 0 0 2px #000; }
  .black { color: #000; }
  #panel { min-width: 280px; }
  button { padding: 6px 14px; margin-left: 6px; }
  input { width: 60px; padding: 4px; }
  #err { color: #b00; min-height: 1em; }
  #status { font-size: 18px; font-weight: bold; }
</style>
</head>
<body>
<div id="board"></div>
<div id="panel">
  <h3>You (White) vs Engine (Black)</h3>
  <label>Engine depth (1-8)
    <input id="depth" type="number" min="1" max="8" value="3">
  </label>
  <button id="newgame">New game</button>
  <p id="status"></p>
  <p>Engine last move: <b id="enginemove">-</b></p>
  <p>Engine score: <b id="score">-</b></p>
  <p id="err"></p>
</div>
<script>
  const SYM = { K:'♔', Q:'♕', R:'♖', B:'♗', N:'♘', P:'♙',
                k:'♚', q:'♛', r:'♜', b:'♝', n:'♞', p:'♟' };
  const FILES = 'abcdefgh';
  let st = null;
  let selected = null;

  const boardEl = document.getElementById('board');
  const errEl = document.getElementById('err');

  function parseFen(fen) {
    const rows = fen.split(' ')[0].split('/');
    return rows.map(r => {
      const row = [];
      for (const ch of r) {
        if (/\\d/.test(ch)) for (let i = 0; i < Number(ch); i++) row.push(null);
        else row.push(ch);
      }
      return row;
    });
  }

  function render() {
    const grid = parseFen(st.fen);
    const targets = selected
      ? st.legal.filter(m => m.slice(0, 2) === selected).map(m => m.slice(2, 4))
      : [];
    boardEl.innerHTML = '';
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const name = FILES[c] + (8 - r);
        const piece = grid[r][c];
        const div = document.createElement('div');
        div.className = 'sq ' + (((r + c) % 2 === 0) ? 'light' : 'dark');
        if (st.last.some(m => m.slice(0, 2) === name || m.slice(2, 4) === name)) div.classList.add('last');
        if (name === selected) div.classList.add('sel');
        if (targets.includes(name)) div.classList.add('dot');
        if (piece) {
          div.textContent = SYM[piece];
          div.classList.add(piece === piece.toUpperCase() ? 'white' : 'black');
        }
        div.onclick = () => onSquare(name);
        boardEl.appendChild(div);
      }
    }
    document.getElementById('status').textContent = st.status;
    document.getElementById('enginemove').textContent = st.engine_move || '-';
    document.getElementById('score').textContent = st.score ?? '-';
    errEl.textContent = st.error || '';
  }

  async function post(url, body) {
    const res = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' },
                                   body: JSON.stringify(body || {}) });
    return res.json();
  }

  async function onSquare(name) {
    if (!st || st.over || st.engine_thinking) return;
    if (selected && st.legal.some(m => m.startsWith(selected) && m.slice(2, 4) === name)) {
      const move = selected + name;
      selected = null;
      document.getElementById('status').textContent = 'Engine is thinking...';
      const needsPromo = st.legal.includes(move + 'q');
      st = await post('/api/move', { move: needsPromo ? move + 'q' : move });
      render();
      return;
    }
    const piece = parseFen(st.fen)[8 - Number(name[1])][FILES.indexOf(name[0])];
    selected = (piece && piece === piece.toUpperCase()) ? name : null;
    render();
  }

  document.getElementById('newgame').onclick = async () => {
    selected = null;
    const depth = Number(document.getElementById('depth').value);
    document.getElementById('status').textContent = 'Starting...';
    st = await post('/api/new', { depth });
    render();
  };

  (async () => {
    st = await (await fetch('/api/state')).json();
    render();
  })();
</script>
</body>
</html>
"""


if __name__ == "__main__":
    print(f"Open http://127.0.0.1:{PORT} in your browser (Ctrl+C to stop)")
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
