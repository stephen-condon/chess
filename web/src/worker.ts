// Web Worker that owns a second WASM instance dedicated to search, so the AI
// never blocks the UI thread. It receives a position (FEN) plus time/depth
// limits and posts back the best move.

import init, { search } from "./wasm/chess_wasm.js";
import type { SearchInfo } from "./types.js";

const ready = init();

export interface SearchRequest {
  id: number;
  fen: string;
  timeMs: number;
  maxDepth: number;
}

export interface SearchResponse {
  id: number;
  result?: SearchInfo;
  error?: string;
}

const post = (r: SearchResponse) => (self as unknown as Worker).postMessage(r);

self.onmessage = async (e: MessageEvent<SearchRequest>) => {
  const { id, fen, timeMs, maxDepth } = e.data;
  try {
    await ready;
    const result = search(fen, timeMs, maxDepth) as SearchInfo;
    post({ id, result });
  } catch (err) {
    post({ id, error: String(err) });
  }
};
