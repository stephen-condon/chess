// Loads the WASM module for the main thread and re-exports the Game class.
// The Game instance here is the synchronous source of truth for the UI: it
// answers legal-move and highlighting queries instantly. AI search runs in a
// separate Worker (see worker.ts).

import init, { Game } from "./wasm/chess_wasm.js";

let ready: Promise<void> | null = null;

export async function loadEngine(): Promise<void> {
  if (!ready) {
    ready = init().then(() => undefined);
  }
  return ready;
}

export { Game };
