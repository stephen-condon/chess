import { Board } from "./board.js";
import { analyzeGame } from "./analysis.js";
import { loadEngine, Game } from "./engine.js";
import {
  ANALYSIS_SPEEDS,
  DIFFICULTIES,
  type AnalysisSpeed,
  type Difficulty,
  type GameReport,
  type Mode,
  type MoveResult,
  type Side,
  type StatusInfo,
} from "./types.js";
import type { SearchRequest, WorkerResponse } from "./worker.js";

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
const analyzeBtn = $<HTMLButtonElement>("analyze-game");
const analysisProgressEl = $("analysis-progress");
const analysisSummaryEl = $("analysis-summary");

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
let analyzing = false;
let analysisSpeed: AnalysisSpeed = "balanced";
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
  analyzeBtn.disabled = analyzing || thinking || status.state === "ongoing";
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
  const req: SearchRequest = { id: searchId, kind: "search", fen: game.fen(), timeMs, maxDepth };
  worker.postMessage(req);
}

function onWorkerMessage(e: MessageEvent<WorkerResponse>): void {
  if (e.data.kind === "analyze") return; // this worker is only used for search
  const { id, result, error } = e.data;
  if (id !== searchId) return; // stale (e.g. New Game pressed mid-search)
  thinking = false;
  if (error || !result) {
    showError("AI error: " + (error ?? "no result"));
    render();
    return;
  }
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

// --- Post-game analysis -----------------------------------------------------

function resetAnalysisUI(): void {
  analyzing = false;
  analysisProgressEl.hidden = true;
  analysisProgressEl.textContent = "";
  analysisSummaryEl.hidden = true;
  analysisSummaryEl.replaceChildren();
}

async function runAnalysis(): Promise<void> {
  if (analyzing || thinking) return;
  analyzing = true;
  analysisSummaryEl.hidden = true;
  analysisSummaryEl.replaceChildren();
  analysisProgressEl.hidden = false;
  analysisProgressEl.textContent = "Analyzing…";
  render();

  try {
    const { timeMs, maxDepth } = ANALYSIS_SPEEDS[analysisSpeed];
    const report = await analyzeGame(game.toPgn(), { timeMs, maxDepth }, (done, total) => {
      analysisProgressEl.textContent = `Analyzing… ${done}/${total}`;
    });
    $<HTMLTextAreaElement>("pgn").value = report.annotatedPgn;
    renderAnalysisSummary(report);
  } catch (e) {
    showError(String(e));
  } finally {
    analyzing = false;
    analysisProgressEl.hidden = true;
    render();
  }
}

function renderAnalysisSummary(report: GameReport): void {
  analysisSummaryEl.replaceChildren();

  const table = document.createElement("table");
  const header = table.insertRow();
  for (const text of ["", "White", "Black"]) {
    const th = document.createElement("th");
    th.textContent = text;
    header.appendChild(th);
  }

  const rows: [string, string, string][] = [
    ["Accuracy", `${report.white.accuracy.toFixed(1)}%`, `${report.black.accuracy.toFixed(1)}%`],
    ["Avg. CPL", `${report.white.avgCpl}`, `${report.black.avgCpl}`],
    ["Inaccuracies", `${report.white.inaccuracies}`, `${report.black.inaccuracies}`],
    ["Mistakes", `${report.white.mistakes}`, `${report.black.mistakes}`],
    ["Blunders", `${report.white.blunders}`, `${report.black.blunders}`],
  ];
  for (const [label, white, black] of rows) {
    const row = table.insertRow();
    row.insertCell().textContent = label;
    row.insertCell().textContent = white;
    row.insertCell().textContent = black;
  }
  analysisSummaryEl.appendChild(table);

  const decisive = report.moves.find((m) => m.decidedGame);
  const note = document.createElement("p");
  note.className = "decisive";
  if (decisive) {
    const moveNo = Math.floor(decisive.ply / 2) + 1;
    const side = decisive.color === "white" ? "White" : "Black";
    note.textContent = `Decided by ${side}'s ${decisive.san} (move ${moveNo}).`;
  } else {
    note.textContent = "No single move decided the game.";
  }
  analysisSummaryEl.appendChild(note);

  analysisSummaryEl.hidden = false;
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
  resetAnalysisUI();
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
  resetAnalysisUI();
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

  const speedSel = $<HTMLSelectElement>("analysis-speed");
  speedSel.addEventListener("change", () => {
    analysisSpeed = speedSel.value as AnalysisSpeed;
  });
  analyzeBtn.addEventListener("click", () => {
    void runAnalysis();
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
  worker.onerror = (e) => {
    thinking = false;
    showError("AI worker error: " + (e.message || "unknown"));
    render();
  };
  wireControls();
  render();
}

void boot();
