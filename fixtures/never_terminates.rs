//! FIXTURE — no terminal state exists; play can only end by the ply cap
//!
//! Expected harness verdict: WARN — `no_terminal_reachable`
//!
//! Installed with: python scripts/integrate.py --run-id fx-never_terminates --from-file fixtures/never_terminates.rs

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
    fn get_moves(&self) -> Vec<Move> { vec![Move { id: 0 }, Move { id: 1 }] }

    fn get_move_legality(&self, _state: &State) -> Vec<f32> { vec![1.0f32; 2] }

    fn try_move(&self, state: &State, mv: &Move) -> State {
        State { n: state.n + 1 + mv.id as u32, turn: 1 - state.turn }
    }

    fn describe_move(&self, _state: &State, mv: &Move) -> String {
        format!("Move {}", mv.id)
    }

    fn is_terminal(&self, _state: &State) -> bool { false }

    fn init_game(&self) -> State { State { n: 0, turn: 0 } }

    fn get_player(&self, state: &State) -> usize { state.turn as usize }

    fn get_num_players(&self) -> usize { 2 }

    fn evaluate_state(&self, _state: &State) -> Vec<f32> { vec![0.0, 0.0] }
}
