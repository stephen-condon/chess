// Owns a dedicated Web Worker for post-game analysis, separate from the AI
// search worker so a long analysis run never competes with AI move search.

import type { AnalyzeRequest, AnalyzeResponse } from "./worker.js";
import type { GameReport } from "./types.js";

let worker: Worker | null = null;
let nextId = 0;

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  }
  return worker;
}

export interface AnalyzeOptions {
  timeMs: number;
  maxDepth: number;
}

/**
 * Analyze a finished game given as PGN. `onProgress` is called with
 * `(done, total)` once per searched position.
 */
export function analyzeGame(
  pgn: string,
  { timeMs, maxDepth }: AnalyzeOptions,
  onProgress?: (done: number, total: number) => void,
): Promise<GameReport> {
  const w = getWorker();
  const id = ++nextId;

  return new Promise((resolve, reject) => {
    const onMessage = (e: MessageEvent<AnalyzeResponse>) => {
      const msg = e.data;
      if (msg.kind !== "analyze" || msg.id !== id) return;
      if (msg.progress) {
        onProgress?.(msg.progress.done, msg.progress.total);
        return;
      }
      w.removeEventListener("message", onMessage);
      if (msg.error || !msg.report) {
        reject(new Error(msg.error ?? "no report"));
        return;
      }
      resolve(msg.report);
    };
    w.addEventListener("message", onMessage);

    const req: AnalyzeRequest = { id, kind: "analyze", pgn, timeMs, maxDepth };
    w.postMessage(req);
  });
}
