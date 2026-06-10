// Renders the 8x8 board as a CSS grid and reports square clicks. Stateless
// about game rules — it only draws what it's told and emits click events.

const GLYPHS: Record<string, string> = {
  k: "♚",
  q: "♛",
  r: "♜",
  b: "♝",
  n: "♞",
  p: "♟",
};

export type SquareClickHandler = (square: number) => void;

export class Board {
  private cells: HTMLElement[] = new Array(64);
  private whiteBottom = true;

  constructor(private root: HTMLElement, private onClick: SquareClickHandler) {
    this.build();
  }

  private build(): void {
    this.root.replaceChildren();
    for (let sq = 0; sq < 64; sq++) {
      const cell = document.createElement("div");
      cell.className = "square";
      const file = sq & 7;
      const rank = sq >> 3;
      cell.classList.add((file + rank) % 2 === 0 ? "dark" : "light");
      cell.addEventListener("click", () => this.onClick(sq));
      this.cells[sq] = cell;
      this.root.appendChild(cell);
    }
    this.layout();
  }

  private layout(): void {
    for (let sq = 0; sq < 64; sq++) {
      const file = sq & 7;
      const rank = sq >> 3;
      const row = this.whiteBottom ? 8 - rank : rank + 1;
      const col = this.whiteBottom ? file + 1 : 8 - file;
      const cell = this.cells[sq];
      cell.style.gridRow = String(row);
      cell.style.gridColumn = String(col);
    }
  }

  setOrientation(whiteBottom: boolean): void {
    this.whiteBottom = whiteBottom;
    this.layout();
  }

  /** Draw pieces from a 64-char board string (FEN letters, '.' for empty). */
  render(board: string): void {
    for (let sq = 0; sq < 64; sq++) {
      const ch = board[sq];
      const cell = this.cells[sq];
      const piece = cell.querySelector(".piece") as HTMLElement | null;
      if (ch === ".") {
        if (piece) piece.remove();
        continue;
      }
      const glyph = GLYPHS[ch.toLowerCase()];
      const colorClass = ch === ch.toUpperCase() ? "white" : "black";
      if (piece) {
        piece.textContent = glyph;
        piece.className = `piece ${colorClass}`;
      } else {
        const el = document.createElement("span");
        el.className = `piece ${colorClass}`;
        el.textContent = glyph;
        cell.appendChild(el);
      }
    }
  }

  clearHighlights(): void {
    for (const cell of this.cells) {
      cell.classList.remove("selected", "target", "capture", "last", "check");
    }
  }

  setSelected(square: number | null): void {
    if (square !== null) this.cells[square].classList.add("selected");
  }

  /** Mark legal destinations; `board` tells us which are captures. */
  setTargets(squares: number[], board: string): void {
    for (const sq of squares) {
      this.cells[sq].classList.add(board[sq] === "." ? "target" : "capture");
    }
  }

  setLastMove(from: number, to: number): void {
    this.cells[from].classList.add("last");
    this.cells[to].classList.add("last");
  }

  setCheck(square: number | null): void {
    if (square !== null) this.cells[square].classList.add("check");
  }
}
