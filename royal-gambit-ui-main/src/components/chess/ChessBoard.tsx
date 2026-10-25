import { AnimatePresence, motion } from "motion/react";
import gsap from "gsap";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { PieceGlyph } from "./PieceGlyph";
import {
  isWhitePiece,
  parseCustomNotation,
  squareName,
  type BoardSquare,
} from "@/lib/chessNotationParser";
import {
  createMockTransport,
  type BoardUpdate,
  type ChessTransport,
} from "@/lib/chessTransport";

interface TrackedPiece {
  id: string;
  piece: string;
  square: number;
}

/** Diff the previous placement against the new board so each piece keeps its identity. */
function reconcile(prev: TrackedPiece[], squares: BoardSquare[]): TrackedPiece[] {
  const remaining = [...prev];
  const next: TrackedPiece[] = [];
  const pending: BoardSquare[] = [];

  for (const sq of squares) {
    if (!sq.piece) continue;
    const exact = remaining.findIndex(
      (p) => p.square === sq.index && p.piece === sq.piece,
    );
    if (exact >= 0) {
      next.push(remaining.splice(exact, 1)[0]!);
    } else {
      pending.push(sq);
    }
  }

  for (const sq of pending) {
    const moved = remaining.findIndex((p) => p.piece === sq.piece);
    if (moved >= 0) {
      const piece = remaining.splice(moved, 1)[0]!;
      next.push({ ...piece, square: sq.index });
    } else {
      next.push({
        id: `${sq.piece}-${sq.index}-${Math.random().toString(36).slice(2, 7)}`,
        piece: sq.piece!,
        square: sq.index,
      });
    }
  }
  return next;
}

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const EMPTY_BOARD = "8/8/8/8/8/8/8/8";

export function ChessBoard({ transport }: { transport?: ChessTransport }) {
  const client = useMemo(() => transport ?? createMockTransport(), [transport]);
  const [state, setState] = useState<BoardUpdate | null>(null);
  const [overlay, setOverlay] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [pieces, setPieces] = useState<TrackedPiece[]>([]);
  const [history, setHistory] = useState<string[]>([]);
  const boardRef = useRef<HTMLDivElement>(null);

  const boardSquares = useMemo(
    () => parseCustomNotation(state?.board ?? EMPTY_BOARD),
    [state?.board],
  );

  // Overlay strings swap pieces for '+'/'-' markers; the previous board state
  // restores the victim piece under each capture ring.
  const squares = useMemo(
    () => (overlay ? parseCustomNotation(overlay, boardSquares) : boardSquares),
    [overlay, boardSquares],
  );

  const applyUpdate = useCallback((update: BoardUpdate) => {
    setState(update);
    setOverlay(null);
    setSelected(null);
    setPieces((prev) => reconcile(prev, parseCustomNotation(update.board)));
  }, []);

  useEffect(() => {
    void client.getInitialState().then(applyUpdate);
  }, [client, applyUpdate]);

  // GSAP: capture burst + check pulse
  useEffect(() => {
    if (!state || !boardRef.current) return;
    const root = boardRef.current;

    if (state.captured) {
      const el = root.querySelector(`[data-square="${state.captured.square}"]`);
      if (el) {
        gsap.fromTo(
          el,
          { boxShadow: "0 0 0 0 oklch(0.65 0.21 25 / 0.9)" },
          { boxShadow: "0 0 0 22px oklch(0.65 0.21 25 / 0)", duration: 0.65, ease: "power2.out" },
        );
      }
    }

    if (state.checkSquare != null) {
      const el = root.querySelector(`[data-square="${state.checkSquare}"]`);
      if (el) {
        gsap.fromTo(
          el,
          { backgroundColor: "oklch(0.62 0.22 25 / 0.85)" },
          {
            backgroundColor: "oklch(0.62 0.22 25 / 0.2)",
            duration: 0.5,
            repeat: 3,
            yoyo: true,
            ease: "sine.inOut",
          },
        );
      }
    }
  }, [state]);

  const handleSquare = async (square: BoardSquare) => {
    if (!state || state.status === "checkmate" || state.status === "stalemate") return;

    if (square.isMoveTarget || square.isCaptureTarget) {
      if (selected == null) return;
      const update = await client.submitMove(selected, square.index);
      setHistory((h) => [
        ...h,
        `${squareName(selected)}${square.isCaptureTarget ? "x" : "-"}${squareName(square.index)}`,
      ]);
      applyUpdate(update);
      return;
    }

    if (selected === square.index) {
      setSelected(null);
      setOverlay(null);
      return;
    }

    const piece = square.piece;
    if (!piece || isWhitePiece(piece) !== (state.sideToMove === "white")) {
      setSelected(null);
      setOverlay(null);
      return;
    }

    const notation = await client.requestLegalMoves(square.index);
    setSelected(square.index);
    setOverlay(notation);
  };

  const statusLabel =
    state?.status === "checkmate"
      ? `Checkmate — ${state.sideToMove === "white" ? "Black" : "White"} wins`
      : state?.status === "stalemate"
        ? "Stalemate"
        : state?.status === "check"
          ? "Check"
          : `${state?.sideToMove === "white" ? "White" : "Black"} to move`;

  return (
    <div className="flex w-full flex-col gap-8 lg:flex-row lg:items-start">
      <div className="flex-1">
        <div
          ref={boardRef}
          className="board-frame relative grid aspect-square w-full grid-cols-8 overflow-hidden rounded-2xl"
        >
          {squares.map((sq) => {
            const row = Math.floor(sq.index / 8);
            const col = sq.index % 8;
            const light = (row + col) % 2 === 0;
            const isLast =
              state?.lastMove &&
              (state.lastMove.from === sq.index || state.lastMove.to === sq.index);
            return (
              <button
                key={sq.index}
                data-square={sq.index}
                onClick={() => void handleSquare(sq)}
                aria-label={`${squareName(sq.index)}${sq.piece ? ` ${sq.piece}` : ""}`}
                className={`relative flex items-center justify-center transition-colors duration-200 ${
                  light ? "bg-board-light" : "bg-board-dark"
                } ${selected === sq.index ? "ring-2 ring-inset ring-accent" : ""}`}
              >
                {isLast && <span className="absolute inset-0 bg-accent/18" />}
                {sq.isMoveTarget && (
                  <motion.span
                    layout
                    initial={{ scale: 0, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    transition={{ type: "spring", stiffness: 420, damping: 24 }}
                    className="pointer-events-none absolute h-[26%] w-[26%] rounded-full bg-accent/70"
                  />
                )}
                {sq.isCaptureTarget && (
                  <motion.span
                    initial={{ scale: 0.6, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    transition={{ type: "spring", stiffness: 380, damping: 20 }}
                    className="pointer-events-none absolute inset-[6%] rounded-full border-4 border-destructive/80"
                  />
                )}
                {col === 0 && (
                  <span className="pointer-events-none absolute left-1 top-0.5 text-[0.6rem] font-medium opacity-45">
                    {8 - row}
                  </span>
                )}
                {row === 7 && (
                  <span className="pointer-events-none absolute bottom-0.5 right-1 text-[0.6rem] font-medium opacity-45">
                    {FILES[col]}
                  </span>
                )}
              </button>
            );
          })}

          {/* Pieces layer: absolutely positioned so Framer Motion interpolates moves */}
          <div className="pointer-events-none absolute inset-0">
            <AnimatePresence>
              {pieces.map((p) => {
                const row = Math.floor(p.square / 8);
                const col = p.square % 8;
                return (
                  <motion.div
                    key={p.id}
                    layoutId={p.id}
                    initial={{ scale: 0.4, opacity: 0 }}
                    animate={{
                      left: `${col * 12.5}%`,
                      top: `${row * 12.5}%`,
                      scale: 1,
                      opacity: 1,
                    }}
                    exit={{ scale: 0.3, opacity: 0, rotate: 25 }}
                    transition={{ type: "spring", stiffness: 260, damping: 22, mass: 0.6 }}
                    className="absolute flex h-[12.5%] w-[12.5%] items-center justify-center p-[1.4%]"
                  >
                    <PieceGlyph piece={p.piece} className="h-full w-full" />
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </div>
        </div>
      </div>

      <aside className="w-full shrink-0 space-y-5 lg:w-72">
        <div className="rounded-xl border border-border bg-card p-5">
          <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Status</p>
          <p className="mt-1 text-lg font-semibold text-card-foreground">{statusLabel}</p>
          <p className="mt-3 break-all font-mono text-[0.68rem] leading-relaxed text-muted-foreground">
            {state?.board}
          </p>
        </div>

        <div className="rounded-xl border border-border bg-card p-5">
          <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">
            Move stream
          </p>
          <ol className="mt-2 max-h-56 space-y-1 overflow-y-auto font-mono text-sm text-card-foreground">
            {history.length === 0 && (
              <li className="text-muted-foreground">Waiting for first move…</li>
            )}
            {history.map((m, i) => (
              <li key={`${m}-${i}`}>
                <span className="mr-2 text-muted-foreground">{i + 1}.</span>
                {m}
              </li>
            ))}
          </ol>
        </div>

        <button
          onClick={async () => {
            setHistory([]);
            applyUpdate(await client.reset());
          }}
          className="w-full rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-transform hover:scale-[1.02] active:scale-95"
        >
          Reset position
        </button>
      </aside>
    </div>
  );
}
