//! BGent core crate.
//!
//! `combinatorial_game` is the game contract. It is byte-for-byte identical to the
//! authoritative file and is hash-pinned in `scripts/contract.sha256`; the integration
//! script refuses to run if it has drifted.
//!
//! `games` is a **git submodule**: the `code-artefacts` repository is mounted at
//! `src/games`. Game code is therefore part of this crate's module tree, and a Rules
//! Agent's output is compiled by the same build as the harness that judges it. A clone
//! without `--recurse-submodules` fails at `pub mod games;` below, which is the
//! intended error: there is no game to compile.
//!
//! `harness` is a player-free test surface: it explores the state graph, checks the
//! contract's invariants, and plays seeded random/greedy games.

// The contract is an immutable, externally-authored file: it is not ours to tidy. It
// carries a stray blank line inside a doc comment, an unused exploration type that the
// harness deliberately does not use, and a needless `return`. Silencing those here keeps
// the byte-for-byte guarantee intact while still letting `-D warnings` mean something
// for every file we do own.
#[allow(
    dead_code,
    clippy::empty_line_after_doc_comments,
    clippy::needless_return
)]
pub mod combinatorial_game;
pub mod games;
pub mod harness;
pub mod report;
