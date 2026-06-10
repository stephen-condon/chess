//! 16-bit packed move encoding and a fixed-capacity move list.

use crate::types::{PieceType, Square};

/// Move flags occupy the high 4 bits. Encoding follows the common scheme where
/// bit 2 (value 4) marks captures and values >= 8 mark promotions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum MoveFlag {
    Quiet = 0,
    DoublePush = 1,
    KingCastle = 2,
    QueenCastle = 3,
    Capture = 4,
    EnPassant = 5,
    PromoKnight = 8,
    PromoBishop = 9,
    PromoRook = 10,
    PromoQueen = 11,
    PromoKnightCapture = 12,
    PromoBishopCapture = 13,
    PromoRookCapture = 14,
    PromoQueenCapture = 15,
}

impl MoveFlag {
    fn from_u16(v: u16) -> MoveFlag {
        match v {
            0 => MoveFlag::Quiet,
            1 => MoveFlag::DoublePush,
            2 => MoveFlag::KingCastle,
            3 => MoveFlag::QueenCastle,
            4 => MoveFlag::Capture,
            5 => MoveFlag::EnPassant,
            8 => MoveFlag::PromoKnight,
            9 => MoveFlag::PromoBishop,
            10 => MoveFlag::PromoRook,
            11 => MoveFlag::PromoQueen,
            12 => MoveFlag::PromoKnightCapture,
            13 => MoveFlag::PromoBishopCapture,
            14 => MoveFlag::PromoRookCapture,
            15 => MoveFlag::PromoQueenCapture,
            _ => unreachable!("invalid move flag"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move(pub u16);

impl Move {
    #[inline]
    pub fn new(from: Square, to: Square, flag: MoveFlag) -> Move {
        Move((from.0 as u16) | ((to.0 as u16) << 6) | ((flag as u16) << 12))
    }

    #[inline]
    pub fn from(self) -> Square {
        Square((self.0 & 0x3F) as u8)
    }

    #[inline]
    pub fn to(self) -> Square {
        Square(((self.0 >> 6) & 0x3F) as u8)
    }

    #[inline]
    pub fn flag(self) -> MoveFlag {
        MoveFlag::from_u16(self.0 >> 12)
    }

    #[inline]
    pub fn is_capture(self) -> bool {
        (self.0 >> 12) & 0b0100 != 0
    }

    #[inline]
    pub fn is_promotion(self) -> bool {
        (self.0 >> 12) >= 8
    }

    #[inline]
    pub fn is_en_passant(self) -> bool {
        self.flag() == MoveFlag::EnPassant
    }

    /// The piece a pawn promotes to, if this is a promotion.
    pub fn promotion(self) -> Option<PieceType> {
        if !self.is_promotion() {
            return None;
        }
        Some(match (self.0 >> 12) & 0b11 {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            _ => PieceType::Queen,
        })
    }

    /// UCI string, e.g. "e2e4" or "e7e8q".
    pub fn to_uci(self) -> String {
        let mut s = format!("{}{}", self.from().to_string(), self.to().to_string());
        if let Some(p) = self.promotion() {
            s.push(p.to_char());
        }
        s
    }
}

/// Stack-allocated move buffer; ample for any legal chess position.
pub struct MoveList {
    moves: [Move; 256],
    len: usize,
}

impl MoveList {
    pub fn new() -> MoveList {
        MoveList {
            moves: [Move(0); 256],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, m: Move) {
        self.moves[self.len] = m;
        self.len += 1;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }
}

impl Default for MoveList {
    fn default() -> Self {
        MoveList::new()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
