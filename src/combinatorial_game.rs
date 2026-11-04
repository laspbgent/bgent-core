use std::collections::HashSet;

pub trait Pure : Clone + Eq + std::hash::Hash + std::fmt::Debug {}

/// The rules of games in general. Visible to the rules agent and to player
/// agents. Used by the rules agent to implement games.

/// A trait for a Combinatorial game, an object which defines a game through
/// code and parameters, and provides an interface to play the game through the
/// required methods below.
/// 
/// Implementations of this trait will specify X, M, but otherwise these
/// types can be chosen without restriction.
pub trait CombinatorialGame<X: Pure, M: Pure> : Pure {
  /// A base vector of moves, independent of state. Filtered by get_moves.
  /// Exposed to give players a better idea of the rules of the game from the
  /// interface.
  fn get_moves(&self) -> Vec<M>;

  /// Vector of weights into self.get_moves. values > 0.f indicate legal moves.
  /// Exact values can be used to indicate an implicit "score" to certain moves if
  /// the inner game logic should reveal that.
  fn get_move_legality(&self, state: &X) -> Vec<f32>;

  /// Returns the state resulting from applying a move to a given state.
  /// If mv is not legal, try_move(g, x, mv) will return x.
  fn try_move(&self, state: &X, mv: &M) -> X;

  /// Return a readable description of a state/move pair.
  fn describe_move(&self, state: &X, mv: &M) -> String;

  /// Returns whether this state is a terminal state. By convention, these
  /// should not have any legal moves, but the output of get_moves is ignored
  /// when is_terminal returns true.
  fn is_terminal(&self, state: &X) -> bool;

  /// Returns the initial state.
  fn init_game(&self) -> X;

  /// Get the player that is active on this current turn. Can also be thought of
  /// as the "color" of the current state. For games with 0 players, this should
  /// always return 0. For games where multiple players have a move available at
  /// a given time, special "player" codes should be given to these combinations
  /// if moves, such as a game with players 0, 1, 2 having "players" 3 4 5 6 7 8
  /// 9 10 11 12 reserved for the possible combinations of players which can
  /// move on a given turn.
  fn get_player(&self, state: &X) -> usize;

  /// Get the total number of individual players who can win. This is the length
  /// of the score vector returned by self.evaluate_state.
  fn get_num_players(&self) -> usize;

  /// Evaluate the "score" of a given state for each player.
  /// 
  /// If self.is_terminal(state) is true, the highest scoring player, if any,
  /// is the winner. In the 1 or 0 player case, the game is played to maximize
  /// a single score.
  fn evaluate_state(&self, state: &X) -> Vec<f32>;
}

/// An optional extension for games with randomness, implemented by generated
/// games when their rules need it.
#[allow(dead_code)]
pub trait CombinatorialGameNondeterministic<X: Pure, M: Pure>: CombinatorialGame<X, M> {
  /// For games with randomness, if get_move_legality_open(g, x)[i] > get_move_legality_open(g, x)[j]
  /// move i is more likely to be legal in get_move_legality(g, x) than move j.
  fn get_move_legality_open(&self, state: &X) -> Vec<f32>;
}

/// A bounded exploration of the state tree of a game, used by the server to
/// answer exploration requests from agents.
pub(crate) struct GameExplorationGraph<G: CombinatorialGame<X, M>, X: Pure, M: Pure> {
  pub(crate) game: G, //Clone required to allow for multiple exploration threads.
  pub(crate) states: HashSet<X>,
  pub(crate) edges: Vec<(X, X, M)>, // (from_state, to_state, move)
  pub(crate) max_states: usize
}

impl<G: CombinatorialGame<X, M>, X: Pure, M: Pure> GameExplorationGraph<G, X, M> {
  /// Explore the graph up to `depth` from the starting state `start`
  ///
  /// Returns true if the graph's maximum capacity was reached.
  pub(crate) fn explore(&mut self, start: &X, depth: usize) -> bool{
    let mut surface = HashSet::new();
    surface.insert(start.clone());
    self.states.insert(start.clone());

    if self.states.len() > self.max_states {
      return true;
    }

    for _ in 0..depth {
      let old_surface = surface.clone();
      let mut next_surface = HashSet::new();

      for x in old_surface {
        let moves = self.game.get_moves();
        let legality = self.game.get_move_legality(&x);

        for (mv, weight) in moves.iter().zip(legality.iter()) {
          if *weight <= 0.0 {
            continue;
          }
          let next_x = self.game.try_move(&x, mv);
          next_surface.insert(next_x.clone());
          self.states.insert(next_x.clone());
          self.edges.push((x.clone(), next_x.clone(), mv.clone()));

          if self.states.len() > self.max_states {
            return true;
          }
        }
      }
    }

    return false;
  }
}
