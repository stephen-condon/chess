//! A bitboard chess engine: move generation, search, and evaluation.

pub mod attacks;
pub mod bitboard;
pub mod fen;
pub mod game;
pub mod magic;
pub mod movegen;
pub mod moves;
pub mod pgn;
pub mod position;
pub mod rules;
pub mod san;
pub mod types;
pub mod zobrist;

pub use bitboard::Bitboard;
pub use game::Game;
pub use moves::{Move, MoveFlag, MoveList};
pub use position::Position;
pub use rules::{DrawReason, Status};
pub use types::{CastleRights, Color, Piece, PieceType, Square};

/// Count leaf nodes of the legal move tree to `depth`. The standard correctness
/// oracle for move generation.
pub fn perft(pos: &mut Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = movegen::legal_moves(pos);
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut nodes = 0;
    for &m in moves.as_slice() {
        let undo = pos.make_move(m);
        nodes += perft(pos, depth - 1);
        pos.unmake_move(undo);
    }
    nodes
}

/// Per-root-move node counts, useful for debugging move-generation mismatches.
pub fn perft_divide(pos: &mut Position, depth: u32) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let moves = movegen::legal_moves(pos);
    for &m in moves.as_slice() {
        let undo = pos.make_move(m);
        let nodes = if depth <= 1 { 1 } else { perft(pos, depth - 1) };
        pos.unmake_move(undo);
        out.push((m.to_uci(), nodes));
    }
    out
}

/// Recompute the Zobrist hash from scratch (oracle for the incremental hash).
pub fn compute_hash(pos: &Position) -> u64 {
    let mut h = 0u64;
    for color in [Color::White, Color::Black] {
        for kind in PieceType::ALL {
            for sq in pos.pieces(color, kind).squares() {
                h ^= zobrist::piece(color, kind, sq);
            }
        }
    }
    h ^= zobrist::castling(pos.castling().bits());
    if let Some(ep) = pos.ep_square() {
        h ^= zobrist::ep_file(ep.file());
    }
    if pos.side_to_move() == Color::Black {
        h ^= zobrist::side_to_move();
    }
    h
}
