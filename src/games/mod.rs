//! Game modules.
//!
//! REWRITTEN BY scripts/integrate.py — do not edit by hand.
//!
//! Only the current run's game is compiled. Everything else lives in the
//! `code-artefacts` repo and is never built, which is why one broken historical
//! run cannot break this crate.
//!
//! `current.rs` must define, at minimum:
//!   State, Move and Game (as type aliases, structs or enums)
//!   pub fn new_game() -> Game
//!   impl CombinatorialGame<State, Move> for Game

pub mod current;
