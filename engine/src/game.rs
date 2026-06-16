//! High-level game state: a position plus move/SAN/repetition history and the
//! rules for game termination. This is the engine's public façade.

use crate::fen;
use crate::movegen;
use crate::moves::{Move, MoveList};
use crate::position::{Position, Undo};
use crate::rules::{self, DrawReason, Status};
use crate::san;
use crate::types::{Color, PieceType, Square};
use std::str::FromStr;

#[derive(Clone)]
pub struct Game {
    pos: Position,
    start_fen: String,
    undo_stack: Vec<Undo>,
    moves: Vec<Move>,
    san: Vec<String>,
    hashes: Vec<u64>,
}

impl Game {
    pub fn new() -> Game {
        Game::from_fen(fen::START_FEN).expect("valid start FEN")
    }

    pub fn from_fen(fen_str: &str) -> Result<Game, String> {
        let pos = fen::parse(fen_str)?;
        let hash = pos.hash();
        Ok(Game {
            pos,
            start_fen: fen::to_fen(&fen::parse(fen_str)?),
            undo_stack: Vec::new(),
            moves: Vec::new(),
            san: Vec::new(),
            hashes: vec![hash],
        })
    }

    // --- queries ---------------------------------------------------------

    pub fn position(&self) -> &Position {
        &self.pos
    }

    pub fn side_to_move(&self) -> Color {
        self.pos.side_to_move()
    }

    pub fn fen(&self) -> String {
        fen::to_fen(&self.pos)
    }

    pub fn start_fen(&self) -> &str {
        &self.start_fen
    }

    pub fn in_check(&self) -> bool {
        self.pos.in_check(self.pos.side_to_move())
    }

    pub fn legal_moves(&mut self) -> MoveList {
        movegen::legal_moves(&mut self.pos)
    }

    /// Every legal move paired with its SAN string (for UI move lists / hints).
    pub fn legal_moves_san(&mut self) -> Vec<(Move, String)> {
        let slice: Vec<Move> = movegen::legal_moves(&mut self.pos).as_slice().to_vec();
        slice
            .iter()
            .map(|&m| {
                let s = san::to_san(&mut self.pos, m, &slice);
                (m, s)
            })
            .collect()
    }

    /// Distinct destination squares reachable from `sq` (for UI highlighting).
    pub fn legal_destinations(&mut self, sq: Square) -> Vec<Square> {
        let mut out: Vec<Square> = Vec::new();
        for &m in movegen::legal_moves(&mut self.pos).as_slice() {
            if m.from() == sq && !out.contains(&m.to()) {
                out.push(m.to());
            }
        }
        out
    }

    pub fn san_history(&self) -> &[String] {
        &self.san
    }

    pub fn move_history(&self) -> &[Move] {
        &self.moves
    }

    pub fn status(&mut self) -> Status {
        let stm = self.pos.side_to_move();
        if movegen::legal_moves(&mut self.pos).is_empty() {
            return if self.pos.in_check(stm) {
                Status::Checkmate(stm)
            } else {
                Status::Stalemate
            };
        }
        if self.pos.halfmove_clock() >= 100 {
            return Status::Draw(DrawReason::FiftyMove);
        }
        if self.repetition_count() >= 3 {
            return Status::Draw(DrawReason::Repetition);
        }
        if rules::insufficient_material(&self.pos) {
            return Status::Draw(DrawReason::InsufficientMaterial);
        }
        Status::Ongoing
    }

    fn repetition_count(&self) -> usize {
        let current = *self.hashes.last().unwrap();
        self.hashes.iter().filter(|&&h| h == current).count()
    }

    // --- mutation --------------------------------------------------------

    /// Play a fully specified legal move, returning its SAN. Errors if illegal.
    pub fn play(&mut self, mv: Move) -> Result<String, String> {
        let legal = movegen::legal_moves(&mut self.pos);
        if !legal.as_slice().contains(&mv) {
            return Err(format!("illegal move {}", mv.to_uci()));
        }
        let san = san::to_san(&mut self.pos, mv, legal.as_slice());
        let undo = self.pos.make_move(mv);
        self.undo_stack.push(undo);
        self.moves.push(mv);
        self.san.push(san.clone());
        self.hashes.push(self.pos.hash());
        Ok(san)
    }

    /// Play a move identified by from/to squares and an optional promotion.
    pub fn play_move(
        &mut self,
        from: Square,
        to: Square,
        promo: Option<PieceType>,
    ) -> Result<String, String> {
        let mv = self
            .find_legal(from, to, promo)
            .ok_or_else(|| format!("no legal move {}{}", from, to))?;
        self.play(mv)
    }

    /// Play a move given in UCI form, e.g. "e2e4" or "e7e8q".
    pub fn play_uci(&mut self, uci: &str) -> Result<String, String> {
        let (from, to, promo) = parse_uci(uci)?;
        self.play_move(from, to, promo)
    }

    /// Play a move written in SAN, e.g. "Nf3", "exd6", "O-O", "e8=Q+".
    pub fn play_san(&mut self, token: &str) -> Result<String, String> {
        let want = normalize_san(token);
        let legal = movegen::legal_moves(&mut self.pos);
        let slice: Vec<Move> = legal.as_slice().to_vec();
        for &m in &slice {
            let s = san::to_san(&mut self.pos, m, &slice);
            if normalize_san(&s) == want {
                return self.play(m);
            }
        }
        Err(format!("no legal move for SAN '{}'", token))
    }

    /// Undo the last move, if any.
    pub fn undo(&mut self) -> bool {
        match self.undo_stack.pop() {
            Some(undo) => {
                self.pos.unmake_move(undo);
                self.moves.pop();
                self.san.pop();
                self.hashes.pop();
                true
            }
            None => false,
        }
    }

    fn find_legal(&mut self, from: Square, to: Square, promo: Option<PieceType>) -> Option<Move> {
        movegen::legal_moves(&mut self.pos)
            .as_slice()
            .iter()
            .copied()
            .find(|m| m.from() == from && m.to() == to && m.promotion() == promo)
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

/// Normalize a SAN token for comparison: drop check/mate and annotation marks
/// and treat `0-0` castling (zeros) as `O-O`.
fn normalize_san(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '+' | '#' | '!' | '?'))
        .map(|c| if c == '0' { 'O' } else { c })
        .collect()
}

fn parse_uci(uci: &str) -> Result<(Square, Square, Option<PieceType>), String> {
    if uci.len() < 4 {
        return Err(format!("bad UCI move '{}'", uci));
    }
    let from = Square::from_str(&uci[0..2]).map_err(|_| format!("bad from in '{}'", uci))?;
    let to = Square::from_str(&uci[2..4]).map_err(|_| format!("bad to in '{}'", uci))?;
    let promo = uci.chars().nth(4).and_then(PieceType::from_char);
    Ok((from, to, promo))
}
