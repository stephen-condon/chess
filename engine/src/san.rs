//! Standard Algebraic Notation (SAN) serialization of a move.

use crate::movegen;
use crate::moves::{Move, MoveFlag};
use crate::position::Position;
use crate::types::{PieceType, Square};

fn file_char(file: u8) -> char {
    (b'a' + file) as char
}

fn rank_char(rank: u8) -> char {
    (b'1' + rank) as char
}

/// Render `mv` as SAN. `pos` must be the position *before* the move and `legal`
/// its full legal move list (used for disambiguation). The position is restored
/// before returning.
pub fn to_san(pos: &mut Position, mv: Move, legal: &[Move]) -> String {
    let mut s = match mv.flag() {
        MoveFlag::KingCastle => "O-O".to_string(),
        MoveFlag::QueenCastle => "O-O-O".to_string(),
        _ => {
            let piece = pos.piece_on(mv.from()).expect("moving piece present");
            let mut out = String::new();
            if piece.kind == PieceType::Pawn {
                if mv.is_capture() {
                    out.push(file_char(mv.from().file()));
                }
            } else {
                out.push(piece.kind.to_char().to_ascii_uppercase());
                out.push_str(&disambiguation(pos, mv, legal, piece.kind));
            }
            if mv.is_capture() {
                out.push('x');
            }
            out.push_str(&mv.to().to_string());
            if let Some(promo) = mv.promotion() {
                out.push('=');
                out.push(promo.to_char().to_ascii_uppercase());
            }
            out
        }
    };

    // Check / checkmate suffix, determined by playing the move.
    let undo = pos.make_move(mv);
    let opp = pos.side_to_move();
    if pos.in_check(opp) {
        if movegen::legal_moves(pos).is_empty() {
            s.push('#');
        } else {
            s.push('+');
        }
    }
    pos.unmake_move(undo);

    s
}

fn disambiguation(pos: &Position, mv: Move, legal: &[Move], kind: PieceType) -> String {
    let from = mv.from();
    let mut others: Vec<Square> = Vec::new();
    for &m in legal {
        if m.to() == mv.to() && m.from() != from {
            if let Some(p) = pos.piece_on(m.from()) {
                if p.kind == kind {
                    others.push(m.from());
                }
            }
        }
    }
    if others.is_empty() {
        return String::new();
    }
    let file_unique = others.iter().all(|s| s.file() != from.file());
    if file_unique {
        return file_char(from.file()).to_string();
    }
    let rank_unique = others.iter().all(|s| s.rank() != from.rank());
    if rank_unique {
        return rank_char(from.rank()).to_string();
    }
    format!("{}{}", file_char(from.file()), rank_char(from.rank()))
}
