//! Game-ending conditions: checkmate, stalemate, and the draw rules.

use crate::position::Position;
use crate::types::{Color, PieceType};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrawReason {
    FiftyMove,
    Repetition,
    InsufficientMaterial,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Ongoing,
    /// The given color has been checkmated (it is the side to move and lost).
    Checkmate(Color),
    Stalemate,
    Draw(DrawReason),
}

impl Status {
    pub fn is_over(self) -> bool {
        !matches!(self, Status::Ongoing)
    }
}

/// True when neither side has sufficient material to force checkmate.
pub fn insufficient_material(pos: &Position) -> bool {
    // Any pawn, rook, or queen means mate is possible.
    for color in [Color::White, Color::Black] {
        if pos.pieces(color, PieceType::Pawn).any()
            || pos.pieces(color, PieceType::Rook).any()
            || pos.pieces(color, PieceType::Queen).any()
        {
            return false;
        }
    }

    let w_knights = pos.pieces(Color::White, PieceType::Knight);
    let b_knights = pos.pieces(Color::Black, PieceType::Knight);
    let w_bishops = pos.pieces(Color::White, PieceType::Bishop);
    let b_bishops = pos.pieces(Color::Black, PieceType::Bishop);
    let knights = w_knights.count() + b_knights.count();
    let bishops = w_bishops.count() + b_bishops.count();
    let minors = knights + bishops;

    // K vs K, and K+single-minor vs K.
    if minors <= 1 {
        return true;
    }

    // K+B vs K+B with both bishops on the same color complex.
    if knights == 0 && w_bishops.count() == 1 && b_bishops.count() == 1 {
        let same_color = |sq: crate::types::Square| (sq.file() + sq.rank()) % 2;
        let wb = w_bishops.lsb();
        let bb = b_bishops.lsb();
        if same_color(wb) == same_color(bb) {
            return true;
        }
    }

    false
}
