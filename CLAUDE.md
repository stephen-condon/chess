# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Required Skills

Before **any** change (planned or implemented), you **must** invoke:
- `git-workflow` skill — for all git operations and multi-part requests
- `ponytail` skill — for all implementation work

## Project Overview

Rust chess engine compiled to WebAssembly, paired with a Vanilla TypeScript + Vite frontend. Three-layer architecture:

- `engine/` — Pure-Rust core (no WASM deps). Bitboard move generation, alpha-beta search, FEN/SAN/PGN I/O, post-game analysis.
- `wasm/` — `wasm-bindgen` wrapper exposing `Game` class and `search()`/`analyze()` functions to JS.
- `web/` — Vite + TypeScript frontend. Two Web Workers: one for AI search, one for post-game analysis.

## Commands

### Rust

```bash
cargo fmt                                                    # format
cargo clippy --workspace --all-targets -- -D warnings       # lint (must pass clean)
cargo test --release                                         # all tests (skip perft by default in pre-commit)
cargo test --release -p chess-engine -- --skip perft_       # tests excluding perft (fast)
cargo test --release perft -- --exact --nocapture            # run a specific perft test
```

### WASM

```bash
wasm-pack build wasm --target nodejs --out-dir pkg-node     # build for Node smoke test
node wasm/smoke.cjs                                          # smoke test WASM output
wasm-pack build wasm --target web --out-dir web/src/wasm    # build for browser (or: cd web && npm run wasm)
```

### Web

```bash
cd web && npm install
cd web && npm run dev        # dev server (builds WASM first)
cd web && npm run build      # production build
cd web && npm run e2e        # Playwright E2E tests (requires built app)
cd web && npm run typecheck   # TypeScript type check
```

## CI Pipeline

Three parallel jobs defined in `.github/workflows/ci.yml`, triggered on PRs to `main`:

| Job | Depends on | What it does |
|-----|------------|--------------|
| `rust` | — | `cargo fmt --check`, `cargo clippy`, `cargo test --release --workspace` |
| `wasm` | — | Builds WASM to Node target, runs `wasm/smoke.cjs` |
| `web` | `wasm` | `npm run build`, Playwright E2E against Vite preview |

**Caching:**
- Rust compilation: `Swatinem/rust-cache@v2` (all jobs)
- npm: via `actions/setup-node@v4` cache keyed on `web/package-lock.json`
- wasm-pack binary: custom cache keyed on `wasm-pack-${{ runner.os }}-0.13.1`
- Playwright browsers: keyed on `web/package-lock.json`

The `wasm-pack` version is pinned at **0.13.1** — do not change it without updating both the cache key and the install step.

## Architecture Notes

### Two WASM instances at runtime

The browser loads WASM twice: once on the main thread (synchronous `Game` for board state/UI) and once inside each Web Worker (blocking `search()` / `analyze()`). This is intentional — the engine is not thread-safe via shared state; isolation via workers is the design.

### Data boundary

Moves cross the WASM boundary as **UCI strings** (e.g. `"e2e4"`). Positions cross as **FEN** or **PGN** strings. The engine never passes Rust structs directly to JS.

### Engine public API

`engine/src/lib.rs` re-exports the full public surface. The key types are `Game`, `Position`, `search()`, `SearchLimits`, `analyze()`, and `GameReport`. The WASM crate re-wraps `Game` and the two functions; don't duplicate logic between layers.

### Testing strategy

- Engine correctness: perft tests (move count verification against known positions) in `engine/tests/`
- Search: mate-finding tests
- WASM: Node.js smoke test (`wasm/smoke.cjs`)
- UI + full stack: Playwright E2E (`web/e2e.mjs`)

Perft tests are slow — they're excluded from the pre-commit hook (`--skip perft_`) but run in CI (`cargo test --release --workspace`).

## Definition of Done

**Important**: When implementing any feature, a critical part of the definition of done is to ensure README.md & CLAUDE.md are updated with any relevant information. Documentation updates are required if any change affects an aspect of the application discussed in one of the documentation files.