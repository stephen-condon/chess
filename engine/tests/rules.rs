//! Tests for game status, draw detection, SAN, and PGN.

use chess_engine::{pgn, Color, DrawReason, Game, Status};

#[test]
fn detects_fools_mate() {
    let mut g = Game::new();
    for mv in ["f3", "e5", "g4", "Qh4"] {
        g.play_san(mv).expect(mv);
    }
    assert_eq!(g.status(), Status::Checkmate(Color::White));
    // The mating move should be rendered with '#'.
    assert_eq!(g.san_history().last().unwrap(), "Qh4#");
}

#[test]
fn detects_scholars_mate() {
    let mut g = Game::new();
    for mv in ["e4", "e5", "Bc4", "Nc6", "Qh5", "Nf6", "Qxf7"] {
        g.play_san(mv).expect(mv);
    }
    assert_eq!(g.status(), Status::Checkmate(Color::Black));
}

#[test]
fn detects_stalemate() {
    // Classic king+queen stalemate: black king on a8, white queen c7, white king.
    let mut g = Game::from_fen("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(g.status(), Status::Stalemate);
}

#[test]
fn detects_insufficient_material() {
    let mut g = Game::from_fen("8/8/8/4k3/8/8/4K3/8 w - - 0 1").unwrap();
    assert_eq!(g.status(), Status::Draw(DrawReason::InsufficientMaterial));

    let mut kb = Game::from_fen("8/8/8/4k3/8/8/3BK3/8 w - - 0 1").unwrap();
    assert_eq!(kb.status(), Status::Draw(DrawReason::InsufficientMaterial));
}

#[test]
fn detects_fifty_move_rule() {
    let mut g = Game::from_fen("8/8/4k3/8/8/4K3/8/6R1 w - - 100 80").unwrap();
    assert_eq!(g.status(), Status::Draw(DrawReason::FiftyMove));
}

#[test]
fn detects_threefold_repetition() {
    let mut g = Game::new();
    // Shuffle knights back and forth to repeat the start position three times.
    for mv in ["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"] {
        g.play_san(mv).expect(mv);
    }
    assert_eq!(g.status(), Status::Draw(DrawReason::Repetition));
}

#[test]
fn san_disambiguation() {
    // Knights on c3 and g1 can both reach e2; file disambiguates.
    let mut g = Game::from_fen("4k3/8/8/8/8/2N5/8/4K1N1 w - - 0 1").unwrap();
    let san = g.play_san("Nce2").expect("Nce2 legal");
    assert_eq!(san, "Nce2");
}

#[test]
fn san_castling_and_promotion() {
    let mut g = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    assert_eq!(g.play_san("O-O").unwrap(), "O-O");

    // Promotion that is not check: black king on e5 is off all of a8's lines.
    let mut p = Game::from_fen("8/P7/8/4k3/8/8/8/6K1 w - - 0 1").unwrap();
    assert_eq!(p.play_san("a8=Q").unwrap(), "a8=Q");
}

#[test]
fn pgn_round_trip_from_played_game() {
    let moves = ["e4", "e5", "Nf3", "Nc6", "Bb5", "a6", "Ba4", "Nf6", "O-O"];
    let mut g = Game::new();
    for m in moves {
        g.play_san(m).expect(m);
    }
    let pgn_text = pgn::to_pgn(&mut g, None);

    let reparsed = pgn::from_pgn(&pgn_text).expect("reparse PGN");
    assert_eq!(reparsed.san_history(), g.san_history());
    assert_eq!(reparsed.fen(), g.fen());
}

#[test]
fn pgn_import_with_numbers_and_comments() {
    let pgn_text = "1. e4 {best by test} e5 2. Nf3 (2. f4 exf4) Nc6 3. Bb5 a6 *";
    let g = pgn::from_pgn(pgn_text).expect("import");
    assert_eq!(
        g.san_history(),
        &["e4", "e5", "Nf3", "Nc6", "Bb5", "a6"]
    );
}
