//! FEN parsing and serialization.

use crate::position::Position;
use crate::types::{CastleRights, Color, Piece, Square};
use std::str::FromStr;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn parse(fen: &str) -> Result<Position, String> {
    let fields: Vec<&str> = fen.split_whitespace().collect();
    if fields.len() < 4 {
        return Err(format!("FEN needs at least 4 fields, got {}", fields.len()));
    }

    let mut pos = Position::empty();

    // 1. Piece placement (rank 8 first).
    let mut rank: i32 = 7;
    for row in fields[0].split('/') {
        if rank < 0 {
            return Err("too many ranks in FEN".into());
        }
        let mut file: i32 = 0;
        for ch in row.chars() {
            if let Some(skip) = ch.to_digit(10) {
                file += skip as i32;
            } else {
                let piece = Piece::from_char(ch).ok_or(format!("bad piece '{}'", ch))?;
                if file > 7 {
                    return Err("rank overflow in FEN".into());
                }
                pos.add_piece(piece.color, piece.kind, Square::from_file_rank(file as u8, rank as u8));
                file += 1;
            }
        }
        if file != 8 {
            return Err(format!("rank {} has {} files", rank + 1, file));
        }
        rank -= 1;
    }
    if rank != -1 {
        return Err("not enough ranks in FEN".into());
    }

    // 2. Side to move.
    pos.side = match fields[1] {
        "w" => Color::White,
        "b" => Color::Black,
        other => return Err(format!("bad side '{}'", other)),
    };

    // 3. Castling rights.
    let mut rights = CastleRights::none();
    if fields[2] != "-" {
        for ch in fields[2].chars() {
            match ch {
                'K' => rights.add(CastleRights::WHITE_KING),
                'Q' => rights.add(CastleRights::WHITE_QUEEN),
                'k' => rights.add(CastleRights::BLACK_KING),
                'q' => rights.add(CastleRights::BLACK_QUEEN),
                other => return Err(format!("bad castling char '{}'", other)),
            }
        }
    }
    pos.castling = rights;

    // 4. En passant.
    pos.ep = if fields[3] == "-" {
        None
    } else {
        Some(Square::from_str(fields[3]).map_err(|_| format!("bad ep square '{}'", fields[3]))?)
    };

    // 5/6. Clocks (optional).
    pos.halfmove = fields.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    pos.fullmove = fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

    // Finalize the Zobrist hash (piece terms were added by add_piece).
    pos.hash ^= crate::zobrist::castling(pos.castling.bits());
    if let Some(ep) = pos.ep {
        pos.hash ^= crate::zobrist::ep_file(ep.file());
    }
    if pos.side == Color::Black {
        pos.hash ^= crate::zobrist::side_to_move();
    }

    // Reject illegal positions that would corrupt move generation.
    for color in [Color::White, Color::Black] {
        let kings = pos.pieces(color, crate::types::PieceType::King).count();
        if kings != 1 {
            return Err(format!("{:?} must have exactly one king, found {}", color, kings));
        }
    }
    // The side not to move must not be in check (it would be their king's turn).
    if pos.in_check(pos.side.opp()) {
        return Err("side not to move is in check (illegal position)".into());
    }

    Ok(pos)
}

pub fn to_fen(pos: &Position) -> String {
    let mut s = String::new();

    for rank in (0..8).rev() {
        let mut empty = 0;
        for file in 0..8 {
            let sq = Square::from_file_rank(file, rank);
            match pos.piece_on(sq) {
                Some(p) => {
                    if empty > 0 {
                        s.push_str(&empty.to_string());
                        empty = 0;
                    }
                    s.push(p.to_char());
                }
                None => empty += 1,
            }
        }
        if empty > 0 {
            s.push_str(&empty.to_string());
        }
        if rank > 0 {
            s.push('/');
        }
    }

    s.push(' ');
    s.push(match pos.side_to_move() {
        Color::White => 'w',
        Color::Black => 'b',
    });

    s.push(' ');
    let r = pos.castling();
    if r.bits() == 0 {
        s.push('-');
    } else {
        if r.has(CastleRights::WHITE_KING) {
            s.push('K');
        }
        if r.has(CastleRights::WHITE_QUEEN) {
            s.push('Q');
        }
        if r.has(CastleRights::BLACK_KING) {
            s.push('k');
        }
        if r.has(CastleRights::BLACK_QUEEN) {
            s.push('q');
        }
    }

    s.push(' ');
    match pos.ep_square() {
        Some(sq) => s.push_str(&sq.to_string()),
        None => s.push('-'),
    }

    s.push(' ');
    s.push_str(&pos.halfmove_clock().to_string());
    s.push(' ');
    s.push_str(&pos.fullmove_number().to_string());

    s
}
