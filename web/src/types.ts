// Shapes returned by the WASM bindings (see wasm/src/lib.rs).

export interface MoveInfo {
  from: number;
  to: number;
  uci: string;
  san: string;
  promotion: string | null;
}

export interface StatusInfo {
  state: "ongoing" | "checkmate" | "stalemate" | "draw";
  winner: "white" | "black" | null;
  reason: "fifty-move" | "repetition" | "insufficient-material" | null;
  inCheck: boolean;
  turn: "white" | "black";
}

export interface MoveResult {
  san: string;
  fen: string;
  status: StatusInfo;
}

export interface SearchInfo {
  bestMove: string | null;
  score: number;
  depth: number;
  nodes: number;
  pv: string[];
}

export type Mode = "hotseat" | "computer";
export type Side = "white" | "black";
export type Difficulty = "very-easy" | "easy" | "medium" | "hard";

export interface GameConfig {
  mode: Mode;
  humanSide: Side;
  difficulty: Difficulty;
}

export interface DifficultySetting {
  timeMs: number;
  maxDepth: number;
}

export const DIFFICULTIES: Record<Difficulty, DifficultySetting> = {
  "very-easy": { timeMs: 50, maxDepth: 1 },
  easy: { timeMs: 150, maxDepth: 3 },
  medium: { timeMs: 600, maxDepth: 8 },
  hard: { timeMs: 2000, maxDepth: 64 },
};

export type MoveClass = "best" | "good" | "inaccuracy" | "mistake" | "blunder";

export interface AnalyzedMove {
  ply: number;
  color: "white" | "black";
  san: string;
  bestSan: string;
  evalBefore: number;
  evalAfter: number;
  cpl: number;
  class: MoveClass;
  turningPoint: boolean;
  decidedGame: boolean;
  pvSan: string[];
}

export interface SideSummary {
  accuracy: number;
  avgCpl: number;
  best: number;
  inaccuracies: number;
  mistakes: number;
  blunders: number;
}

export interface GameReport {
  moves: AnalyzedMove[];
  white: SideSummary;
  black: SideSummary;
  result: string;
  annotatedPgn: string;
}

export type AnalysisSpeed = "fast" | "balanced" | "deep";

export const ANALYSIS_SPEEDS: Record<AnalysisSpeed, DifficultySetting> = {
  fast: { timeMs: 100, maxDepth: 8 },
  balanced: { timeMs: 250, maxDepth: 12 },
  deep: { timeMs: 750, maxDepth: 20 },
};
