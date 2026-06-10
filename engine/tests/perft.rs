//! Move-generation correctness via perft node counts on standard positions,
//! plus FEN round-trip and Zobrist-hash integrity checks.
//!
//! Run with `cargo test -p chess-engine --release` for reasonable speed.

use chess_engine::{compute_hash, fen, movegen, perft, Position};

fn pos(fen_str: &str) -> Position {
    fen::parse(fen_str).expect("valid FEN")
}

fn check(fen_str: &str, expected: &[(u32, u64)]) {
    let mut p = pos(fen_str);
    for &(depth, nodes) in expected {
        let got = perft(&mut p, depth);
        assert_eq!(
            got, nodes,
            "perft({}) for `{}` = {}, expected {}",
            depth, fen_str, got, nodes
        );
    }
}

#[test]
fn perft_startpos() {
    check(
        fen::START_FEN,
        &[(1, 20), (2, 400), (3, 8902), (4, 197281), (5, 4865609)],
    );
}

#[test]
fn perft_kiwipete() {
    check(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[(1, 48), (2, 2039), (3, 97862), (4, 4085603)],
    );
}

#[test]
fn perft_position3() {
    check(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[(1, 14), (2, 191), (3, 2812), (4, 43238), (5, 674624)],
    );
}

#[test]
fn perft_position4() {
    check(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[(1, 6), (2, 264), (3, 9467), (4, 422333)],
    );
}

#[test]
fn perft_position5() {
    check(
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[(1, 44), (2, 1486), (3, 62379)],
    );
}

#[test]
fn perft_position6() {
    check(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &[(1, 46), (2, 2079), (3, 89890)],
    );
}

#[test]
fn fen_round_trip() {
    for f in [
        fen::START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ] {
        let p = pos(f);
        assert_eq!(fen::to_fen(&p), f, "FEN round-trip mismatch");
    }
}

#[test]
fn hash_integrity_after_make_unmake() {
    // Across a few positions, every legal move must: keep the incremental hash
    // equal to a from-scratch recompute, and make+unmake must restore the
    // exact position (FEN and hash).
    for f in [
        fen::START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ] {
        let mut p = pos(f);
        let before_fen = fen::to_fen(&p);
        let before_hash = p.hash();
        assert_eq!(before_hash, compute_hash(&p), "initial hash mismatch");

        let moves = movegen::legal_moves(&mut p);
        for &m in moves.as_slice() {
            let undo = p.make_move(m);
            assert_eq!(
                p.hash(),
                compute_hash(&p),
                "incremental hash mismatch after {} in `{}`",
                m.to_uci(),
                f
            );
            p.unmake_move(undo);
            assert_eq!(fen::to_fen(&p), before_fen, "unmake changed position");
            assert_eq!(p.hash(), before_hash, "unmake changed hash");
        }
    }
}
