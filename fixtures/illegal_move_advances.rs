//! FIXTURE — try_move changes state for a move whose weight is 0
//!
//! Expected harness verdict: FAIL — error `illegal_move_advances`, plus warning `no_terminal_reachable`
//!
//! Installed with: python scripts/integrate.py --run-id fx-illegal_move_advances --from-file fixtures/illegal_move_advances.rs

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
        vec![Move { id: 0 }, Move { id: 1 }]
    }

    fn get_move_legality(&self, state: &State) -> Vec<f32> {
        // Move 1 is declared illegal after ply 2...
        if state.n >= 2 { vec![1.0, 0.0] } else { vec![1.0, 1.0] }
    }

    fn try_move(&self, state: &State, mv: &Move) -> State {
        // ...but try_move applies it anyway. BUG: no legality check.
        State { n: state.n + 1 + mv.id as u32, turn: 1 - state.turn }
    }

    fn describe_move(&self, _state: &State, mv: &Move) -> String {
        format!("Move {}", mv.id)
    }

    fn is_terminal(&self, state: &State) -> bool { state.n >= 12 }

    fn init_game(&self) -> State { State { n: 0, turn: 0 } }

    fn get_player(&self, state: &State) -> usize { state.turn as usize }

    fn get_num_players(&self) -> usize { 2 }

    fn evaluate_state(&self, state: &State) -> Vec<f32> {
        if self.is_terminal(state) { vec![1.0, 0.0] } else { vec![0.0, 0.0] }
    }
}
