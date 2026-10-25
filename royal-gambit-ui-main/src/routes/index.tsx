import { createFileRoute } from "@tanstack/react-router";

import { ChessBoard } from "@/components/chess/ChessBoard";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "Bitboard — Interactive Chess Board UI" },
      {
        name: "description",
        content:
          "An animated chess board that renders board state and legal-move overlays streamed from a Rust engine.",
      },
      { property: "og:title", content: "Bitboard — Interactive Chess Board UI" },
      {
        property: "og:description",
        content:
          "Animated chess board driven by FEN-style board strings and legal-move overlay notation.",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary_large_image" },
    ],
  }),
  component: Index,
});

function Index() {
  return (
    <main className="min-h-screen bg-background px-5 py-10 md:px-10">
      <div className="mx-auto max-w-6xl">
        <header className="mb-8 flex flex-wrap items-end justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-muted-foreground">
              Rust engine · streamed board state
            </p>
            <h1 className="mt-2 font-display text-4xl font-semibold tracking-tight text-foreground md:text-5xl">
              Bitboard
            </h1>
          </div>
          <p className="max-w-sm text-sm leading-relaxed text-muted-foreground">
            Click a piece to request its legal moves. Dots mark quiet squares, rings mark
            captures — both decoded from the engine's overlay notation.
          </p>
        </header>

        <ChessBoard />
      </div>
    </main>
  );
}
