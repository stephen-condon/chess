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
export type Difficulty = "easy" | "medium" | "hard";

export interface DifficultySetting {
  timeMs: number;
  maxDepth: number;
}

export const DIFFICULTIES: Record<Difficulty, DifficultySetting> = {
  easy: { timeMs: 150, maxDepth: 3 },
  medium: { timeMs: 600, maxDepth: 8 },
  hard: { timeMs: 2000, maxDepth: 64 },
};
