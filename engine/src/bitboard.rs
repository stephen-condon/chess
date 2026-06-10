//! A 64-bit bitboard, one bit per square (little-endian rank-file).

use crate::types::Square;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Bitboard(pub u64);

pub const FILE_A: u64 = 0x0101_0101_0101_0101;
pub const FILE_H: u64 = 0x8080_8080_8080_8080;
pub const RANK_1: u64 = 0x0000_0000_0000_00FF;
pub const RANK_8: u64 = 0xFF00_0000_0000_0000;

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL: Bitboard = Bitboard(!0);

    #[inline]
    pub fn from_square(sq: Square) -> Bitboard {
        Bitboard(1u64 << sq.0)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn any(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn contains(self, sq: Square) -> bool {
        self.0 & (1u64 << sq.0) != 0
    }

    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Square of the least-significant set bit. Caller must ensure non-empty.
    #[inline]
    pub fn lsb(self) -> Square {
        debug_assert!(self.0 != 0);
        Square(self.0.trailing_zeros() as u8)
    }

    /// Pop and return the least-significant set square, clearing it.
    #[inline]
    pub fn pop_lsb(&mut self) -> Square {
        let sq = self.lsb();
        self.0 &= self.0 - 1;
        sq
    }

    // Directional one-step shifts with wrap masking.
    #[inline]
    pub fn north(self) -> Bitboard {
        Bitboard(self.0 << 8)
    }
    #[inline]
    pub fn south(self) -> Bitboard {
        Bitboard(self.0 >> 8)
    }
    #[inline]
    pub fn east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H) << 1)
    }
    #[inline]
    pub fn west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A) >> 1)
    }
    #[inline]
    pub fn north_east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H) << 9)
    }
    #[inline]
    pub fn north_west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A) << 7)
    }
    #[inline]
    pub fn south_east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H) >> 7)
    }
    #[inline]
    pub fn south_west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A) >> 9)
    }

    /// Iterate set squares, consuming the bitboard.
    pub fn squares(self) -> SquareIter {
        SquareIter(self.0)
    }
}

pub struct SquareIter(u64);

impl Iterator for SquareIter {
    type Item = Square;
    #[inline]
    fn next(&mut self) -> Option<Square> {
        if self.0 == 0 {
            None
        } else {
            let sq = Square(self.0.trailing_zeros() as u8);
            self.0 &= self.0 - 1;
            Some(sq)
        }
    }
}

impl BitAnd for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}
impl BitOr for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}
impl BitXor for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ rhs.0)
    }
}
impl Not for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}
impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Bitboard) {
        self.0 &= rhs.0;
    }
}
impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Bitboard) {
        self.0 |= rhs.0;
    }
}
impl BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Bitboard) {
        self.0 ^= rhs.0;
    }
}
