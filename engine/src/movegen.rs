//! Pseudo-legal move generation and the legality filter.

use crate::attacks;
use crate::bitboard::Bitboard;
use crate::magic;
use crate::moves::{Move, MoveFlag, MoveList};
use crate::position::Position;
use crate::types::{Color, PieceType, Square};

/// All fully legal moves in the position. Mutates `pos` transiently via
/// make/unmake but leaves it unchanged on return.
pub fn legal_moves(pos: &mut Position) -> MoveList {
    let mut pseudo = MoveList::new();
    generate_pseudo_legal(pos, &mut pseudo);

    let us = pos.side_to_move();
    let mut legal = MoveList::new();
    for &m in pseudo.as_slice() {
        let undo = pos.make_move(m);
        if !pos.is_attacked(pos.king_square(us), us.opp()) {
            legal.push(m);
        }
        pos.unmake_move(undo);
    }
    legal
}

pub fn generate_pseudo_legal(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    gen_pawns(pos, us, list);
    gen_knights(pos, us, list);
    gen_king(pos, us, list);
    gen_sliders(pos, us, list);
    gen_castling(pos, us, list);
}

fn gen_pawns(pos: &Position, us: Color, list: &mut MoveList) {
    let them = us.opp();
    let empty = !pos.occupancy();
    let enemy = pos.color_occupancy(them);
    let pawns = pos.pieces(us, PieceType::Pawn);
    let (promo_rank, start_rank) = match us {
        Color::White => (7u8, 1u8),
        Color::Black => (0u8, 6u8),
    };

    for from in pawns.squares() {
        // Pushes.
        if let Some(one) = forward(from, us) {
            if empty.contains(one) {
                if one.rank() == promo_rank {
                    push_promotions(list, from, one, false);
                } else {
                    list.push(Move::new(from, one, MoveFlag::Quiet));
                    if from.rank() == start_rank {
                        let two = forward(one, us).unwrap();
                        if empty.contains(two) {
                            list.push(Move::new(from, two, MoveFlag::DoublePush));
                        }
                    }
                }
            }
        }
        // Captures.
        let caps = attacks::pawn_attacks(us, from) & enemy;
        for to in caps.squares() {
            if to.rank() == promo_rank {
                push_promotions(list, from, to, true);
            } else {
                list.push(Move::new(from, to, MoveFlag::Capture));
            }
        }
        // En passant.
        if let Some(ep) = pos.ep_square() {
            if attacks::pawn_attacks(us, from).contains(ep) {
                list.push(Move::new(from, ep, MoveFlag::EnPassant));
            }
        }
    }
}

fn push_promotions(list: &mut MoveList, from: Square, to: Square, capture: bool) {
    let flags = if capture {
        [
            MoveFlag::PromoKnightCapture,
            MoveFlag::PromoBishopCapture,
            MoveFlag::PromoRookCapture,
            MoveFlag::PromoQueenCapture,
        ]
    } else {
        [
            MoveFlag::PromoKnight,
            MoveFlag::PromoBishop,
            MoveFlag::PromoRook,
            MoveFlag::PromoQueen,
        ]
    };
    for f in flags {
        list.push(Move::new(from, to, f));
    }
}

fn gen_knights(pos: &Position, us: Color, list: &mut MoveList) {
    let own = pos.color_occupancy(us);
    let enemy = pos.color_occupancy(us.opp());
    for from in pos.pieces(us, PieceType::Knight).squares() {
        let targets = attacks::knight_attacks(from) & !own;
        add_moves(list, from, targets, enemy);
    }
}

fn gen_king(pos: &Position, us: Color, list: &mut MoveList) {
    let own = pos.color_occupancy(us);
    let enemy = pos.color_occupancy(us.opp());
    let from = pos.king_square(us);
    let targets = attacks::king_attacks(from) & !own;
    add_moves(list, from, targets, enemy);
}

fn gen_sliders(pos: &Position, us: Color, list: &mut MoveList) {
    let own = pos.color_occupancy(us);
    let enemy = pos.color_occupancy(us.opp());
    let occ = pos.occupancy();

    for from in pos.pieces(us, PieceType::Bishop).squares() {
        add_moves(list, from, magic::bishop_attacks(from, occ) & !own, enemy);
    }
    for from in pos.pieces(us, PieceType::Rook).squares() {
        add_moves(list, from, magic::rook_attacks(from, occ) & !own, enemy);
    }
    for from in pos.pieces(us, PieceType::Queen).squares() {
        add_moves(list, from, magic::queen_attacks(from, occ) & !own, enemy);
    }
}

fn add_moves(list: &mut MoveList, from: Square, targets: Bitboard, enemy: Bitboard) {
    for to in targets.squares() {
        let flag = if enemy.contains(to) {
            MoveFlag::Capture
        } else {
            MoveFlag::Quiet
        };
        list.push(Move::new(from, to, flag));
    }
}

fn gen_castling(pos: &Position, us: Color, list: &mut MoveList) {
    if pos.in_check(us) {
        return;
    }
    let occ = pos.occupancy();
    let rights = pos.castling();
    let them = us.opp();

    // Square indices per color: (E, F, G, D, C, B).
    let (e, f, g, d, c, b, king_flag, queen_flag) = match us {
        Color::White => (4, 5, 6, 3, 2, 1, crate::types::CastleRights::WHITE_KING, crate::types::CastleRights::WHITE_QUEEN),
        Color::Black => (60, 61, 62, 59, 58, 57, crate::types::CastleRights::BLACK_KING, crate::types::CastleRights::BLACK_QUEEN),
    };
    let sq = |i: u8| Square(i);

    if rights.has(king_flag)
        && !occ.contains(sq(f))
        && !occ.contains(sq(g))
        && !pos.is_attacked(sq(f), them)
        && !pos.is_attacked(sq(g), them)
    {
        list.push(Move::new(sq(e), sq(g), MoveFlag::KingCastle));
    }
    if rights.has(queen_flag)
        && !occ.contains(sq(d))
        && !occ.contains(sq(c))
        && !occ.contains(sq(b))
        && !pos.is_attacked(sq(d), them)
        && !pos.is_attacked(sq(c), them)
    {
        list.push(Move::new(sq(e), sq(c), MoveFlag::QueenCastle));
    }
}

#[inline]
fn forward(sq: Square, us: Color) -> Option<Square> {
    match us {
        Color::White => {
            if sq.rank() < 7 {
                Some(Square(sq.0 + 8))
            } else {
                None
            }
        }
        Color::Black => {
            if sq.rank() > 0 {
                Some(Square(sq.0 - 8))
            } else {
                None
            }
        }
    }
}
