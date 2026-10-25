import { arrayToFen, fenToArray, isWhitePiece } from "./chessNotationParser";

/**
 * Local rules module. Mirrors the Rust backend's
 * `get_legal_possible_moves_notation_for_rendering` output format so the UI can
 * run standalone (mock transport) with exactly the same string contract.
 */

const onBoard = (r: number, c: number) => r >= 0 && r < 8 && c >= 0 && c < 8;

export function pseudoLegalMoves(board: string[], from: number): number[] {
  const piece = board[from] ?? ".";
  if (piece === ".") return [];
  const white = isWhitePiece(piece);
  const type = piece.toLowerCase();
  const r = Math.floor(from / 8);
  const c = from % 8;
  const moves: number[] = [];

  const enemy = (idx: number) =>
    board[idx] !== "." && isWhitePiece(board[idx] ?? ".") !== white;
  const empty = (idx: number) => board[idx] === ".";

  const ray = (dr: number, dc: number, limit = 8) => {
    for (let i = 1; i <= limit; i++) {
      const nr = r + dr * i;
      const nc = c + dc * i;
      if (!onBoard(nr, nc)) break;
      const idx = nr * 8 + nc;
      if (empty(idx)) moves.push(idx);
      else {
        if (enemy(idx)) moves.push(idx);
        break;
      }
    }
  };

  if (type === "p") {
    const dir = white ? -1 : 1; // rank 8 is row 0
    const one = (r + dir) * 8 + c;
    if (onBoard(r + dir, c) && empty(one)) {
      moves.push(one);
      const startRow = white ? 6 : 1;
      const two = (r + dir * 2) * 8 + c;
      if (r === startRow && empty(two)) moves.push(two);
    }
    for (const dc of [-1, 1]) {
      const nr = r + dir;
      const nc = c + dc;
      if (onBoard(nr, nc) && enemy(nr * 8 + nc)) moves.push(nr * 8 + nc);
    }
  } else if (type === "n") {
    const deltas = [
      [-2, -1],
      [-2, 1],
      [-1, -2],
      [-1, 2],
      [1, -2],
      [1, 2],
      [2, -1],
      [2, 1],
    ];
    for (const [dr, dc] of deltas as [number, number][]) {
      const nr = r + dr;
      const nc = c + dc;
      if (!onBoard(nr, nc)) continue;
      const idx = nr * 8 + nc;
      if (empty(idx) || enemy(idx)) moves.push(idx);
    }
  } else if (type === "b" || type === "r" || type === "q") {
    const diagonals = [
      [-1, -1],
      [-1, 1],
      [1, -1],
      [1, 1],
    ];
    const straights = [
      [-1, 0],
      [1, 0],
      [0, -1],
      [0, 1],
    ];
    const dirs =
      type === "b" ? diagonals : type === "r" ? straights : [...diagonals, ...straights];
    for (const [dr, dc] of dirs as [number, number][]) ray(dr, dc);
  } else if (type === "k") {
    for (const dr of [-1, 0, 1])
      for (const dc of [-1, 0, 1]) {
        if (!dr && !dc) continue;
        const nr = r + dr;
        const nc = c + dc;
        if (!onBoard(nr, nc)) continue;
        const idx = nr * 8 + nc;
        if (empty(idx) || enemy(idx)) moves.push(idx);
      }
  }
  return moves;
}

function attacked(board: string[], target: number, byWhite: boolean): boolean {
  for (let i = 0; i < 64; i++) {
    const p = board[i] ?? ".";
    if (p === "." || isWhitePiece(p) !== byWhite) continue;
    if (pseudoLegalMoves(board, i).includes(target)) return true;
  }
  return false;
}

export function kingSquare(board: string[], white: boolean): number {
  return board.indexOf(white ? "K" : "k");
}

export function legalMoves(board: string[], from: number): number[] {
  const piece = board[from] ?? ".";
  if (piece === ".") return [];
  const white = isWhitePiece(piece);
  return pseudoLegalMoves(board, from).filter((to) => {
    const next = [...board];
    next[to] = next[from] ?? ".";
    next[from] = ".";
    const king = kingSquare(next, white);
    return king === -1 || !attacked(next, king, !white);
  });
}

export function isInCheck(board: string[], white: boolean): boolean {
  const king = kingSquare(board, white);
  return king !== -1 && attacked(board, king, !white);
}

export function hasAnyMove(board: string[], white: boolean): boolean {
  for (let i = 0; i < 64; i++) {
    const p = board[i] ?? ".";
    if (p !== "." && isWhitePiece(p) === white && legalMoves(board, i).length) return true;
  }
  return false;
}

/** Board string with '+' (quiet move) and '-' (capture) overlaid on legal targets. */
export function overlayNotation(boardFen: string, from: number): string {
  const board = fenToArray(boardFen);
  const marked = [...board];
  for (const to of legalMoves(board, from)) {
    marked[to] = board[to] === "." ? "+" : "-";
  }
  return arrayToFen(marked);
}

export function applyMove(boardFen: string, from: number, to: number): string {
  const board = fenToArray(boardFen);
  const piece = board[from] ?? ".";
  board[to] = piece;
  board[from] = ".";
  // auto-queen promotion
  if (piece === "P" && to < 8) board[to] = "Q";
  if (piece === "p" && to >= 56) board[to] = "q";
  return arrayToFen(board);
}
