//! Board state: piece bitboards + mailbox, plus make/unmake and attack queries.

use crate::attacks;
use crate::bitboard::Bitboard;
use crate::magic;
use crate::moves::{Move, MoveFlag};
use crate::types::{CastleRights, Color, Piece, PieceType, Square};
use crate::zobrist;

#[derive(Clone)]
pub struct Position {
    pieces: [[Bitboard; 6]; 2], // [color][piece]
    color_bb: [Bitboard; 2],
    mailbox: [Option<Piece>; 64],
    pub(crate) side: Color,
    pub(crate) castling: CastleRights,
    pub(crate) ep: Option<Square>,
    pub(crate) halfmove: u16,
    pub(crate) fullmove: u16,
    pub(crate) hash: u64,
}

/// State needed to reverse a move.
#[derive(Clone, Copy)]
pub struct Undo {
    mv: Move,
    captured: Option<Piece>,
    castling: CastleRights,
    ep: Option<Square>,
    halfmove: u16,
    fullmove: u16,
    hash: u64,
}

impl Position {
    pub fn empty() -> Position {
        Position {
            pieces: [[Bitboard::EMPTY; 6]; 2],
            color_bb: [Bitboard::EMPTY; 2],
            mailbox: [None; 64],
            side: Color::White,
            castling: CastleRights::none(),
            ep: None,
            halfmove: 0,
            fullmove: 1,
            hash: 0,
        }
    }

    // --- accessors -------------------------------------------------------

    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.side
    }

    #[inline]
    pub fn castling(&self) -> CastleRights {
        self.castling
    }

    #[inline]
    pub fn ep_square(&self) -> Option<Square> {
        self.ep
    }

    #[inline]
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove
    }

    #[inline]
    pub fn fullmove_number(&self) -> u16 {
        self.fullmove
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        self.mailbox[sq.index()]
    }

    #[inline]
    pub fn pieces(&self, color: Color, kind: PieceType) -> Bitboard {
        self.pieces[color.index()][kind.index()]
    }

    #[inline]
    pub fn color_occupancy(&self, color: Color) -> Bitboard {
        self.color_bb[color.index()]
    }

    #[inline]
    pub fn occupancy(&self) -> Bitboard {
        self.color_bb[0] | self.color_bb[1]
    }

    #[inline]
    pub fn king_square(&self, color: Color) -> Square {
        self.pieces(color, PieceType::King).lsb()
    }

    // --- low-level piece mutation (keeps bitboards, mailbox, hash in sync) --

    pub(crate) fn add_piece(&mut self, color: Color, kind: PieceType, sq: Square) {
        let bb = Bitboard::from_square(sq);
        self.pieces[color.index()][kind.index()] |= bb;
        self.color_bb[color.index()] |= bb;
        self.mailbox[sq.index()] = Some(Piece::new(color, kind));
        self.hash ^= zobrist::piece(color, kind, sq);
    }

    fn remove_piece(&mut self, sq: Square) {
        let p = self.mailbox[sq.index()].expect("remove_piece on empty square");
        let bb = Bitboard::from_square(sq);
        self.pieces[p.color.index()][p.kind.index()] ^= bb;
        self.color_bb[p.color.index()] ^= bb;
        self.mailbox[sq.index()] = None;
        self.hash ^= zobrist::piece(p.color, p.kind, sq);
    }

    fn move_piece(&mut self, from: Square, to: Square) {
        let p = self.mailbox[from.index()].expect("move_piece from empty square");
        self.remove_piece(from);
        self.add_piece(p.color, p.kind, to);
    }

    // --- attack queries --------------------------------------------------

    /// Is `sq` attacked by any piece of color `by`?
    pub fn is_attacked(&self, sq: Square, by: Color) -> bool {
        if (attacks::pawn_attacks(by.opp(), sq) & self.pieces(by, PieceType::Pawn)).any() {
            return true;
        }
        if (attacks::knight_attacks(sq) & self.pieces(by, PieceType::Knight)).any() {
            return true;
        }
        if (attacks::king_attacks(sq) & self.pieces(by, PieceType::King)).any() {
            return true;
        }
        let occ = self.occupancy();
        let bishops = self.pieces(by, PieceType::Bishop) | self.pieces(by, PieceType::Queen);
        if (magic::bishop_attacks(sq, occ) & bishops).any() {
            return true;
        }
        let rooks = self.pieces(by, PieceType::Rook) | self.pieces(by, PieceType::Queen);
        if (magic::rook_attacks(sq, occ) & rooks).any() {
            return true;
        }
        false
    }

    #[inline]
    pub fn in_check(&self, color: Color) -> bool {
        self.is_attacked(self.king_square(color), color.opp())
    }

    // --- make / unmake ---------------------------------------------------

    pub fn make_move(&mut self, mv: Move) -> Undo {
        let us = self.side;
        let them = us.opp();
        let from = mv.from();
        let to = mv.to();
        let flag = mv.flag();
        let moving = self.mailbox[from.index()].expect("make_move from empty square");

        let undo = Undo {
            mv,
            captured: None,
            castling: self.castling,
            ep: self.ep,
            halfmove: self.halfmove,
            fullmove: self.fullmove,
            hash: self.hash,
        };

        // Clear any existing en-passant square from the hash.
        if let Some(ep) = self.ep {
            self.hash ^= zobrist::ep_file(ep.file());
        }
        self.ep = None;

        // Hash out current castling rights (re-hashed in after updating).
        self.hash ^= zobrist::castling(self.castling.bits());

        let is_pawn = moving.kind == PieceType::Pawn;
        let is_capture = mv.is_capture();
        let mut captured = None;

        match flag {
            MoveFlag::Quiet => self.move_piece(from, to),
            MoveFlag::DoublePush => {
                self.move_piece(from, to);
                let ep_sq = Square((from.0 + to.0) / 2);
                self.ep = Some(ep_sq);
                self.hash ^= zobrist::ep_file(ep_sq.file());
            }
            MoveFlag::Capture => {
                captured = self.mailbox[to.index()];
                self.remove_piece(to);
                self.move_piece(from, to);
            }
            MoveFlag::EnPassant => {
                let cap_sq = Square::from_file_rank(to.file(), from.rank());
                captured = self.mailbox[cap_sq.index()];
                self.remove_piece(cap_sq);
                self.move_piece(from, to);
            }
            MoveFlag::KingCastle => {
                self.move_piece(from, to);
                self.move_piece(Square(to.0 + 1), Square(to.0 - 1));
            }
            MoveFlag::QueenCastle => {
                self.move_piece(from, to);
                self.move_piece(Square(to.0 - 2), Square(to.0 + 1));
            }
            MoveFlag::PromoKnight
            | MoveFlag::PromoBishop
            | MoveFlag::PromoRook
            | MoveFlag::PromoQueen => {
                let promo = mv.promotion().unwrap();
                self.remove_piece(from);
                self.add_piece(us, promo, to);
            }
            MoveFlag::PromoKnightCapture
            | MoveFlag::PromoBishopCapture
            | MoveFlag::PromoRookCapture
            | MoveFlag::PromoQueenCapture => {
                let promo = mv.promotion().unwrap();
                captured = self.mailbox[to.index()];
                self.remove_piece(to);
                self.remove_piece(from);
                self.add_piece(us, promo, to);
            }
        }

        // Castling-rights update via per-square masks.
        let new_rights = self.castling.bits() & castle_mask(from) & castle_mask(to);
        self.castling = CastleRights(new_rights);
        self.hash ^= zobrist::castling(new_rights);

        // Halfmove clock and fullmove number.
        self.halfmove = if is_pawn || is_capture {
            0
        } else {
            self.halfmove + 1
        };
        if us == Color::Black {
            self.fullmove += 1;
        }

        // Flip side to move.
        self.side = them;
        self.hash ^= zobrist::side_to_move();

        let mut undo = undo;
        undo.captured = captured;
        undo
    }

    pub fn unmake_move(&mut self, undo: Undo) {
        // Flip side back to the mover.
        self.side = self.side.opp();
        let us = self.side;
        let them = us.opp();
        let mv = undo.mv;
        let from = mv.from();
        let to = mv.to();
        let flag = mv.flag();

        match flag {
            MoveFlag::Quiet | MoveFlag::DoublePush => self.move_piece(to, from),
            MoveFlag::Capture => {
                self.move_piece(to, from);
                let cap = undo.captured.unwrap();
                self.add_piece(cap.color, cap.kind, to);
            }
            MoveFlag::EnPassant => {
                self.move_piece(to, from);
                let cap_sq = Square::from_file_rank(to.file(), from.rank());
                self.add_piece(them, PieceType::Pawn, cap_sq);
            }
            MoveFlag::KingCastle => {
                self.move_piece(to, from);
                self.move_piece(Square(to.0 - 1), Square(to.0 + 1));
            }
            MoveFlag::QueenCastle => {
                self.move_piece(to, from);
                self.move_piece(Square(to.0 + 1), Square(to.0 - 2));
            }
            MoveFlag::PromoKnight
            | MoveFlag::PromoBishop
            | MoveFlag::PromoRook
            | MoveFlag::PromoQueen => {
                self.remove_piece(to);
                self.add_piece(us, PieceType::Pawn, from);
            }
            MoveFlag::PromoKnightCapture
            | MoveFlag::PromoBishopCapture
            | MoveFlag::PromoRookCapture
            | MoveFlag::PromoQueenCapture => {
                self.remove_piece(to);
                self.add_piece(us, PieceType::Pawn, from);
                let cap = undo.captured.unwrap();
                self.add_piece(cap.color, cap.kind, to);
            }
        }

        // Restore scalar state directly (hash overwrite makes the incremental
        // XORs done above irrelevant).
        self.castling = undo.castling;
        self.ep = undo.ep;
        self.halfmove = undo.halfmove;
        self.fullmove = undo.fullmove;
        self.hash = undo.hash;
    }
}

/// Castling-rights mask for a square: bits to *keep* when a piece leaves or
/// arrives there. King and rook origin squares clear the relevant rights.
fn castle_mask(sq: Square) -> u8 {
    const ALL: u8 = 0b1111;
    match sq.0 {
        0 => ALL & !CastleRights::WHITE_QUEEN, // a1
        7 => ALL & !CastleRights::WHITE_KING,  // h1
        4 => ALL & !(CastleRights::WHITE_KING | CastleRights::WHITE_QUEEN), // e1
        56 => ALL & !CastleRights::BLACK_QUEEN, // a8
        63 => ALL & !CastleRights::BLACK_KING, // h8
        60 => ALL & !(CastleRights::BLACK_KING | CastleRights::BLACK_QUEEN), // e8
        _ => ALL,
    }
}
