//! Negamax alpha-beta search with iterative deepening, a transposition table,
//! quiescence search, and move ordering.
//!
//! Timing is supplied by the caller through a `now_ms` closure so the engine
//! stays free of `std::time` (which is unavailable on `wasm32`).

use crate::eval::evaluate;
use crate::movegen;
use crate::moves::Move;
use crate::position::Position;
use crate::tt::{Bound, TranspositionTable};
use crate::types::PieceType;

const INF: i32 = 1_000_000;
const MATE: i32 = 30_000;
const MATE_THRESHOLD: i32 = MATE - 1000;
const MAX_PLY: usize = 64;

/// Centipawn-ish values for move ordering (king set high so its capture, which
/// never actually occurs, still sorts first if seen).
const ORDER_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 20_000];

#[derive(Clone, Copy)]
pub struct SearchLimits {
    pub max_depth: u8,
    pub time_ms: u64,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            max_depth: 64,
            time_ms: 1000,
        }
    }
}

pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub pv: Vec<Move>,
}

struct Searcher<F: Fn() -> u64> {
    pos: Position,
    tt: TranspositionTable,
    killers: [[Move; 2]; MAX_PLY],
    history: [[[i32; 64]; 64]; 2],
    path: Vec<u64>,
    nodes: u64,
    now_ms: F,
    start_ms: u64,
    deadline_ms: u64,
    stop: bool,
}

/// Search `root` under `limits`, returning the best move found.
pub fn search<F: Fn() -> u64>(root: &Position, limits: SearchLimits, now_ms: F) -> SearchResult {
    let start = now_ms();
    let mut s = Searcher {
        pos: root.clone(),
        tt: TranspositionTable::new(16),
        killers: [[Move(0); 2]; MAX_PLY],
        history: [[[0; 64]; 64]; 2],
        path: vec![root.hash()],
        nodes: 0,
        now_ms,
        start_ms: start,
        deadline_ms: limits.time_ms,
        stop: false,
    };

    let mut best_move: Option<Move> = None;
    let mut best_score = 0;
    let mut completed_depth = 0;

    for depth in 1..=limits.max_depth {
        let (mv, score) = s.search_root(depth as i32, best_move);
        if s.stop {
            break;
        }
        best_move = mv;
        best_score = score;
        completed_depth = depth;
        // Stop early once a forced mate is proven.
        if score.abs() >= MATE_THRESHOLD {
            break;
        }
    }

    // If we never finished even depth 1 (shouldn't happen), fall back to any move.
    if best_move.is_none() {
        let legal = movegen::legal_moves(&mut s.pos);
        best_move = legal.as_slice().first().copied();
    }

    let pv = best_move.map(|m| s.extract_pv(m)).unwrap_or_default();
    SearchResult {
        best_move,
        score: best_score,
        depth: completed_depth,
        nodes: s.nodes,
        pv,
    }
}

impl<F: Fn() -> u64> Searcher<F> {
    fn check_time(&mut self) {
        if self.nodes & 2047 == 0 && (self.now_ms)() - self.start_ms >= self.deadline_ms {
            self.stop = true;
        }
    }

    fn search_root(&mut self, depth: i32, prev_best: Option<Move>) -> (Option<Move>, i32) {
        let mut moves: Vec<Move> = movegen::legal_moves(&mut self.pos).as_slice().to_vec();
        if moves.is_empty() {
            return (None, 0);
        }
        let tt_move = prev_best.unwrap_or(Move(0));
        self.order_moves(&mut moves, tt_move, 0);

        let mut alpha = -INF;
        let beta = INF;
        let mut best = -INF;
        let mut best_move = moves[0];

        for &m in &moves {
            let undo = self.pos.make_move(m);
            self.path.push(self.pos.hash());
            let score = -self.negamax(depth - 1, -beta, -alpha, 1);
            self.path.pop();
            self.pos.unmake_move(undo);

            if self.stop {
                return (Some(best_move), best);
            }
            if score > best {
                best = score;
                best_move = m;
                if score > alpha {
                    alpha = score;
                }
            }
        }
        (Some(best_move), best)
    }

    fn negamax(&mut self, mut depth: i32, mut alpha: i32, beta: i32, ply: i32) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stop {
            return 0;
        }

        // Draw by repetition (twofold within the search path) or fifty-move.
        let h = self.pos.hash();
        if self.path.iter().filter(|&&x| x == h).count() >= 2 || self.pos.halfmove_clock() >= 100 {
            return 0;
        }

        let stm = self.pos.side_to_move();
        let in_check = self.pos.in_check(stm);
        if in_check && (ply as usize) < MAX_PLY {
            depth += 1; // check extension
        }

        if depth <= 0 {
            return self.quiescence(alpha, beta, ply);
        }

        // Transposition table probe.
        let alpha_orig = alpha;
        let mut tt_move = Move(0);
        if let Some(e) = self.tt.probe(h) {
            tt_move = e.best;
            if e.depth as i32 >= depth {
                let score = score_from_tt(e.score, ply);
                match e.bound {
                    Bound::Exact => return score,
                    Bound::Lower if score >= beta => return score,
                    Bound::Upper if score <= alpha => return score,
                    _ => {}
                }
            }
        }

        let mut moves: Vec<Move> = movegen::legal_moves(&mut self.pos).as_slice().to_vec();
        if moves.is_empty() {
            return if in_check { -MATE + ply } else { 0 };
        }
        self.order_moves(&mut moves, tt_move, ply);

        let mut best = -INF;
        let mut best_move = moves[0];
        for &m in &moves {
            let undo = self.pos.make_move(m);
            self.path.push(self.pos.hash());
            let score = -self.negamax(depth - 1, -beta, -alpha, ply + 1);
            self.path.pop();
            self.pos.unmake_move(undo);

            if self.stop {
                return 0;
            }
            if score > best {
                best = score;
                best_move = m;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !m.is_capture() {
                    self.record_killer(m, ply);
                    self.history[stm.index()][m.from().index()][m.to().index()] += depth * depth;
                }
                break;
            }
        }

        let bound = if best <= alpha_orig {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt
            .store(h, depth as u8, score_to_tt(best, ply), bound, best_move);
        best
    }

    fn quiescence(&mut self, mut alpha: i32, beta: i32, ply: i32) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stop {
            return 0;
        }

        let stm = self.pos.side_to_move();
        let in_check = self.pos.in_check(stm);

        // When in check, search all evasions (no stand-pat).
        if !in_check {
            let stand = evaluate(&self.pos);
            if stand >= beta {
                return stand;
            }
            if stand > alpha {
                alpha = stand;
            }
        }

        let mut moves: Vec<Move> = movegen::legal_moves(&mut self.pos)
            .as_slice()
            .iter()
            .copied()
            .filter(|m| in_check || m.is_capture() || m.is_promotion())
            .collect();

        if moves.is_empty() {
            if in_check {
                return -MATE + ply; // checkmate
            }
            return alpha;
        }
        self.order_moves(&mut moves, Move(0), ply);

        let mut best = if in_check { -INF } else { alpha };
        for &m in &moves {
            let undo = self.pos.make_move(m);
            self.path.push(self.pos.hash());
            let score = -self.quiescence(-beta, -alpha, ply + 1);
            self.path.pop();
            self.pos.unmake_move(undo);

            if self.stop {
                return 0;
            }
            if score > best {
                best = score;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                break;
            }
        }
        best
    }

    fn record_killer(&mut self, m: Move, ply: i32) {
        let k = &mut self.killers[ply as usize];
        if k[0] != m {
            k[1] = k[0];
            k[0] = m;
        }
    }

    fn order_moves(&self, moves: &mut [Move], tt_move: Move, ply: i32) {
        let stm = self.pos.side_to_move();
        moves.sort_by_key(|&m| {
            let score: i64 = if m == tt_move {
                10_000_000
            } else if m.is_capture() {
                let victim = self
                    .pos
                    .piece_on(m.to())
                    .map(|p| ORDER_VALUE[p.kind.index()])
                    .unwrap_or(ORDER_VALUE[PieceType::Pawn.index()]); // en passant
                let attacker = self
                    .pos
                    .piece_on(m.from())
                    .map(|p| ORDER_VALUE[p.kind.index()])
                    .unwrap_or(0);
                1_000_000 + (victim as i64) * 16 - attacker as i64
            } else if self.killers[ply as usize][0] == m {
                900_000
            } else if self.killers[ply as usize][1] == m {
                800_000
            } else {
                self.history[stm.index()][m.from().index()][m.to().index()] as i64
            };
            -score // ascending sort -> highest score first
        });
    }

    /// Reconstruct the principal variation by walking transposition-table moves.
    fn extract_pv(&mut self, first: Move) -> Vec<Move> {
        let mut pv = vec![first];
        let undo = self.pos.make_move(first);
        let mut applied = vec![undo];
        for _ in 0..MAX_PLY {
            let h = self.pos.hash();
            let Some(e) = self.tt.probe(h) else { break };
            let m = e.best;
            if m == Move(0) {
                break;
            }
            let legal = movegen::legal_moves(&mut self.pos);
            if !legal.as_slice().contains(&m) {
                break;
            }
            pv.push(m);
            applied.push(self.pos.make_move(m));
        }
        while let Some(u) = applied.pop() {
            self.pos.unmake_move(u);
        }
        pv
    }
}

/// Adjust a mate score for storage in the TT (encode distance from the root).
fn score_to_tt(score: i32, ply: i32) -> i32 {
    if score >= MATE_THRESHOLD {
        score + ply
    } else if score <= -MATE_THRESHOLD {
        score - ply
    } else {
        score
    }
}

fn score_from_tt(score: i32, ply: i32) -> i32 {
    if score >= MATE_THRESHOLD {
        score - ply
    } else if score <= -MATE_THRESHOLD {
        score + ply
    } else {
        score
    }
}
