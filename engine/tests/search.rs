//! Tests for evaluation symmetry and search (mate-finding, tactics, self-play).
//!
//! Run with `cargo test -p chess-engine --release`.

use chess_engine::{
    compute_hash, eval::evaluate, fen, movegen, search, Game, SearchLimits,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Vertically flip the board and swap colors, producing the mirror position
/// from the other side's point of view.
fn mirror_fen(fen_str: &str) -> String {
    let parts: Vec<&str> = fen_str.split_whitespace().collect();
    let ranks: Vec<String> = parts[0]
        .split('/')
        .rev()
        .map(|r| r.chars().map(swap_case).collect())
        .collect();
    let board = ranks.join("/");

    let side = if parts[1] == "w" { "b" } else { "w" };

    let castling: String = if parts[2] == "-" {
        "-".to_string()
    } else {
        parts[2].chars().map(swap_case).collect()
    };

    let ep = if parts[3] == "-" {
        "-".to_string()
    } else {
        let bytes = parts[3].as_bytes();
        let file = bytes[0] as char;
        let rank = bytes[1] - b'0';
        format!("{}{}", file, 9 - rank)
    };

    format!("{} {} {} {} 0 1", board, side, castling, ep)
}

fn swap_case(c: char) -> char {
    if c.is_ascii_uppercase() {
        c.to_ascii_lowercase()
    } else if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

#[test]
fn eval_start_is_balanced() {
    let pos = fen::parse(fen::START_FEN).unwrap();
    assert_eq!(evaluate(&pos), 0);
}

#[test]
fn eval_is_symmetric() {
    for f in [
        fen::START_FEN,
        "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR w KQkq - 0 1",
    ] {
        let a = fen::parse(f).unwrap();
        let b = fen::parse(&mirror_fen(f)).unwrap();
        assert_eq!(
            evaluate(&a),
            evaluate(&b),
            "eval not symmetric for `{}` vs mirror `{}`",
            f,
            mirror_fen(f)
        );
    }
}

#[test]
fn finds_back_rank_mate_in_one() {
    let pos = fen::parse("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
    let res = search(
        &pos,
        SearchLimits {
            max_depth: 4,
            time_ms: 2000,
        },
        now_ms,
    );
    assert_eq!(res.best_move.unwrap().to_uci(), "a1a8");
    assert!(res.score > 29_000, "expected mate score, got {}", res.score);
}

#[test]
fn wins_hanging_queen() {
    // Black queen on d4 is defended by nothing; white rook on d2 takes it.
    let pos = fen::parse("4k3/8/8/8/3q4/8/3R4/3K4 w - - 0 1").unwrap();
    let res = search(
        &pos,
        SearchLimits {
            max_depth: 6,
            time_ms: 2000,
        },
        now_ms,
    );
    assert_eq!(res.best_move.unwrap().to_uci(), "d2d4");
}

#[test]
fn opening_move_is_reasonable() {
    let pos = fen::parse(fen::START_FEN).unwrap();
    let res = search(
        &pos,
        SearchLimits {
            max_depth: 6,
            time_ms: 3000,
        },
        now_ms,
    );
    let sane = ["e2e4", "d2d4", "c2c4", "g1f3", "b1c3", "e2e3", "d2d3", "g2g3"];
    let mv = res.best_move.unwrap().to_uci();
    assert!(sane.contains(&mv.as_str()), "unexpected opening move {}", mv);
}

#[test]
fn self_play_stays_legal_and_consistent() {
    let mut game = Game::new();
    for _ in 0..50 {
        if game.status().is_over() {
            break;
        }
        let res = search(
            game.position(),
            SearchLimits {
                max_depth: 4,
                time_ms: 200,
            },
            now_ms,
        );
        let mv = res.best_move.expect("a move while game is ongoing");
        // The chosen move must be legal in the current position.
        let mut probe = game.position().clone();
        assert!(
            movegen::legal_moves(&mut probe).as_slice().contains(&mv),
            "search returned an illegal move"
        );
        game.play(mv).expect("play searched move");
        // Hash integrity must hold after every move.
        assert_eq!(game.position().hash(), compute_hash(game.position()));
    }
}
