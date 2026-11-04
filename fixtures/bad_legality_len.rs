//! FIXTURE — get_move_legality returns fewer weights than there are moves
//!
//! Expected harness verdict: FAIL — error `legality_length`, plus warning `no_terminal_reachable`
//! (this fixture also never terminates — a property of the
//! fixture, not the planted bug)
//!
//! Installed with: python scripts/integrate.py --run-id fx-bad_legality_len --from-file fixtures/bad_legality_len.rs

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
        // BUG: hard-coded length, one short. The third move becomes unreachable
        // and no caller notices, because the vectors are paired by index.
        if self.is_terminal(state) { return vec![0.0, 0.0]; }
        vec![1.0, 1.0]
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
        if self.is_terminal(state) { vec![state.n as f32 % 2.0, (state.n + 1) as f32 % 2.0] }
        else { vec![0.0, 0.0] }
    }
}
