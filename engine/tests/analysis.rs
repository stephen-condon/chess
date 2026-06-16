//! Tests for post-game analysis: classification, turning points, the decided
//! move, and the annotated PGN round trip.
//!
//! Run with `cargo test -p chess-engine --release`.

use chess_engine::analysis::{analyze, MoveClass};
use chess_engine::{pgn, Game, SearchLimits, Status};

mod common;
use common::now_ms;

const LIMITS: SearchLimits = SearchLimits {
    max_depth: 4,
    time_ms: 2000,
};

#[test]
fn flags_a_hanging_queen_as_a_blunder_and_turning_point() {
    // White king e1, queen d1; black king e8, rook d8. White is up a queen
    // for nothing. Qd4 walks the queen onto the open d-file in front of the
    // rook, hanging it for free.
    let mut game = Game::from_fen("3rk3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();
    game.play_uci("d1d4").unwrap();
    game.play_uci("d8d4").unwrap();

    let report = analyze(&mut game, LIMITS, now_ms, |_, _| {});
    assert_eq!(report.moves.len(), 2);

    let blunder = &report.moves[0];
    assert_eq!(blunder.san, "Qd4");
    assert_eq!(blunder.class, MoveClass::Blunder);
    assert!(
        blunder.cpl >= 200,
        "expected a large cpl, got {}",
        blunder.cpl
    );
    assert!(
        blunder.turning_point,
        "Qd4 should flip the evaluation bucket"
    );
}

#[test]
fn fools_mate_is_decided_by_the_losing_blunder() {
    let mut game = Game::new();
    for mv in ["f3", "e5", "g4", "Qh4#"] {
        game.play_san(mv).unwrap();
    }
    assert_eq!(game.status(), Status::Checkmate(chess_engine::Color::White));

    let report = analyze(&mut game, LIMITS, now_ms, |_, _| {});
    assert_eq!(report.result, "0-1");
    assert_eq!(report.moves.len(), 4);

    // White's second move (g4, ply index 2) hangs mate in one.
    let g4 = &report.moves[2];
    assert_eq!(g4.san, "g4");
    assert_eq!(g4.class, MoveClass::Blunder);
    assert!(
        g4.decided_game,
        "g4 should be marked as the decisive blunder"
    );

    // Exactly one move is marked as deciding the game.
    assert_eq!(report.moves.iter().filter(|m| m.decided_game).count(), 1);
}

#[test]
fn annotated_pgn_round_trips_through_from_pgn() {
    let mut game = Game::new();
    for mv in ["e4", "e5", "Nf3", "Nc6"] {
        game.play_san(mv).unwrap();
    }
    let original_san = game.san_history().to_vec();

    let report = analyze(&mut game, LIMITS, now_ms, |_, _| {});
    assert!(report.annotated_pgn.contains("[WhiteAccuracy "));
    assert!(report.annotated_pgn.contains("[BlackAccuracy "));

    let mut replayed = pgn::from_pgn(&report.annotated_pgn).expect("annotated PGN should parse");
    assert_eq!(replayed.san_history(), original_san.as_slice());
    assert_eq!(replayed.status(), game.status());
}

#[test]
fn handles_a_single_ply_game() {
    let mut game = Game::new();
    game.play_san("e4").unwrap();

    let mut calls = Vec::new();
    let report = analyze(&mut game, LIMITS, now_ms, |done, total| {
        calls.push((done, total))
    });

    assert_eq!(report.moves.len(), 1);
    assert_eq!(calls, vec![(1, 1)]);
    // Should still produce a parseable PGN.
    let replayed = pgn::from_pgn(&report.annotated_pgn).expect("annotated PGN should parse");
    assert_eq!(replayed.san_history(), game.san_history());
}

#[test]
fn handles_custom_start_position_with_setup_tag() {
    let custom_fen = "8/8/8/4k3/8/8/8/4K2R w K - 0 1";
    let mut game = Game::from_fen(custom_fen).unwrap();
    game.play_san("Kf2").unwrap();
    game.play_san("Kd5").unwrap();

    let report = analyze(&mut game, LIMITS, now_ms, |_, _| {});
    assert!(report.annotated_pgn.contains("[SetUp \"1\"]"));
    assert!(report
        .annotated_pgn
        .contains(&format!("[FEN \"{}\"]", custom_fen)));

    let replayed = pgn::from_pgn(&report.annotated_pgn).expect("annotated PGN should parse");
    assert_eq!(replayed.san_history(), game.san_history());
}

#[test]
fn stalemate_with_no_moves_is_a_draw_with_no_decided_move() {
    // Black to move, not in check, no legal moves.
    let mut game = Game::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(game.status(), Status::Stalemate);

    let report = analyze(&mut game, LIMITS, now_ms, |_, _| {});
    assert_eq!(report.result, "1/2-1/2");
    assert!(report.moves.is_empty());
    assert_eq!(report.white.accuracy, 100.0);
    assert_eq!(report.black.accuracy, 100.0);
    assert!(report.moves.iter().all(|m| !m.decided_game));

    // Still a parseable PGN with the SetUp/FEN tags for the non-standard start.
    assert!(report.annotated_pgn.contains("[SetUp \"1\"]"));
    assert!(report.annotated_pgn.contains("1/2-1/2"));
    let replayed = pgn::from_pgn(&report.annotated_pgn).expect("annotated PGN should parse");
    assert!(replayed.san_history().is_empty());
}
