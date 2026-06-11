//! Static evaluation: material + piece-square tables, with a king table that is
//! tapered between middlegame and endgame by remaining material.
//!
//! Tables are written in the human-readable a8-first layout (rank 8 on top).
//! A white piece on square `sq` reads `table[sq ^ 56]`; a black piece reads
//! `table[sq]` and is subtracted, giving a vertically symmetric evaluation.

use crate::position::Position;
use crate::types::{Color, PieceType};

/// Centipawn material values, indexed by `PieceType` (king = 0).
const MATERIAL: [i32; 6] = [100, 320, 330, 500, 900, 0];

/// Phase weights per piece for tapering; full board = 24.
const PHASE_WEIGHT: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

const BISHOP_PAIR_BONUS: i32 = 30;

#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
   -50,-40,-30,-30,-30,-30,-40,-50,
   -40,-20,  0,  0,  0,  0,-20,-40,
   -30,  0, 10, 15, 15, 10,  0,-30,
   -30,  5, 15, 20, 20, 15,  5,-30,
   -30,  0, 15, 20, 20, 15,  0,-30,
   -30,  5, 10, 15, 15, 10,  5,-30,
   -40,-20,  0,  5,  5,  0,-20,-40,
   -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
   -20,-10,-10,-10,-10,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5, 10, 10,  5,  0,-10,
   -10,  5,  5, 10, 10,  5,  5,-10,
   -10,  0, 10, 10, 10, 10,  0,-10,
   -10, 10, 10, 10, 10, 10, 10,-10,
   -10,  5,  0,  0,  0,  0,  5,-10,
   -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];

#[rustfmt::skip]
const QUEEN_PST: [i32; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
     0,  0,  5,  5,  5,  5,  0, -5,
   -10,  5,  5,  5,  5,  5,  0,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20,
];

#[rustfmt::skip]
const KING_MG_PST: [i32; 64] = [
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -10,-20,-20,-20,-20,-20,-20,-10,
    20, 20,  0,  0,  0,  0, 20, 20,
    20, 30, 10,  0,  0, 10, 30, 20,
];

#[rustfmt::skip]
const KING_EG_PST: [i32; 64] = [
   -50,-40,-30,-20,-20,-30,-40,-50,
   -30,-20,-10,  0,  0,-10,-20,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-30,  0,  0,  0,  0,-30,-30,
   -50,-30,-30,-30,-30,-30,-30,-50,
];

fn pst(kind: PieceType) -> &'static [i32; 64] {
    match kind {
        PieceType::Pawn => &PAWN_PST,
        PieceType::Knight => &KNIGHT_PST,
        PieceType::Bishop => &BISHOP_PST,
        PieceType::Rook => &ROOK_PST,
        PieceType::Queen => &QUEEN_PST,
        PieceType::King => &KING_MG_PST, // handled specially in evaluate()
    }
}

fn game_phase(pos: &Position) -> i32 {
    let mut phase = 0;
    for color in [Color::White, Color::Black] {
        for kind in [
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
        ] {
            phase += pos.pieces(color, kind).count() as i32 * PHASE_WEIGHT[kind.index()];
        }
    }
    phase.min(MAX_PHASE)
}

/// Evaluate from the side-to-move's perspective (positive = better for them).
pub fn evaluate(pos: &Position) -> i32 {
    let phase = game_phase(pos);
    let mut score = 0i32; // white's perspective

    for kind in [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
    ] {
        let table = pst(kind);
        let mat = MATERIAL[kind.index()];
        for sq in pos.pieces(Color::White, kind).squares() {
            score += mat + table[(sq.0 ^ 56) as usize];
        }
        for sq in pos.pieces(Color::Black, kind).squares() {
            score -= mat + table[sq.0 as usize];
        }
    }

    // Tapered king.
    let king_value = |idx: usize| -> i32 {
        (KING_MG_PST[idx] * phase + KING_EG_PST[idx] * (MAX_PHASE - phase)) / MAX_PHASE
    };
    score += king_value((pos.king_square(Color::White).0 ^ 56) as usize);
    score -= king_value(pos.king_square(Color::Black).0 as usize);

    // Bishop pair.
    if pos.pieces(Color::White, PieceType::Bishop).count() >= 2 {
        score += BISHOP_PAIR_BONUS;
    }
    if pos.pieces(Color::Black, PieceType::Bishop).count() >= 2 {
        score -= BISHOP_PAIR_BONUS;
    }

    match pos.side_to_move() {
        Color::White => score,
        Color::Black => -score,
    }
}
