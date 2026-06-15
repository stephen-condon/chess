//! Core value types: colors, piece kinds, squares, castling rights.

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[inline]
    pub fn opp(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub const ALL: [PieceType; 6] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Lowercase character used by FEN/SAN (`p n b r q k`).
    pub fn to_char(self) -> char {
        match self {
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        }
    }

    pub fn from_char(c: char) -> Option<PieceType> {
        Some(match c.to_ascii_lowercase() {
            'p' => PieceType::Pawn,
            'n' => PieceType::Knight,
            'b' => PieceType::Bishop,
            'r' => PieceType::Rook,
            'q' => PieceType::Queen,
            'k' => PieceType::King,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceType,
}

impl Piece {
    pub fn new(color: Color, kind: PieceType) -> Piece {
        Piece { color, kind }
    }

    /// FEN character: uppercase for white, lowercase for black.
    pub fn to_char(self) -> char {
        let c = self.kind.to_char();
        match self.color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }

    pub fn from_char(c: char) -> Option<Piece> {
        let kind = PieceType::from_char(c)?;
        let color = if c.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece { color, kind })
    }
}

/// A board square, 0..=63, little-endian rank-file: a1=0, b1=1, ... h8=63.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Square(pub u8);

impl Square {
    #[inline]
    pub fn new(index: u8) -> Square {
        debug_assert!(index < 64);
        Square(index)
    }

    #[inline]
    pub fn from_file_rank(file: u8, rank: u8) -> Square {
        debug_assert!(file < 8 && rank < 8);
        Square(rank * 8 + file)
    }

    #[inline]
    pub fn file(self) -> u8 {
        self.0 & 7
    }

    #[inline]
    pub fn rank(self) -> u8 {
        self.0 >> 3
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Algebraic name, e.g. "e4".
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        write!(f, "{}{}", file, rank)
    }
}

/// Parse algebraic name, e.g. "e4".
impl FromStr for Square {
    type Err = String;

    fn from_str(s: &str) -> Result<Square, String> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return Err(format!("bad square '{}'", s));
        }
        let file = bytes[0].checked_sub(b'a').ok_or_else(|| format!("bad square '{}'", s))?;
        let rank = bytes[1].checked_sub(b'1').ok_or_else(|| format!("bad square '{}'", s))?;
        if file < 8 && rank < 8 {
            Ok(Square::from_file_rank(file, rank))
        } else {
            Err(format!("bad square '{}'", s))
        }
    }
}

/// Castling availability, packed into 4 bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CastleRights(pub u8);

impl CastleRights {
    pub const WHITE_KING: u8 = 0b0001;
    pub const WHITE_QUEEN: u8 = 0b0010;
    pub const BLACK_KING: u8 = 0b0100;
    pub const BLACK_QUEEN: u8 = 0b1000;

    pub fn none() -> CastleRights {
        CastleRights(0)
    }

    #[inline]
    pub fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    #[inline]
    pub fn add(&mut self, flag: u8) {
        self.0 |= flag;
    }

    #[inline]
    pub fn bits(self) -> u8 {
        self.0
    }
}
