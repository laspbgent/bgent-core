//! FIXTURE — get_moves returns an empty vector
//!
//! Expected harness verdict: FAIL — errors `no_moves`, `no_successors`
//!
//! Installed with: python scripts/integrate.py --run-id fx-no_moves --from-file fixtures/no_moves.rs

use crate::combinatorial_game::{CombinatorialGame, Pure};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct State { pub n: u32, pub turn: u8 }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Move { pub id: u8 }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Game;

impl Pure for State {}
impl Pure for Move {}
impl Pure for Game {}

pub fn new_game() -> Game { Game }

impl CombinatorialGame<State, Move> for Game {
    fn get_moves(&self) -> Vec<Move> { vec![] }

    fn get_move_legality(&self, _state: &State) -> Vec<f32> { vec![] }

    fn try_move(&self, state: &State, _mv: &Move) -> State { *state }

    fn describe_move(&self, _state: &State, _mv: &Move) -> String { String::new() }

    fn is_terminal(&self, _state: &State) -> bool { false }

    fn init_game(&self) -> State { State { n: 0, turn: 0 } }

    fn get_player(&self, _state: &State) -> usize { 0 }

    fn get_num_players(&self) -> usize { 2 }

    fn evaluate_state(&self, _state: &State) -> Vec<f32> { vec![0.0, 0.0] }
}
