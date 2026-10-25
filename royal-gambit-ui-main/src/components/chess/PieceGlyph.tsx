import { isWhitePiece } from "@/lib/chessNotationParser";

/** Minimal geometric SVG chess pieces, drawn with design-system colors. */
const PATHS: Record<string, React.ReactNode> = {
  p: (
    <>
      <circle cx="22" cy="12" r="5.6" />
      <path d="M16.6 19h10.8l2.4 8.5h-15.6z" />
      <path d="M12.5 35c0-4.6 3-6.5 9.5-7.5 6.5 1 9.5 2.9 9.5 7.5z" />
      <rect x="11" y="34.5" width="22" height="4" rx="1.6" />
    </>
  ),
  n: (
    <>
      <path d="M14 34c0-7 2.5-10.5 7-13l-2.2-3.4 3-1.2-1-4.4 5 2.4 2.6-3.4c4.6 3.4 6.6 8.2 6.6 14.4V34z" />
      <rect x="11" y="34" width="22" height="4.5" rx="1.6" />
    </>
  ),
  b: (
    <>
      <circle cx="22" cy="9.5" r="3" />
      <path d="M22 12.5c5.4 4 8 8 8 11.8 0 3.6-3.6 5.7-8 5.7s-8-2.1-8-5.7c0-3.8 2.6-7.8 8-11.8z" />
      <rect x="15" y="30" width="14" height="3.4" rx="1.4" />
      <rect x="11" y="34.5" width="22" height="4" rx="1.6" />
    </>
  ),
  r: (
    <>
      <path d="M11 8h5v4h4V8h4v4h4V8h5v8l-3 3v12l3 3.5H11L14 31V19z" />
      <rect x="9.5" y="34.5" width="25" height="4" rx="1.6" />
    </>
  ),
  q: (
    <>
      <circle cx="22" cy="7" r="2.8" />
      <circle cx="11" cy="12" r="2.6" />
      <circle cx="33" cy="12" r="2.6" />
      <path d="M11 14l4 16h14l4-16-6.5 7L22 10l-4.5 11z" />
      <rect x="13" y="30.5" width="18" height="3.4" rx="1.4" />
      <rect x="10" y="34.5" width="24" height="4" rx="1.6" />
    </>
  ),
  k: (
    <>
      <rect x="20.4" y="3" width="3.2" height="10" rx="1" />
      <rect x="17" y="6" width="10" height="3.2" rx="1" />
      <path d="M22 13.5c6.5 2.5 10 6.4 10 10.6 0 4-4.2 6.4-10 6.4s-10-2.4-10-6.4c0-4.2 3.5-8.1 10-10.6z" />
      <rect x="13" y="30.5" width="18" height="3.4" rx="1.4" />
      <rect x="10" y="34.5" width="24" height="4" rx="1.6" />
    </>
  ),
};

export function PieceGlyph({ piece, className }: { piece: string; className?: string }) {
  const white = isWhitePiece(piece);
  return (
    <svg
      viewBox="0 0 44 42"
      className={className}
      aria-hidden="true"
      style={{
        fill: white ? "var(--piece-white)" : "var(--piece-black)",
        stroke: white ? "var(--piece-white-edge)" : "var(--piece-black-edge)",
        strokeWidth: 1.1,
        strokeLinejoin: "round",
        filter: "drop-shadow(0 2px 3px oklch(0.2 0.03 60 / 0.45))",
      }}
    >
      {PATHS[piece.toLowerCase()]}
    </svg>
  );
}
