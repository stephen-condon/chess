//! WASM bindings for the chess engine.
//!
//! Exposes a `Game` class (the main-thread, synchronous source of truth for
//! rules/highlighting) and a `search` function (run in a Web Worker). Data
//! crosses the boundary as plain JS objects via serde, with moves as UCI
//! strings and positions/games as FEN/PGN.

use chess_engine::{
    fen, pgn, Color, DrawReason, Game as CoreGame, MoveClass, PieceType, SearchLimits, SideSummary, Status,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

fn err_to_js(e: String) -> JsValue {
    JsValue::from_str(&e)
}

fn color_name(c: Color) -> &'static str {
    match c {
        Color::White => "white",
        Color::Black => "black",
    }
}

/// Today's local date as a PGN `YYYY.MM.DD` string.
fn today_pgn_date() -> String {
    let now = js_sys::Date::new_0();
    // `get_month` is 0-based; the rest are already calendar values.
    format!(
        "{:04}.{:02}.{:02}",
        now.get_full_year(),
        now.get_month() + 1,
        now.get_date()
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveInfo {
    from: u8,
    to: u8,
    uci: String,
    san: String,
    promotion: Option<char>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusInfo {
    /// "ongoing" | "checkmate" | "stalemate" | "draw"
    state: String,
    /// Winning side for checkmate.
    winner: Option<String>,
    /// Draw reason: "fifty-move" | "repetition" | "insufficient-material".
    reason: Option<String>,
    in_check: bool,
    turn: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveResult {
    san: String,
    fen: String,
    status: StatusInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchInfo {
    best_move: Option<String>,
    score: i32,
    depth: u8,
    nodes: u64,
    pv: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzedMoveInfo {
    ply: usize,
    color: String,
    san: String,
    best_san: String,
    eval_before: i32,
    eval_after: i32,
    cpl: i32,
    /// "best" | "good" | "inaccuracy" | "mistake" | "blunder"
    class: String,
    turning_point: bool,
    decided_game: bool,
    pv_san: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SideSummaryInfo {
    accuracy: f32,
    avg_cpl: i32,
    best: u32,
    inaccuracies: u32,
    mistakes: u32,
    blunders: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameReportInfo {
    moves: Vec<AnalyzedMoveInfo>,
    white: SideSummaryInfo,
    black: SideSummaryInfo,
    /// "1-0" | "0-1" | "1/2-1/2" | "*"
    result: String,
    annotated_pgn: String,
}

fn move_class_name(c: MoveClass) -> &'static str {
    match c {
        MoveClass::Best => "best",
        MoveClass::Good => "good",
        MoveClass::Inaccuracy => "inaccuracy",
        MoveClass::Mistake => "mistake",
        MoveClass::Blunder => "blunder",
    }
}

fn side_summary_info(s: &SideSummary) -> SideSummaryInfo {
    SideSummaryInfo {
        accuracy: s.accuracy,
        avg_cpl: s.avg_cpl,
        best: s.best,
        inaccuracies: s.inaccuracies,
        mistakes: s.mistakes,
        blunders: s.blunders,
    }
}

fn status_info(game: &mut CoreGame) -> StatusInfo {
    let status = game.status();
    let (state, winner, reason) = match status {
        Status::Ongoing => ("ongoing", None, None),
        Status::Checkmate(loser) => ("checkmate", Some(color_name(loser.opp()).to_string()), None),
        Status::Stalemate => ("stalemate", None, None),
        Status::Draw(r) => {
            let reason = match r {
                DrawReason::FiftyMove => "fifty-move",
                DrawReason::Repetition => "repetition",
                DrawReason::InsufficientMaterial => "insufficient-material",
            };
            ("draw", None, Some(reason.to_string()))
        }
    };
    StatusInfo {
        state: state.to_string(),
        winner,
        reason,
        in_check: game.in_check(),
        turn: color_name(game.side_to_move()).to_string(),
    }
}

#[wasm_bindgen]
pub struct Game {
    inner: CoreGame,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game {
            inner: CoreGame::new(),
        }
    }

    #[wasm_bindgen(js_name = fromFen)]
    pub fn from_fen(fen_str: &str) -> Result<Game, JsValue> {
        CoreGame::from_fen(fen_str)
            .map(|inner| Game { inner })
            .map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = fromPgn)]
    pub fn from_pgn(pgn_str: &str) -> Result<Game, JsValue> {
        pgn::from_pgn(pgn_str)
            .map(|inner| Game { inner })
            .map_err(err_to_js)
    }

    pub fn fen(&self) -> String {
        self.inner.fen()
    }

    #[wasm_bindgen(js_name = toPgn)]
    pub fn to_pgn(&mut self) -> String {
        pgn::to_pgn(&mut self.inner, Some(&today_pgn_date()))
    }

    /// 64-character board string, index = square (a1=0 … h8=63). Each char is a
    /// FEN piece letter or '.' for an empty square.
    pub fn board(&self) -> String {
        let mut s = String::with_capacity(64);
        for i in 0..64u8 {
            match self.inner.position().piece_on(chess_engine::Square(i)) {
                Some(p) => s.push(p.to_char()),
                None => s.push('.'),
            }
        }
        s
    }

    #[wasm_bindgen(js_name = sideToMove)]
    pub fn side_to_move(&self) -> String {
        color_name(self.inner.side_to_move()).to_string()
    }

    #[wasm_bindgen(js_name = legalMoves)]
    pub fn legal_moves(&mut self) -> JsValue {
        let moves: Vec<MoveInfo> = self
            .inner
            .legal_moves_san()
            .into_iter()
            .map(|(m, san)| MoveInfo {
                from: m.from().0,
                to: m.to().0,
                uci: m.to_uci(),
                san,
                promotion: m.promotion().map(|p| p.to_char()),
            })
            .collect();
        serde_wasm_bindgen::to_value(&moves).unwrap()
    }

    /// Distinct legal destination squares from `square`, for highlighting.
    #[wasm_bindgen(js_name = legalDestinations)]
    pub fn legal_destinations(&mut self, square: u8) -> Vec<u8> {
        self.inner
            .legal_destinations(chess_engine::Square(square))
            .into_iter()
            .map(|s| s.0)
            .collect()
    }

    /// Apply a move by square indices, with an optional promotion piece
    /// ("q"|"r"|"b"|"n"). Returns the SAN, new FEN, and game status.
    #[wasm_bindgen(js_name = makeMove)]
    pub fn make_move(&mut self, from: u8, to: u8, promotion: Option<String>) -> Result<JsValue, JsValue> {
        let promo = promotion
            .as_deref()
            .and_then(|s| s.chars().next())
            .and_then(PieceType::from_char);
        let san = self
            .inner
            .play_move(chess_engine::Square(from), chess_engine::Square(to), promo)
            .map_err(err_to_js)?;
        let result = MoveResult {
            san,
            fen: self.inner.fen(),
            status: status_info(&mut self.inner),
        };
        Ok(serde_wasm_bindgen::to_value(&result).unwrap())
    }

    /// Apply a move in UCI form ("e2e4", "e7e8q").
    #[wasm_bindgen(js_name = makeUci)]
    pub fn make_uci(&mut self, uci: &str) -> Result<JsValue, JsValue> {
        let san = self.inner.play_uci(uci).map_err(err_to_js)?;
        let result = MoveResult {
            san,
            fen: self.inner.fen(),
            status: status_info(&mut self.inner),
        };
        Ok(serde_wasm_bindgen::to_value(&result).unwrap())
    }

    pub fn status(&mut self) -> JsValue {
        serde_wasm_bindgen::to_value(&status_info(&mut self.inner)).unwrap()
    }

    #[wasm_bindgen(js_name = historySan)]
    pub fn history_san(&self) -> Vec<String> {
        self.inner.san_history().to_vec()
    }

    pub fn undo(&mut self) -> bool {
        self.inner.undo()
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

/// Search a position (given as FEN) for the best move. Intended to run in a Web
/// Worker. Returns the best move (UCI), score, depth reached, nodes, and PV.
#[wasm_bindgen]
pub fn search(fen_str: &str, time_ms: u32, max_depth: u8) -> Result<JsValue, JsValue> {
    let pos = fen::parse(fen_str).map_err(err_to_js)?;
    let limits = SearchLimits {
        max_depth,
        time_ms: time_ms as u64,
    };
    let res = chess_engine::search(&pos, limits, || js_sys::Date::now() as u64);
    let info = SearchInfo {
        best_move: res.best_move.map(|m| m.to_uci()),
        score: res.score,
        depth: res.depth,
        nodes: res.nodes,
        pv: res.pv.iter().map(|m| m.to_uci()).collect(),
    };
    Ok(serde_wasm_bindgen::to_value(&info).unwrap())
}

/// Analyze a finished game given as PGN. Intended to run in a Web Worker.
/// `progress(done, total)` is called once per searched position so the caller
/// can report progress for long games.
#[wasm_bindgen]
pub fn analyze(pgn_str: &str, time_ms: u32, max_depth: u8, progress: &js_sys::Function) -> Result<JsValue, JsValue> {
    let mut game = pgn::from_pgn(pgn_str).map_err(err_to_js)?;
    let limits = SearchLimits {
        max_depth,
        time_ms: time_ms as u64,
    };
    let report = chess_engine::analyze(&mut game, limits, || js_sys::Date::now() as u64, |done, total| {
        let _ = progress.call2(&JsValue::NULL, &JsValue::from(done as u32), &JsValue::from(total as u32));
    });

    let info = GameReportInfo {
        moves: report
            .moves
            .iter()
            .map(|m| AnalyzedMoveInfo {
                ply: m.ply,
                color: color_name(m.color).to_string(),
                san: m.san.clone(),
                best_san: m.best_san.clone(),
                eval_before: m.eval_before,
                eval_after: m.eval_after,
                cpl: m.cpl,
                class: move_class_name(m.class).to_string(),
                turning_point: m.turning_point,
                decided_game: m.decided_game,
                pv_san: m.pv_san.clone(),
            })
            .collect(),
        white: side_summary_info(&report.white),
        black: side_summary_info(&report.black),
        result: report.result,
        annotated_pgn: report.annotated_pgn,
    };
    Ok(serde_wasm_bindgen::to_value(&info).unwrap())
}
