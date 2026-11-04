//! REFERENCE GAME — tic-tac-toe, installed by hand, not by a Rules Agent.
//!
//! Purpose: this file is what a correct run looks like. It exercises every part of
//! the module contract, builds clean, and passes the harness. Compare a Rules
//! Agent's output against it when something looks wrong.
//!
//! Board is `cells[i]` with 0 = empty, 1 = X, 2 = O. `turn` is 0 for X and 1 for O.

use crate::combinatorial_game::{CombinatorialGame, Pure};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct State {
    pub cells: [u8; 9],
    pub turn: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Move {
    /// Index 0..9, row-major.
    pub cell: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Game;

impl Pure for State {}
impl Pure for Move {}
impl Pure for Game {}

const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

/// Returns the mark (1 or 2) that owns a completed line, if any.
fn winner_of(cells: &[u8; 9]) -> Option<u8> {
    for line in LINES.iter() {
        let a = cells[line[0]];
        if a != 0 && a == cells[line[1]] && a == cells[line[2]] {
            return Some(a);
        }
    }
    None
}

fn is_draw(cells: &[u8; 9]) -> bool {
    winner_of(cells).is_none() && cells.iter().all(|c| *c != 0)
}

fn symbol(mark: u8) -> char {
    match mark {
        1 => 'X',
        2 => 'O',
        _ => '.',
    }
}

impl Game {
    /// How close `mark` is to owning a line: lines with two of its marks and an
    /// empty third square. Gives greedy playouts something to climb.
    fn line_pressure(cells: &[u8; 9], mark: u8) -> f32 {
        let mut n = 0.0;
        for line in LINES.iter() {
            let mine = line.iter().filter(|i| cells[**i] == mark).count();
            let empty = line.iter().filter(|i| cells[**i] == 0).count();
            if mine == 2 && empty == 1 {
                n += 1.0;
            }
        }
        n
    }
}

impl CombinatorialGame<State, Move> for Game {
    fn get_moves(&self) -> Vec<Move> {
        (0..9).map(|cell| Move { cell }).collect()
    }

    fn get_move_legality(&self, state: &State) -> Vec<f32> {
        let moves = self.get_moves();
        // Sized from the move list, never hard-coded: the two vectors are paired
        // by index, so a mismatch silently drops moves.
        let mut out = vec![0.0f32; moves.len()];
        if self.is_terminal(state) {
            return out;
        }
        for (i, mv) in moves.iter().enumerate() {
            out[i] = if state.cells[mv.cell] == 0 { 1.0 } else { 0.0 };
        }
        out
    }

    fn try_move(&self, state: &State, mv: &Move) -> State {
        if mv.cell >= 9 || state.cells[mv.cell] != 0 {
            return *state;
        }
        let mut next = *state;
        next.cells[mv.cell] = state.turn + 1;
        next.turn = 1 - state.turn;
        next
    }

    fn describe_move(&self, state: &State, mv: &Move) -> String {
        if mv.cell >= 9 {
            return format!("Off-board move {}", mv.cell);
        }
        format!(
            "Place {} at row {}, column {}",
            symbol(state.turn + 1),
            mv.cell / 3 + 1,
            mv.cell % 3 + 1
        )
    }

    fn is_terminal(&self, state: &State) -> bool {
        winner_of(&state.cells).is_some() || is_draw(&state.cells)
    }

    fn init_game(&self) -> State {
        State {
            cells: [0; 9],
            turn: 0,
        }
    }

    fn get_player(&self, state: &State) -> usize {
        state.turn as usize
    }

    fn get_num_players(&self) -> usize {
        2
    }

    fn evaluate_state(&self, state: &State) -> Vec<f32> {
        match winner_of(&state.cells) {
            Some(1) => return vec![1.0, 0.0],
            Some(2) => return vec![0.0, 1.0],
            _ => {}
        }
        if is_draw(&state.cells) {
            // Equal scores: nobody is the highest, so the harness reports a tie.
            return vec![0.5, 0.5];
        }
        // Non-terminal: mild pressure toward completion, scaled well under 1.0 so
        // a real win always dominates.
        vec![
            Self::line_pressure(&state.cells, 1) * 0.1,
            Self::line_pressure(&state.cells, 2) * 0.1,
        ]
    }
}

pub fn new_game() -> Game {
    Game
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_board_is_playable() {
        let g = Game;
        let s = g.init_game();
        assert!(!g.is_terminal(&s));
        assert_eq!(g.get_move_legality(&s).len(), g.get_moves().len());
        assert!(g.get_move_legality(&s).iter().all(|w| *w > 0.0));
    }

    #[test]
    fn row_win_is_terminal() {
        let g = Game;
        let mut s = g.init_game();
        for cell in [0usize, 3, 1, 4, 2] {
            s = g.try_move(&s, &Move { cell });
        }
        assert!(g.is_terminal(&s));
        assert_eq!(g.evaluate_state(&s), vec![1.0, 0.0]);
    }

    #[test]
    fn illegal_move_is_a_noop() {
        let g = Game;
        let s = g.try_move(&g.init_game(), &Move { cell: 4 });
        assert_eq!(g.try_move(&s, &Move { cell: 4 }), s);
        assert_eq!(g.get_move_legality(&s)[4], 0.0);
    }
}
