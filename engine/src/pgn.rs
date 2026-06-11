//! Minimal PGN export and import.
//!
//! Export writes the seven-tag roster plus movetext. Import understands tags,
//! `{}` comments, `()` variations, NAGs, move numbers, and a result token; it
//! replays SAN moves against the engine's own generator.

use crate::fen;
use crate::game::Game;
use crate::rules::Status;
use crate::types::Color;

/// Serialize a game to PGN.
///
/// `date` fills the `Date` tag and must be in PGN `YYYY.MM.DD` form; pass `None`
/// when the date is unknown (emits the `????.??.??` placeholder). The engine has
/// no clock of its own, so callers supply the date (the WASM layer uses
/// `js_sys::Date`).
pub fn to_pgn(game: &mut Game, date: Option<&str>) -> String {
    let result = result_string(game.status());
    let start = fen::parse(game.start_fen()).expect("valid start FEN");

    let mut out = String::new();
    out.push_str("[Event \"Casual Game\"]\n");
    out.push_str("[Site \"local\"]\n");
    out.push_str(&format!("[Date \"{}\"]\n", date.unwrap_or("????.??.??")));
    out.push_str("[Round \"-\"]\n");
    out.push_str("[White \"White\"]\n");
    out.push_str("[Black \"Black\"]\n");
    out.push_str(&format!("[Result \"{}\"]\n", result));
    if game.start_fen() != fen::START_FEN {
        out.push_str("[SetUp \"1\"]\n");
        out.push_str(&format!("[FEN \"{}\"]\n", game.start_fen()));
    }
    out.push('\n');

    let mut num = start.fullmove_number();
    let mut side = start.side_to_move();
    let mut text = String::new();
    for (i, san) in game.san_history().iter().enumerate() {
        if side == Color::White {
            text.push_str(&format!("{}. ", num));
        } else if i == 0 {
            text.push_str(&format!("{}... ", num));
        }
        text.push_str(san);
        text.push(' ');
        if side == Color::Black {
            num += 1;
        }
        side = side.opp();
    }
    text.push_str(&result);

    out.push_str(&wrap(&text, 80));
    out.push('\n');
    out
}

pub fn from_pgn(pgn: &str) -> Result<Game, String> {
    let mut fen_tag: Option<String> = None;
    let mut movetext = String::new();

    for line in pgn.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if let Some(value) = tag_value(line, "FEN") {
                fen_tag = Some(value);
            }
        } else if !line.is_empty() {
            movetext.push_str(line);
            movetext.push(' ');
        }
    }

    let mut game = match fen_tag {
        Some(f) => Game::from_fen(&f)?,
        None => Game::new(),
    };

    for token in tokenize(&movetext) {
        let mv = strip_move_number(&token);
        if mv.is_empty() || is_result(mv) {
            continue;
        }
        game.play_san(mv)?;
    }

    Ok(game)
}

pub(crate) fn result_string(status: Status) -> String {
    match status {
        Status::Checkmate(Color::White) => "0-1",
        Status::Checkmate(Color::Black) => "1-0",
        Status::Stalemate | Status::Draw(_) => "1/2-1/2",
        Status::Ongoing => "*",
    }
    .to_string()
}

fn tag_value(line: &str, key: &str) -> Option<String> {
    let rest = line.trim_start_matches('[').trim_end_matches(']');
    let rest = rest.strip_prefix(key)?.trim_start();
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

/// Split movetext into tokens, dropping `{}` comments, `()` variations, and
/// `$n` NAGs.
fn tokenize(movetext: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth_brace = 0u32;
    let mut depth_paren = 0u32;
    let mut in_nag = false;

    let flush = |current: &mut String, tokens: &mut Vec<String>| {
        if !current.is_empty() {
            tokens.push(std::mem::take(current));
        }
    };

    for c in movetext.chars() {
        match c {
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            _ if depth_brace > 0 || depth_paren > 0 => {}
            '$' => {
                flush(&mut current, &mut tokens);
                in_nag = true;
            }
            c if c.is_whitespace() => {
                flush(&mut current, &mut tokens);
                in_nag = false;
            }
            _ if in_nag => {}
            c => current.push(c),
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

/// Strip a leading move-number prefix like `12.` or `12...`.
fn strip_move_number(tok: &str) -> &str {
    let bytes = tok.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b'.' {
        while i < bytes.len() && bytes[i] == b'.' {
            i += 1;
        }
        &tok[i..]
    } else {
        tok
    }
}

fn is_result(tok: &str) -> bool {
    matches!(tok, "1-0" | "0-1" | "1/2-1/2" | "*")
}

/// Wrap text at a column boundary on spaces.
pub(crate) fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        if line_len > 0 && line_len + 1 + word.len() > width {
            out.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += word.len();
    }
    out
}
