//! Post-game analysis: replay a finished game through the search, classify
//! every move by centipawn loss, find the moves that decided the outcome, and
//! render an annotated PGN report.
//!
//! The core trick is that one [`search`] per position covers the whole game.
//! Negamax scores are relative to the side to move, so the search at position
//! `i` gives both the best move available there and the best achievable score;
//! the search at position `i + 1`, negated, gives the score of whatever was
//! actually played. The difference is the move's centipawn loss.

use crate::fen;
use crate::game::Game;
use crate::movegen;
use crate::moves::Move;
use crate::pgn::{result_string, wrap};
use crate::position::Position;
use crate::rules::Status;
use crate::san;
use crate::search::{search, SearchLimits};
use crate::types::Color;

/// Mate scores returned by `search` are within ~1000 of `search::MATE`
/// (30,000); kept in sync manually since that constant isn't exported.
const MATE_CP: i32 = 30_000;
const MATE_THRESHOLD: i32 = 29_000;

/// A position counts as decisive for a side once the White-relative
/// evaluation crosses this margin (centipawns). Used for turning-point and
/// decided-move detection.
const DECISIVE_CP: i32 = 150;

/// Centipawn-loss thresholds for move classification (tunable).
const BEST_CP: i32 = 10;
const GOOD_CP: i32 = 50;
const INACCURACY_CP: i32 = 100;
const MISTAKE_CP: i32 = 200;

/// Number of principal-variation plies shown for "best move" suggestions.
const PV_PLIES: usize = 4;

/// Quality bucket for a single move, derived from its centipawn loss.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveClass {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

impl MoveClass {
    fn from_cpl(cpl: i32) -> MoveClass {
        if cpl < BEST_CP {
            MoveClass::Best
        } else if cpl < GOOD_CP {
            MoveClass::Good
        } else if cpl < INACCURACY_CP {
            MoveClass::Inaccuracy
        } else if cpl < MISTAKE_CP {
            MoveClass::Mistake
        } else {
            MoveClass::Blunder
        }
    }

    /// Annotation glyph appended to the move's SAN (empty for Best/Good).
    pub fn glyph(self) -> &'static str {
        match self {
            MoveClass::Inaccuracy => "?!",
            MoveClass::Mistake => "?",
            MoveClass::Blunder => "??",
            MoveClass::Best | MoveClass::Good => "",
        }
    }
}

/// `(cpl, class)` for a move with the given (non-negative) raw centipawn
/// loss. A forced move (the only legal move) is never penalized.
fn classify_move(raw_cpl: i32, forced: bool) -> (i32, MoveClass) {
    if forced {
        (0, MoveClass::Best)
    } else {
        (raw_cpl, MoveClass::from_cpl(raw_cpl))
    }
}

/// Analysis of a single played move.
#[derive(Clone, Debug)]
pub struct AnalyzedMove {
    /// Zero-based ply index into the game's move history.
    pub ply: usize,
    pub color: Color,
    /// The move actually played, in SAN.
    pub san: String,
    /// The engine's preferred move from this position, in SAN.
    pub best_san: String,
    /// Evaluation before the move, from the mover's perspective (centipawns).
    pub eval_before: i32,
    /// Evaluation after the move, from the mover's perspective (centipawns).
    pub eval_after: i32,
    /// `max(0, eval_before - eval_after)`; zero for forced (only-legal-move) moves.
    pub cpl: i32,
    pub class: MoveClass,
    /// True if this move swung the position from one side's advantage to the
    /// other's (or into/out of balance).
    pub turning_point: bool,
    /// True for the last turning point after which the eventual winner never
    /// lost their advantage. At most one move per game is marked.
    pub decided_game: bool,
    /// The engine's principal variation from this position, in SAN.
    pub pv_san: Vec<String>,
}

/// Per-side scorecard for a [`GameReport`].
#[derive(Clone, Debug)]
pub struct SideSummary {
    /// Heuristic accuracy in `0..=100`, derived from win-probability loss.
    pub accuracy: f32,
    pub avg_cpl: i32,
    pub best: u32,
    pub inaccuracies: u32,
    pub mistakes: u32,
    pub blunders: u32,
}

/// Full post-game analysis report.
#[derive(Clone, Debug)]
pub struct GameReport {
    pub moves: Vec<AnalyzedMove>,
    pub white: SideSummary,
    pub black: SideSummary,
    /// PGN result token: "1-0" | "0-1" | "1/2-1/2" | "*".
    pub result: String,
    pub annotated_pgn: String,
}

/// Replay `game` from its starting position, searching every position the
/// players reached, and produce a [`GameReport`] with per-move
/// classification, turning points, accuracy summaries, and an annotated PGN.
///
/// `progress(done, total)` is called once per searched position so callers can
/// report progress for long games.
pub fn analyze<F, P>(
    game: &mut Game,
    limits: SearchLimits,
    now_ms: F,
    mut progress: P,
) -> GameReport
where
    F: Fn() -> u64,
    P: FnMut(usize, usize),
{
    let status = game.status();
    let result = result_string(status);
    let start_pos = fen::parse(game.start_fen()).expect("valid start FEN");
    let played: Vec<Move> = game.move_history().to_vec();
    let sans: Vec<String> = game.san_history().to_vec();
    let n = played.len();

    let start_side = start_pos.side_to_move();
    let mover = |i: usize| {
        if i.is_multiple_of(2) {
            start_side
        } else {
            start_side.opp()
        }
    };

    // white_eval[i] = White-relative centipawn evaluation of the position
    // before move i is played, for i in 0..=n (white_eval[n] is the final
    // position, after the last move).
    let mut white_eval = vec![0i32; n + 1];
    let mut best_sans: Vec<String> = Vec::with_capacity(n);
    let mut pv_sans: Vec<Vec<String>> = Vec::with_capacity(n);
    let mut forced = vec![false; n];

    let now_ref = &now_ms;
    let mut pos = start_pos.clone();
    for i in 0..n {
        let legal = movegen::legal_moves(&mut pos);
        forced[i] = legal.len() == 1;

        let stm = pos.side_to_move();
        let res = search(&pos, limits, now_ref);
        white_eval[i] = if stm == Color::White {
            res.score
        } else {
            -res.score
        };

        let pv = pv_to_san(&pos, &res.pv, PV_PLIES);
        best_sans.push(pv.first().cloned().unwrap_or_default());
        pv_sans.push(pv);

        pos.make_move(played[i]);
        progress(i + 1, n);
    }

    // The final position's evaluation comes from the actual game outcome, not
    // a search: search returns 0 for a position with no legal moves, and the
    // engine can't see history-based draws (repetition, fifty-move) from the
    // root alone.
    white_eval[n] = match status {
        Status::Checkmate(loser) => {
            if loser == Color::Black {
                MATE_CP
            } else {
                -MATE_CP
            }
        }
        _ => 0,
    };

    let mut moves = Vec::with_capacity(n);
    for i in 0..n {
        let color = mover(i);
        let eval_before = pov(white_eval[i], color);
        let eval_after = pov(white_eval[i + 1], color);
        let (cpl, class) = classify_move((eval_before - eval_after).max(0), forced[i]);

        moves.push(AnalyzedMove {
            ply: i,
            color,
            san: sans[i].clone(),
            best_san: best_sans[i].clone(),
            eval_before,
            eval_after,
            cpl,
            class,
            turning_point: bucket(white_eval[i]) != bucket(white_eval[i + 1]),
            decided_game: false,
            pv_san: pv_sans[i].clone(),
        });
    }

    mark_decided_move(&mut moves, &white_eval, status);

    let white = side_summary(&moves, Color::White, &white_eval);
    let black = side_summary(&moves, Color::Black, &white_eval);

    let mut report = GameReport {
        moves,
        white,
        black,
        result,
        annotated_pgn: String::new(),
    };
    report.annotated_pgn = to_annotated_pgn(game, &report, &white_eval, limits.max_depth);
    report
}

/// Convert a White-relative centipawn value to `color`'s perspective.
fn pov(white_cp: i32, color: Color) -> i32 {
    if color == Color::White {
        white_cp
    } else {
        -white_cp
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Bucket {
    White,
    Balanced,
    Black,
}

fn bucket(white_cp: i32) -> Bucket {
    if white_cp > DECISIVE_CP {
        Bucket::White
    } else if white_cp < -DECISIVE_CP {
        Bucket::Black
    } else {
        Bucket::Balanced
    }
}

/// Find the last turning point after which the eventual winner's advantage
/// bucket held all the way to the end of the game, and mark it as the move
/// that decided the game. Draws have no decided move.
fn mark_decided_move(moves: &mut [AnalyzedMove], white_eval: &[i32], status: Status) {
    let final_bucket = match status {
        Status::Checkmate(Color::Black) => Bucket::White,
        Status::Checkmate(Color::White) => Bucket::Black,
        _ => Bucket::Balanced,
    };
    if final_bucket == Bucket::Balanced {
        return;
    }

    let n = moves.len();
    // suffix_all[k] = white_eval[k..=n] are all in `final_bucket`.
    let mut suffix_all = vec![true; n + 2];
    for k in (0..=n).rev() {
        suffix_all[k] = bucket(white_eval[k]) == final_bucket && suffix_all[k + 1];
    }
    for i in (0..n).rev() {
        if moves[i].turning_point && suffix_all[i + 1] {
            moves[i].decided_game = true;
            return;
        }
    }
}

/// Render the engine's principal variation from `pos` (before any move) as
/// SAN, capped at `max_plies`.
fn pv_to_san(pos: &Position, pv: &[Move], max_plies: usize) -> Vec<String> {
    let mut p = pos.clone();
    let mut out = Vec::new();
    for &m in pv.iter().take(max_plies) {
        let legal = movegen::legal_moves(&mut p);
        if !legal.as_slice().contains(&m) {
            break;
        }
        out.push(san::to_san(&mut p, m, legal.as_slice()));
        p.make_move(m);
    }
    out
}

/// Heuristic win probability for White (0..100) from a White-relative
/// centipawn evaluation. Same family of logistic curve used by Lichess-style
/// accuracy reports; not a calibrated probability.
fn win_percent(white_cp: i32) -> f64 {
    100.0 / (1.0 + 10f64.powf(-(white_cp as f64) / 400.0))
}

/// Map a win-probability drop (0..100) to a per-move accuracy score (0..100).
fn accuracy_from_loss(win_pct_loss: f64) -> f64 {
    let acc = 103.1668 * (-0.04354 * win_pct_loss).exp() - 3.1669;
    acc.clamp(0.0, 100.0)
}

fn side_summary(moves: &[AnalyzedMove], color: Color, white_eval: &[i32]) -> SideSummary {
    let mut total_cpl: i64 = 0;
    let mut total_acc = 0.0;
    let mut count: u32 = 0;
    let (mut best, mut inaccuracies, mut mistakes, mut blunders) = (0, 0, 0, 0);

    for m in moves.iter().filter(|m| m.color == color) {
        count += 1;
        total_cpl += m.cpl as i64;
        match m.class {
            MoveClass::Best | MoveClass::Good => best += 1,
            MoveClass::Inaccuracy => inaccuracies += 1,
            MoveClass::Mistake => mistakes += 1,
            MoveClass::Blunder => blunders += 1,
        }

        let before = white_eval[m.ply];
        let after = white_eval[m.ply + 1];
        let (before_pct, after_pct) = match color {
            Color::White => (win_percent(before), win_percent(after)),
            Color::Black => (100.0 - win_percent(before), 100.0 - win_percent(after)),
        };
        total_acc += accuracy_from_loss((before_pct - after_pct).max(0.0));
    }

    SideSummary {
        accuracy: if count > 0 {
            (total_acc / count as f64) as f32
        } else {
            100.0
        },
        avg_cpl: if count > 0 {
            (total_cpl / count as i64) as i32
        } else {
            0
        },
        best,
        inaccuracies,
        mistakes,
        blunders,
    }
}

fn color_name(c: Color) -> &'static str {
    match c {
        Color::White => "White",
        Color::Black => "Black",
    }
}

/// Format a White-relative centipawn evaluation for a PGN `{}` comment, e.g.
/// `+1.93`, `-0.40`, or `+M`/`-M` for a position at or beyond mate threshold.
fn format_eval(white_cp: i32) -> String {
    if white_cp >= MATE_THRESHOLD {
        "+M".to_string()
    } else if white_cp <= -MATE_THRESHOLD {
        "-M".to_string()
    } else {
        format!("{:+.2}", white_cp as f64 / 100.0)
    }
}

/// Build an annotated PGN: the standard seven-tag roster plus accuracy tags,
/// a summary comment, and per-move eval/NAG/best-move annotations. Comments
/// and variations are all `{}`/`()`-delimited so [`crate::pgn::from_pgn`]
/// reproduces the original move list unchanged.
fn to_annotated_pgn(
    game: &mut Game,
    report: &GameReport,
    white_eval: &[i32],
    max_depth: u8,
) -> String {
    let start = fen::parse(game.start_fen()).expect("valid start FEN");
    let n = report.moves.len();

    // Move-number prefix for each ply ("N. " / "N... " / "" for a continuing
    // black move), computed once and reused for the summary and movetext.
    let mut move_label = vec![String::new(); n];
    let mut full_move_no = vec![0u16; n];
    {
        let mut num = start.fullmove_number();
        let mut side = start.side_to_move();
        for i in 0..n {
            full_move_no[i] = num;
            if side == Color::White {
                move_label[i] = format!("{}. ", num);
            } else if i == 0 {
                move_label[i] = format!("{}... ", num);
            }
            if side == Color::Black {
                num += 1;
            }
            side = side.opp();
        }
    }

    let mut out = String::new();
    out.push_str("[Event \"Casual Game\"]\n");
    out.push_str("[Site \"local\"]\n");
    out.push_str("[Date \"????.??.??\"]\n");
    out.push_str("[Round \"-\"]\n");
    out.push_str("[White \"White\"]\n");
    out.push_str("[Black \"Black\"]\n");
    out.push_str(&format!("[Result \"{}\"]\n", report.result));
    if game.start_fen() != fen::START_FEN {
        out.push_str("[SetUp \"1\"]\n");
        out.push_str(&format!("[FEN \"{}\"]\n", game.start_fen()));
    }
    out.push_str(&format!(
        "[Annotator \"chess engine (depth {})\"]\n",
        max_depth
    ));
    out.push_str(&format!(
        "[WhiteAccuracy \"{:.1}\"]\n",
        report.white.accuracy
    ));
    out.push_str(&format!(
        "[BlackAccuracy \"{:.1}\"]\n",
        report.black.accuracy
    ));
    out.push('\n');

    let mut summary = format!(
        "{{ White {:.1}% accuracy ({}b/{}m/{}i), Black {:.1}% accuracy ({}b/{}m/{}i).",
        report.white.accuracy,
        report.white.blunders,
        report.white.mistakes,
        report.white.inaccuracies,
        report.black.accuracy,
        report.black.blunders,
        report.black.mistakes,
        report.black.inaccuracies,
    );
    if let Some(decisive) = report.moves.iter().find(|m| m.decided_game) {
        let ellipsis = if decisive.color == Color::White {
            "."
        } else {
            "..."
        };
        summary.push_str(&format!(
            " The decisive moment was {}{} {} ({}).",
            full_move_no[decisive.ply],
            ellipsis,
            decisive.san,
            color_name(decisive.color),
        ));
    }
    summary.push_str(" }");
    out.push_str(&wrap(&summary, 80));
    out.push_str("\n\n");

    let mut movetext = String::new();
    for (i, san) in game.san_history().iter().enumerate() {
        let m = &report.moves[i];
        movetext.push_str(&move_label[i]);
        movetext.push_str(san);
        movetext.push_str(m.class.glyph());
        movetext.push_str(&format!(" {{{}}}", format_eval(white_eval[i + 1])));

        let suboptimal = !matches!(m.class, MoveClass::Best | MoveClass::Good);
        if suboptimal && !m.best_san.is_empty() && m.best_san != m.san {
            movetext.push_str(" (");
            movetext.push_str(&m.best_san);
            movetext.push_str(&format!(" {{{}}}", format_eval(white_eval[i])));
            for cont in m.pv_san.iter().skip(1) {
                movetext.push(' ');
                movetext.push_str(cont);
            }
            movetext.push(')');
        }
        if m.decided_game {
            movetext.push_str(" {This move decided the game.}");
        }
        movetext.push(' ');
    }
    movetext.push_str(&report.result);

    out.push_str(&wrap(&movetext, 80));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_thresholds() {
        assert_eq!(MoveClass::from_cpl(0), MoveClass::Best);
        assert_eq!(MoveClass::from_cpl(BEST_CP - 1), MoveClass::Best);
        assert_eq!(MoveClass::from_cpl(BEST_CP), MoveClass::Good);
        assert_eq!(MoveClass::from_cpl(GOOD_CP - 1), MoveClass::Good);
        assert_eq!(MoveClass::from_cpl(GOOD_CP), MoveClass::Inaccuracy);
        assert_eq!(
            MoveClass::from_cpl(INACCURACY_CP - 1),
            MoveClass::Inaccuracy
        );
        assert_eq!(MoveClass::from_cpl(INACCURACY_CP), MoveClass::Mistake);
        assert_eq!(MoveClass::from_cpl(MISTAKE_CP - 1), MoveClass::Mistake);
        assert_eq!(MoveClass::from_cpl(MISTAKE_CP), MoveClass::Blunder);
        assert_eq!(MoveClass::from_cpl(10_000), MoveClass::Blunder);
    }

    #[test]
    fn forced_moves_are_never_penalized() {
        // Even a huge eval drop is not penalized when it was the only legal move.
        let (cpl, class) = classify_move(5_000, true);
        assert_eq!(cpl, 0);
        assert_eq!(class, MoveClass::Best);

        let (cpl, class) = classify_move(5_000, false);
        assert_eq!(cpl, 5_000);
        assert_eq!(class, MoveClass::Blunder);
    }

    #[test]
    fn pov_flips_for_black() {
        assert_eq!(pov(120, Color::White), 120);
        assert_eq!(pov(120, Color::Black), -120);
    }

    #[test]
    fn bucket_thresholds() {
        assert_eq!(bucket(DECISIVE_CP + 1), Bucket::White);
        assert_eq!(bucket(DECISIVE_CP), Bucket::Balanced);
        assert_eq!(bucket(0), Bucket::Balanced);
        assert_eq!(bucket(-DECISIVE_CP), Bucket::Balanced);
        assert_eq!(bucket(-DECISIVE_CP - 1), Bucket::Black);
    }

    #[test]
    fn format_eval_handles_mate_and_centipawns() {
        assert_eq!(format_eval(0), "+0.00");
        assert_eq!(format_eval(193), "+1.93");
        assert_eq!(format_eval(-40), "-0.40");
        assert_eq!(format_eval(MATE_THRESHOLD), "+M");
        assert_eq!(format_eval(MATE_CP), "+M");
        assert_eq!(format_eval(-MATE_THRESHOLD), "-M");
        assert_eq!(format_eval(-MATE_CP), "-M");
    }

    #[test]
    fn win_percent_is_centered_and_monotonic() {
        assert!((win_percent(0) - 50.0).abs() < 1e-9);
        assert!(win_percent(400) > 50.0);
        assert!(win_percent(-400) < 50.0);
        assert!(win_percent(400) > win_percent(0));
        assert!(win_percent(-400) < win_percent(0));
    }

    #[test]
    fn accuracy_from_loss_is_full_at_zero_and_clamped() {
        assert!((accuracy_from_loss(0.0) - 100.0).abs() < 1e-3);
        assert_eq!(accuracy_from_loss(1000.0), 0.0);
    }
}
