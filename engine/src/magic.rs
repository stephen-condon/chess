//! Magic bitboards for sliding pieces (rook, bishop, queen).
//!
//! Magics are found at runtime once and cached. A slow ray-walk generator
//! produces the reference attack sets used to fill the tables (and serves as an
//! oracle in tests).

use crate::bitboard::Bitboard;
use crate::rng::SplitMix64;
use crate::types::Square;
use std::sync::OnceLock;

const ROOK_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

struct Magic {
    mask: u64,
    magic: u64,
    shift: u32,
    attacks: Vec<Bitboard>,
}

impl Magic {
    #[inline]
    fn index(&self, occ: u64) -> usize {
        (((occ & self.mask).wrapping_mul(self.magic)) >> self.shift) as usize
    }

    #[inline]
    fn attacks(&self, occ: u64) -> Bitboard {
        self.attacks[self.index(occ)]
    }
}

struct Magics {
    rook: Vec<Magic>,
    bishop: Vec<Magic>,
}

static MAGICS: OnceLock<Magics> = OnceLock::new();

fn magics() -> &'static Magics {
    MAGICS.get_or_init(|| {
        let mut rng = SplitMix64::new(0x1234_5678_9abc_def0);
        Magics {
            rook: (0..64)
                .map(|s| build_magic(Square(s), &ROOK_DIRS, &mut rng))
                .collect(),
            bishop: (0..64)
                .map(|s| build_magic(Square(s), &BISHOP_DIRS, &mut rng))
                .collect(),
        }
    })
}

/// Ray-walk attack set for the given occupancy. Stops on (and includes) the
/// first blocker in each direction.
pub fn slow_slider_attacks(sq: Square, occ: u64, dirs: &[(i8, i8); 4]) -> Bitboard {
    let mut attacks = 0u64;
    let f0 = sq.file() as i8;
    let r0 = sq.rank() as i8;
    for &(df, dr) in dirs {
        let mut f = f0 + df;
        let mut r = r0 + dr;
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let bit = 1u64 << (r * 8 + f);
            attacks |= bit;
            if occ & bit != 0 {
                break;
            }
            f += df;
            r += dr;
        }
    }
    Bitboard(attacks)
}

/// Relevant occupancy mask: ray squares excluding the board edge in each axis.
fn relevant_mask(sq: Square, dirs: &[(i8, i8); 4]) -> u64 {
    let mut mask = 0u64;
    let f0 = sq.file() as i8;
    let r0 = sq.rank() as i8;
    for &(df, dr) in dirs {
        let mut f = f0 + df;
        let mut r = r0 + dr;
        // Include a ray square only while the *next* step stays on the board,
        // so the edge square in each direction is excluded.
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let nf = f + df;
            let nr = r + dr;
            if !(0..8).contains(&nf) || !(0..8).contains(&nr) {
                break;
            }
            mask |= 1u64 << (r * 8 + f);
            f = nf;
            r = nr;
        }
    }
    mask
}

fn build_magic(sq: Square, dirs: &[(i8, i8); 4], rng: &mut SplitMix64) -> Magic {
    let mask = relevant_mask(sq, dirs);
    let bits = mask.count_ones();
    let size = 1usize << bits;
    let shift = 64 - bits;

    // Enumerate every occupancy subset of the mask (Carry-Rippler) and record
    // its reference attack set.
    let mut occupancies = vec![0u64; size];
    let mut references = vec![Bitboard::EMPTY; size];
    let mut subset = 0u64;
    for i in 0..size {
        occupancies[i] = subset;
        references[i] = slow_slider_attacks(sq, subset, dirs);
        subset = subset.wrapping_sub(mask) & mask;
    }

    // Search for a magic that maps all subsets without conflicting collisions.
    loop {
        let magic = rng.sparse();
        // Cheap reject: magic must spread the high bits of the mask product.
        if (mask.wrapping_mul(magic) & 0xFF00_0000_0000_0000).count_ones() < 6 {
            continue;
        }
        let mut table = vec![Bitboard::EMPTY; size];
        let mut used = vec![false; size];
        let mut ok = true;
        for i in 0..size {
            let idx = ((occupancies[i].wrapping_mul(magic)) >> shift) as usize;
            if used[idx] {
                if table[idx] != references[i] {
                    ok = false;
                    break;
                }
            } else {
                used[idx] = true;
                table[idx] = references[i];
            }
        }
        if ok {
            return Magic {
                mask,
                magic,
                shift,
                attacks: table,
            };
        }
    }
}

#[inline]
pub fn rook_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    magics().rook[sq.index()].attacks(occ.0)
}

#[inline]
pub fn bishop_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    magics().bishop[sq.index()].attacks(occ.0)
}

#[inline]
pub fn queen_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}
