//! FIXTURE — get_move_legality returns NaN for one move
//!
//! Expected harness verdict: FAIL — error `legality_nan`, plus warning `no_terminal_reachable`
//!
//! Installed with: python scripts/integrate.py --run-id fx-nan_weights --from-file fixtures/nan_weights.rs

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
    fn get_moves(&self) -> Vec<Move> {
        vec![Move { id: 0 }, Move { id: 1 }, Move { id: 2 }]
    }

    fn get_move_legality(&self, state: &State) -> Vec<f32> {
        if self.is_terminal(state) { return vec![0.0; 3]; }
        // BUG: 0.0/0.0 is NaN. `weight <= 0.0` is false for NaN, so a naive
        // filter treats this move as legal.
        let broken = if state.n == 1 { 0.0f32 / 0.0f32 } else { 1.0 };
        vec![1.0, broken, 1.0]
    }

    fn try_move(&self, state: &State, mv: &Move) -> State {
        if state.n >= 20 { return *state; }
        State { n: state.n + 1 + mv.id as u32 % 2, turn: 1 - state.turn }
    }

    fn describe_move(&self, _state: &State, mv: &Move) -> String {
        format!("Move {}", mv.id)
    }

    fn is_terminal(&self, state: &State) -> bool { state.n >= 20 }

    fn init_game(&self) -> State { State { n: 0, turn: 0 } }

    fn get_player(&self, state: &State) -> usize { state.turn as usize }

    fn get_num_players(&self) -> usize { 2 }

    fn evaluate_state(&self, state: &State) -> Vec<f32> {
        if self.is_terminal(state) { vec![1.0, 0.0] } else { vec![0.0, 0.0] }
    }
}
