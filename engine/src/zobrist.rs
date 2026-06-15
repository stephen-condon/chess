//! Zobrist hashing keys, generated deterministically once.

use crate::rng::SplitMix64;
use crate::types::{Color, PieceType, Square};
use std::sync::OnceLock;

struct Keys {
    pieces: [[[u64; 64]; 6]; 2], // [color][piece][square]
    castling: [u64; 16],
    ep_file: [u64; 8],
    side: u64,
}

static KEYS: OnceLock<Keys> = OnceLock::new();

fn keys() -> &'static Keys {
    KEYS.get_or_init(|| {
        let mut rng = SplitMix64::new(0xD1B5_4A32_D192_ED03);
        let mut pieces = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for s in 0..64 {
                    pieces[c][p][s] = rng.next();
                }
            }
        }
        let mut castling = [0u64; 16];
        for c in castling.iter_mut() {
            *c = rng.next();
        }
        let mut ep_file = [0u64; 8];
        for e in ep_file.iter_mut() {
            *e = rng.next();
        }
        let side = rng.next();
        Keys {
            pieces,
            castling,
            ep_file,
            side,
        }
    })
}

#[inline]
pub fn piece(color: Color, kind: PieceType, sq: Square) -> u64 {
    keys().pieces[color.index()][kind.index()][sq.index()]
}

#[inline]
pub fn castling(rights: u8) -> u64 {
    keys().castling[rights as usize]
}

#[inline]
pub fn ep_file(file: u8) -> u64 {
    keys().ep_file[file as usize]
}

#[inline]
pub fn side_to_move() -> u64 {
    keys().side
}
