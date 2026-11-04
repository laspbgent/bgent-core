//! BGent core crate.
//!
//! `combinatorial_game` is the game contract. It is byte-for-byte identical to the
//! authoritative file and is hash-pinned in `scripts/contract.sha256`; the
//! integration step refuses to run if it has drifted.
//!
//! `games::current` is the one generated game that is compiled. The integration
//! step rewrites it and narrows `games/mod.rs` to point at it, so no historical
//! run can break the build.
//!
//! `harness` is a player-free test surface: it explores the state graph, checks the
//! contract's invariants, and plays seeded random/greedy games. It exists so game
//! code can be validated before any Player Agent is involved.

// The contract file is byte-identical to the authoritative source and is
// hash-pinned in scripts/contract.sha256, so lints are silenced here
// rather than by editing it.
#[allow(dead_code)]
pub mod combinatorial_game;
pub mod games;
pub mod harness;
pub mod report;
