// Node smoke test for the WASM bindings (built with --target nodejs).
// Run: node wasm/smoke.cjs
const wasm = require("./pkg-node/chess_wasm.js");

let failures = 0;
function check(cond, msg) {
  if (!cond) {
    console.error("FAIL:", msg);
    failures++;
  } else {
    console.log("ok:", msg);
  }
}

const g = new wasm.Game();

const board = g.board();
check(board.length === 64, "board is 64 chars");
check(board[12] === "P", "e2 has a white pawn"); // e2 = index 12

const moves = g.legalMoves();
check(Array.isArray(moves) && moves.length === 20, "20 legal moves at start");
check(
  moves.some((m) => m.uci === "e2e4" && m.san === "e4"),
  "e2e4 present with SAN e4"
);

const dests = Array.from(g.legalDestinations(12));
check(dests.includes(28) && dests.includes(20), "e2 reaches e3 and e4");

const res = g.makeMove(12, 28, undefined); // e2 -> e4
check(res.san === "e4", "makeMove returns SAN e4");
check(res.status.turn === "black", "turn flips to black");
check(typeof res.fen === "string" && res.fen.includes(" b "), "fen shows black to move");

const search = wasm.search(g.fen(), 300, 6);
check(typeof search.bestMove === "string" && search.bestMove.length >= 4, "search returns a bestMove");
check(search.depth >= 1 && search.nodes > 0, "search reports depth and nodes");
check(Array.isArray(search.pv), "search returns a pv array");

// Round-trip via PGN.
g.makeUci("e7e5");
const pgn = g.toPgn();
const g2 = wasm.Game.fromPgn(pgn);
check(JSON.stringify(g2.historySan()) === JSON.stringify(g.historySan()), "PGN round-trip preserves SAN history");

console.log(failures === 0 ? "\nALL SMOKE CHECKS PASSED" : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
