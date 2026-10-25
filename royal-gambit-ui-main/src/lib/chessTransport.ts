import { START_POSITION, fenToArray, isWhitePiece } from "./chessNotationParser";
import { applyMove, hasAnyMove, isInCheck, overlayNotation } from "./chessRules";

/**
 * Transport contract matching the Rust gRPC-Web service.
 *
 * Swap `mockTransport` for a generated gRPC-Web client (grpc-web / connect-web):
 *   const client = createClient(ChessService, createGrpcWebTransport({ baseUrl }));
 *   subscribeBoard -> client.streamBoard({}) async iterator
 *   requestLegalMoves -> client.getLegalMoves({ square })   // returns overlay string
 *   submitMove -> client.playMove({ from, to })             // returns board string
 */
export interface BoardUpdate {
  /** Board string from get_pgn_notation_from_bitboards */
  board: string;
  sideToMove: "white" | "black";
  lastMove?: { from: number; to: number };
  captured?: { square: number; piece: string };
  checkSquare?: number | null;
  status: "active" | "check" | "checkmate" | "stalemate";
}

export interface ChessTransport {
  getInitialState(): Promise<BoardUpdate>;
  /** Overlay string from get_legal_possible_moves_notation_for_rendering */
  requestLegalMoves(square: number): Promise<string>;
  submitMove(from: number, to: number): Promise<BoardUpdate>;
  reset(): Promise<BoardUpdate>;
}

const latency = (ms = 90) => new Promise((r) => setTimeout(r, ms));

function describe(board: string, sideToMove: "white" | "black"): BoardUpdate["status"] {
  const arr = fenToArray(board);
  const white = sideToMove === "white";
  const check = isInCheck(arr, white);
  const canMove = hasAnyMove(arr, white);
  if (check && !canMove) return "checkmate";
  if (!canMove) return "stalemate";
  return check ? "check" : "active";
}

export function createMockTransport(): ChessTransport {
  let board = START_POSITION;
  let side: "white" | "black" = "white";

  const snapshot = (extra: Partial<BoardUpdate> = {}): BoardUpdate => {
    const status = describe(board, side);
    const arr = fenToArray(board);
    const kingIdx = arr.indexOf(side === "white" ? "K" : "k");
    return {
      board,
      sideToMove: side,
      status,
      checkSquare: status === "check" || status === "checkmate" ? kingIdx : null,
      ...extra,
    };
  };

  return {
    async getInitialState() {
      await latency(40);
      return snapshot();
    },
    async requestLegalMoves(square) {
      await latency(20);
      const arr = fenToArray(board);
      const piece = arr[square] ?? ".";
      if (piece === "." || isWhitePiece(piece) !== (side === "white")) return board;
      return overlayNotation(board, square);
    },
    async submitMove(from, to) {
      await latency(60);
      const arr = fenToArray(board);
      const target = arr[to] ?? ".";
      board = applyMove(board, from, to);
      side = side === "white" ? "black" : "white";
      return snapshot({
        lastMove: { from, to },
        ...(target !== "." ? { captured: { square: to, piece: target } } : {}),
      });
    },
    async reset() {
      board = START_POSITION;
      side = "white";
      return snapshot();
    },
  };
}
