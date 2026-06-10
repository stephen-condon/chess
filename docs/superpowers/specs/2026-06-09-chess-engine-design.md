# Chess Game — Rust Engine + WASM/TS Frontend (MVP) — Design

Status: approved 2026-06-09. Full multi-altitude plan archived at
`~/.claude/plans/do-several-rounds-of-indexed-stearns.md`.

## Goal

Playable browser chess game with local **hotseat** and **human-vs-computer**
modes and a strong-ish engine opponent. No networked multiplayer in the MVP.

## Key decisions

| Decision | Choice |
|---|---|
| Core engine | Pure Rust crate (`engine`), no WASM deps, natively unit-testable |
| Board representation | Bitboards (`u64`) + magic bitboards for sliders |
| Computer opponent | Negamax + α-β + iterative deepening + transposition table + quiescence + move ordering |
| WASM boundary | Separate `wasm` crate wrapping `engine` via `wasm-bindgen` |
| Frontend | Vanilla TypeScript + Vite |
| Board UI | CSS grid, SVG pieces, click-to-move |
| AI threading | Web Worker (separate engine instance); main thread keeps a synchronous instance for instant legal-move highlighting |
| MVP extras | SAN move history, legal-move highlighting + check indicator, FEN/PGN import-export |
| Deferred | Undo/redo, drag-and-drop, opening book, multi-threaded search, online play |

## Architecture

Three layers in a Cargo workspace + a Vite web app:

- `engine/` — pure Rust core (bitboards, movegen, make/unmake, Zobrist, search,
  eval, FEN/SAN/PGN). The source of truth for rules and AI.
- `wasm/` — thin `wasm-bindgen` adapter exposing a `Game` class + `search` fn;
  marshals via UCI/FEN/PGN strings (richer structs via `serde-wasm-bindgen`).
- `web/` — vanilla TS + Vite. Main thread holds a synchronous `Game` instance
  for highlighting/validation; a Web Worker holds a second instance that runs the
  search, so the UI never freezes. Position shipped to the worker as FEN; the
  worker returns a best move (UCI).

## Engine module map (`engine/src/`)

`types`, `bitboard`, `attacks` (knight/king/pawn), `magic` (rook/bishop sliders),
`zobrist`, `position` (board state + make/unmake), `moves` (16-bit packed),
`movegen` (pseudo-legal + legality filter), `fen`, `san`, `pgn`, `rules`
(status/draws), `eval`, `tt`, `search`, `lib` (`Game` façade + `perft`).

Move generation: pseudo-legal generation, then filter by make/unmake +
king-safety test. Correctness verified by **perft** against known node counts
(startpos, Kiwipete, CPW positions 3/4/5).

## Computer-opponent pattern (chosen + alternatives)

Chosen: classical alpha-beta search (negamax) with iterative deepening, a
Zobrist-keyed transposition table, quiescence search, and move ordering (TT move
→ MVV-LVA captures → killer moves → history heuristic). Deterministic,
debuggable, no training data, fast single-threaded on WASM. Difficulty = a dial
mapped to think-time + optional depth cap (Easy ~100 ms/d≤3, Medium ~500 ms,
Hard ~2000 ms).

Considered and deferred/rejected: random/greedy (too weak, kept only as a test
baseline), null-move/LMR/aspiration (post-MVP strength dial), opening book
(optional polish), Syzygy tablebases (out of scope), MCTS and NNUE/neural eval
(out of scope; the eval interface stays swappable so a net could slot in later).

## Milestones (each independently verifiable)

- M1 engine core → perft passes.
- M2 rules + SAN + PGN → unit tests + round-trips.
- M3 search + eval → mate-in-N, eval symmetry, native self-play.
- M4 wasm bindings → node smoke test of API shapes.
- M5 frontend hotseat → manual full hotseat game.
- M6 AI via worker + FEN/PGN UI + difficulty → full vs-CPU game, no UI freeze.

## Out of MVP scope (YAGNI)

Online/networked play, undo/redo UI, drag-and-drop, opening book, endgame
tablebases, NNUE eval, multi-threaded search, animations, sound, persistence.
