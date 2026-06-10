import { Board } from "./board.js";
import { loadEngine, Game } from "./engine.js";
import {
  DIFFICULTIES,
  type Difficulty,
  type Mode,
  type MoveResult,
  type Side,
  type StatusInfo,
} from "./types.js";
import type { SearchRequest, SearchResponse } from "./worker.js";

// --- DOM helpers -----------------------------------------------------------

const $ = <T extends HTMLElement>(id: string): T => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el as T;
};

const boardEl = $("board");
const statusEl = $("status");
const movesEl = $("moves");
const ioError = $("io-error");
const promotionEl = $("promotion");

// --- App state -------------------------------------------------------------

let game: Game;
let board: Board;
let worker: Worker;

let mode: Mode = "hotseat";
let humanSide: Side = "white";
let difficulty: Difficulty = "medium";
let whiteBottom = true;

let selected: number | null = null;
let targets: number[] = [];
let lastMove: { from: number; to: number } | null = null;
let pendingPromotion: { from: number; to: number } | null = null;
let thinking = false;
let searchId = 0;

// --- Square helpers --------------------------------------------------------

const squareFromName = (name: string): number => {
  const file = name.charCodeAt(0) - 97;
  const rank = name.charCodeAt(1) - 49;
  return rank * 8 + file;
};

// --- Rendering -------------------------------------------------------------

function render(): void {
  const boardStr = game.board();
  board.render(boardStr);
  board.clearHighlights();

  if (lastMove) board.setLastMove(lastMove.from, lastMove.to);
  if (selected !== null) {
    board.setSelected(selected);
    board.setTargets(targets, boardStr);
  }

  const status = game.status() as StatusInfo;
  if (status.inCheck) {
    const kingChar = status.turn === "white" ? "K" : "k";
    const kingSq = boardStr.indexOf(kingChar);
    if (kingSq >= 0) board.setCheck(kingSq);
  }

  renderStatus(status);
  renderMoves();
}

function renderStatus(status: StatusInfo): void {
  let text: string;
  switch (status.state) {
    case "checkmate":
      text = `Checkmate — ${status.winner} wins`;
      break;
    case "stalemate":
      text = "Draw — stalemate";
      break;
    case "draw":
      text = `Draw — ${status.reason?.replace("-", " ")}`;
      break;
    default:
      if (thinking) {
        text = "Computer is thinking…";
      } else {
        const turn = status.turn === "white" ? "White" : "Black";
        text = `${turn} to move${status.inCheck ? " — check" : ""}`;
      }
  }
  statusEl.textContent = text;
}

function renderMoves(): void {
  const sans = game.historySan();
  movesEl.replaceChildren();
  for (let i = 0; i < sans.length; i += 2) {
    const li = document.createElement("li");
    const white = sans[i];
    const black = sans[i + 1] ?? "";
    li.textContent = `${white}  ${black}`;
    movesEl.appendChild(li);
  }
  movesEl.scrollTop = movesEl.scrollHeight;
}

// --- Move application ------------------------------------------------------

function isPromotion(from: number, to: number, boardStr: string): boolean {
  const piece = boardStr[from].toLowerCase();
  const toRank = to >> 3;
  return piece === "p" && (toRank === 7 || toRank === 0);
}

function applyMove(from: number, to: number, promo?: string): void {
  let result: MoveResult;
  try {
    result = game.makeMove(from, to, promo) as MoveResult;
  } catch (e) {
    showError(String(e));
    return;
  }
  lastMove = { from, to };
  selected = null;
  targets = [];
  render();

  if (result.status.state === "ongoing" && mode === "computer") {
    if (result.status.turn !== humanSide) triggerAI();
  }
}

// --- AI via worker ---------------------------------------------------------

function triggerAI(): void {
  thinking = true;
  renderStatus(game.status() as StatusInfo);
  const { timeMs, maxDepth } = DIFFICULTIES[difficulty];
  searchId += 1;
  const req: SearchRequest = { id: searchId, fen: game.fen(), timeMs, maxDepth };
  worker.postMessage(req);
}

function onWorkerMessage(e: MessageEvent<SearchResponse>): void {
  const { id, result } = e.data;
  if (id !== searchId) return; // stale (e.g. New Game pressed mid-search)
  thinking = false;
  if (!result.bestMove) {
    render();
    return;
  }
  const from = squareFromName(result.bestMove.slice(0, 2));
  const to = squareFromName(result.bestMove.slice(2, 4));
  try {
    game.makeUci(result.bestMove);
  } catch (err) {
    showError(String(err));
    return;
  }
  lastMove = { from, to };
  render();
}

// --- Click handling --------------------------------------------------------

function onSquareClick(sq: number): void {
  if (thinking || pendingPromotion) return;
  if (mode === "computer" && (game.status() as StatusInfo).turn !== humanSide) {
    return;
  }

  const boardStr = game.board();
  const turn = (game.status() as StatusInfo).turn;
  const pieceHere = boardStr[sq];
  const isOwnPiece =
    pieceHere !== "." &&
    (turn === "white"
      ? pieceHere === pieceHere.toUpperCase()
      : pieceHere === pieceHere.toLowerCase());

  if (selected === null) {
    if (isOwnPiece) selectSquare(sq);
    return;
  }

  if (sq === selected) {
    deselect();
    return;
  }

  if (targets.includes(sq)) {
    if (isPromotion(selected, sq, boardStr)) {
      pendingPromotion = { from: selected, to: sq };
      promotionEl.hidden = false;
      return;
    }
    applyMove(selected, sq);
    return;
  }

  if (isOwnPiece) {
    selectSquare(sq);
  } else {
    deselect();
  }
}

function selectSquare(sq: number): void {
  selected = sq;
  targets = Array.from(game.legalDestinations(sq));
  render();
}

function deselect(): void {
  selected = null;
  targets = [];
  render();
}

// --- Controls --------------------------------------------------------------

function newGame(): void {
  searchId += 1; // invalidate any in-flight search
  thinking = false;
  game = new Game();
  selected = null;
  targets = [];
  lastMove = null;
  pendingPromotion = null;
  promotionEl.hidden = true;
  render();
  if (mode === "computer" && humanSide === "black") triggerAI();
}

function loadFromFen(fen: string): void {
  try {
    game = Game.fromFen(fen);
  } catch (e) {
    showError(String(e));
    return;
  }
  resetAfterLoad();
}

function loadFromPgn(pgn: string): void {
  try {
    game = Game.fromPgn(pgn);
  } catch (e) {
    showError(String(e));
    return;
  }
  resetAfterLoad();
}

function resetAfterLoad(): void {
  searchId += 1;
  thinking = false;
  selected = null;
  targets = [];
  lastMove = null;
  pendingPromotion = null;
  promotionEl.hidden = true;
  render();
  if (mode === "computer" && (game.status() as StatusInfo).turn !== humanSide) {
    triggerAI();
  }
}

function undoMove(): void {
  if (thinking) return;
  if (!game.undo()) return;
  // In computer mode, step back past the AI reply too, so it's the human's turn.
  if (mode === "computer" && (game.status() as StatusInfo).turn !== humanSide) {
    game.undo();
  }
  selected = null;
  targets = [];
  lastMove = null;
  render();
}

function showError(msg: string): void {
  ioError.textContent = msg;
  window.setTimeout(() => {
    if (ioError.textContent === msg) ioError.textContent = "";
  }, 4000);
}

// --- Wiring ----------------------------------------------------------------

function wireControls(): void {
  $("new-game").addEventListener("click", newGame);
  $("flip").addEventListener("click", () => {
    whiteBottom = !whiteBottom;
    board.setOrientation(whiteBottom);
    render();
  });
  $("undo").addEventListener("click", undoMove);

  const modeSel = $<HTMLSelectElement>("mode");
  modeSel.addEventListener("change", () => {
    mode = modeSel.value as Mode;
    $("side-row").hidden = mode !== "computer";
    $("difficulty-row").hidden = mode !== "computer";
    newGame();
  });

  const sideSel = $<HTMLSelectElement>("human-side");
  sideSel.addEventListener("change", () => {
    humanSide = sideSel.value as Side;
    whiteBottom = humanSide === "white";
    board.setOrientation(whiteBottom);
    newGame();
  });

  const diffSel = $<HTMLSelectElement>("difficulty");
  diffSel.addEventListener("change", () => {
    difficulty = diffSel.value as Difficulty;
  });

  $("load-fen").addEventListener("click", () => {
    loadFromFen($<HTMLInputElement>("fen").value.trim());
  });
  $("copy-fen").addEventListener("click", () => {
    void navigator.clipboard.writeText(game.fen());
  });
  $("load-pgn").addEventListener("click", () => {
    loadFromPgn($<HTMLTextAreaElement>("pgn").value);
  });
  $("export-pgn").addEventListener("click", () => {
    $<HTMLTextAreaElement>("pgn").value = game.toPgn();
  });

  for (const btn of promotionEl.querySelectorAll<HTMLButtonElement>(".promo-btn")) {
    btn.addEventListener("click", () => {
      if (!pendingPromotion) return;
      const piece = btn.dataset.piece!;
      const { from, to } = pendingPromotion;
      pendingPromotion = null;
      promotionEl.hidden = true;
      applyMove(from, to, piece);
    });
  }
}

// --- Boot ------------------------------------------------------------------

async function boot(): Promise<void> {
  await loadEngine();
  game = new Game();
  board = new Board(boardEl, onSquareClick);
  worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker.onmessage = onWorkerMessage;
  wireControls();
  render();
}

void boot();
