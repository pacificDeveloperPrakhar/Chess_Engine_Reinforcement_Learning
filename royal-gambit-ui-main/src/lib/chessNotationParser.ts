export interface BoardSquare {
  index: number; // 0 to 63
  rank: number; // 1 to 8
  file: string; // 'a' to 'h'
  piece: string | null; // 'P', 'p', 'N', etc.
  isMoveTarget: boolean; // true if '+'
  isCaptureTarget: boolean; // true if '-'
}

export function parseCustomNotation(
  notation: string,
  previousBoardState?: BoardSquare[],
): BoardSquare[] {
  const squares: BoardSquare[] = [];
  const rows = notation.split("/");

  let globalIndex = 0;
  for (let r = 0; r < 8; r++) {
    const rowStr = rows[r] ?? "8";
    const rank = 8 - r;

    let fileIndex = 0;
    for (let c = 0; c < rowStr.length; c++) {
      const char = rowStr[c] as string;

      if (char >= "1" && char <= "8") {
        const emptyCount = parseInt(char, 10);
        for (let i = 0; i < emptyCount; i++) {
          const file = String.fromCharCode("a".charCodeAt(0) + fileIndex);
          squares.push({
            index: globalIndex,
            rank,
            file,
            piece: null,
            isMoveTarget: false,
            isCaptureTarget: false,
          });
          globalIndex++;
          fileIndex++;
        }
      } else {
        const file = String.fromCharCode("a".charCodeAt(0) + fileIndex);
        const isMove = char === "+";
        const isCapture = char === "-";

        let existingPiece: string | null = null;
        const previousSquare = previousBoardState?.[globalIndex];
        if (isCapture && previousSquare) {
          existingPiece = previousSquare.piece;
        } else if (!isMove && !isCapture) {
          existingPiece = char;
        }

        squares.push({
          index: globalIndex,
          rank,
          file,
          piece: existingPiece,
          isMoveTarget: isMove,
          isCaptureTarget: isCapture,
        });
        globalIndex++;
        fileIndex++;
      }
    }
  }
  return squares;
}

/** Flat 64-char array ('.' = empty) from a board string (no overlay markers). */
export function fenToArray(fenString: string): string[] {
  const board: string[] = [];
  for (const row of fenString.split("/")) {
    for (const char of row) {
      if (char >= "1" && char <= "8") {
        for (let i = 0; i < parseInt(char, 10); i++) board.push(".");
      } else {
        board.push(char);
      }
    }
  }
  while (board.length < 64) board.push(".");
  return board.slice(0, 64);
}

/** Inverse of fenToArray: RLE-compress a 64-cell array back to a board string. */
export function arrayToFen(board: string[]): string {
  const rows: string[] = [];
  for (let r = 0; r < 8; r++) {
    let row = "";
    let empty = 0;
    for (let c = 0; c < 8; c++) {
      const cell = board[r * 8 + c] ?? ".";
      if (cell === "." || cell === "") {
        empty++;
      } else {
        if (empty) {
          row += String(empty);
          empty = 0;
        }
        row += cell;
      }
    }
    if (empty) row += String(empty);
    rows.push(row);
  }
  return rows.join("/");
}

export const squareName = (index: number) =>
  `${String.fromCharCode(97 + (index % 8))}${8 - Math.floor(index / 8)}`;

export const isWhitePiece = (piece: string) => piece >= "A" && piece <= "Z";

export const START_POSITION = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
