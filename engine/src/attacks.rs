//! Precomputed non-sliding attack tables (pawn, knight, king), built once.

use crate::bitboard::Bitboard;
use crate::types::{Color, Square};
use std::sync::OnceLock;

struct Tables {
    knight: [Bitboard; 64],
    king: [Bitboard; 64],
    // pawn attacks indexed [color][square]
    pawn: [[Bitboard; 64]; 2],
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn tables() -> &'static Tables {
    TABLES.get_or_init(build_tables)
}

fn build_tables() -> Tables {
    let mut knight = [Bitboard::EMPTY; 64];
    let mut king = [Bitboard::EMPTY; 64];
    let mut pawn = [[Bitboard::EMPTY; 64]; 2];

    for sq in 0u8..64 {
        let bb = Bitboard(1u64 << sq);
        knight[sq as usize] = knight_from(bb);
        king[sq as usize] = king_from(bb);
        pawn[Color::White.index()][sq as usize] = bb.north_east() | bb.north_west();
        pawn[Color::Black.index()][sq as usize] = bb.south_east() | bb.south_west();
    }

    Tables { knight, king, pawn }
}

fn knight_from(b: Bitboard) -> Bitboard {
    let nne = b.north().north_east();
    let nee = b.north().east().east();
    let see = b.south().east().east();
    let sse = b.south().south_east();
    let ssw = b.south().south_west();
    let sww = b.south().west().west();
    let nww = b.north().west().west();
    let nnw = b.north().north_west();
    nne | nee | see | sse | ssw | sww | nww | nnw
}

fn king_from(b: Bitboard) -> Bitboard {
    b.north()
        | b.south()
        | b.east()
        | b.west()
        | b.north_east()
        | b.north_west()
        | b.south_east()
        | b.south_west()
}

#[inline]
pub fn knight_attacks(sq: Square) -> Bitboard {
    tables().knight[sq.index()]
}

#[inline]
pub fn king_attacks(sq: Square) -> Bitboard {
    tables().king[sq.index()]
}

#[inline]
pub fn pawn_attacks(color: Color, sq: Square) -> Bitboard {
    tables().pawn[color.index()][sq.index()]
}
