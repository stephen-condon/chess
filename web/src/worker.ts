// Web Worker that owns a second WASM instance dedicated to search and
// post-game analysis, so the AI and analyzer never block the UI thread.

import init, { analyze, search } from "./wasm/chess_wasm.js";
import type { GameReport, SearchInfo } from "./types.js";

const ready = init();

export interface SearchRequest {
  id: number;
  kind: "search";
  fen: string;
  timeMs: number;
  maxDepth: number;
}

export interface AnalyzeRequest {
  id: number;
  kind: "analyze";
  pgn: string;
  timeMs: number;
  maxDepth: number;
}

export type WorkerRequest = SearchRequest | AnalyzeRequest;

export interface SearchResponse {
  id: number;
  kind: "search";
  result?: SearchInfo;
  error?: string;
}

export interface AnalyzeProgress {
  done: number;
  total: number;
}

export interface AnalyzeResponse {
  id: number;
  kind: "analyze";
  report?: GameReport;
  progress?: AnalyzeProgress;
  error?: string;
}

export type WorkerResponse = SearchResponse | AnalyzeResponse;

const post = (r: WorkerResponse) => (self as unknown as Worker).postMessage(r);

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const req = e.data;
  try {
    await ready;
    if (req.kind === "analyze") {
      const { id, pgn, timeMs, maxDepth } = req;
      const report = analyze(pgn, timeMs, maxDepth, (done: number, total: number) => {
        post({ id, kind: "analyze", progress: { done, total } });
      }) as GameReport;
      post({ id, kind: "analyze", report });
    } else {
      const { id, fen, timeMs, maxDepth } = req;
      const result = search(fen, timeMs, maxDepth) as SearchInfo;
      post({ id, kind: "search", result });
    }
  } catch (err) {
    post({ id: req.id, kind: req.kind, error: String(err) });
  }
};
