//! Player-free test harness.
//!
//! Everything here works without a server, without Player Agents, and without the
//! network. It exists so a generated game can be judged on its own terms:
//! structural contract checks, a bounded crawl of the state graph, and seeded
//! random/greedy playouts. Deterministic given a seed, so a failure is reportable.
//!
//! Two deliberate differences from `GameExplorationGraph` in the contract file,
//! because the contract's version has known gaps: this harness does *not* expand
//! terminal states, and it treats NaN as illegal rather than legal.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::combinatorial_game::{CombinatorialGame, Pure};

/// Deterministic, dependency-free PRNG (xorshift64*). Seeded so a failing playout
/// can be reproduced exactly from the report.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Seed 0 is a fixed point for xorshift; nudge it.
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { return 0; }
        (self.next_u64() % n as u64) as usize
    }
}

/// A move counts as legal only if its weight is a finite value above zero.
/// NaN fails this test, which is the whole point: `w > 0.0` alone lets NaN
/// through, because every comparison with NaN is false.
#[inline]
fn is_legal(w: f32) -> bool {
    w.is_finite() && w > 0.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub code: &'static str,
    pub detail: String,
}

impl Finding {
    fn err(code: &'static str, detail: impl Into<String>) -> Self {
        Finding { severity: Severity::Error, code, detail: detail.into() }
    }
    fn warn(code: &'static str, detail: impl Into<String>) -> Self {
        Finding { severity: Severity::Warning, code, detail: detail.into() }
    }
    fn info(code: &'static str, detail: impl Into<String>) -> Self {
        Finding { severity: Severity::Info, code, detail: detail.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub seed: u64,
    pub max_depth: usize,
    pub max_states: usize,
    pub random_plays: usize,
    pub greedy_plays: usize,
    pub max_plies: usize,
    /// How many concrete states to spot-check structurally during the crawl.
    pub max_checked_states: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            seed: 0xB6E17,
            max_depth: 6,
            max_states: 20_000,
            random_plays: 20,
            greedy_plays: 5,
            max_plies: 400,
            max_checked_states: 2_000,
        }
    }
}

/// One seeded playout from the initial state.
#[derive(Debug, Clone)]
pub struct Playout {
    pub label: String,
    pub plies: usize,
    pub reached_terminal: bool,
    pub hit_ply_cap: bool,
    pub final_scores: Vec<f32>,
    pub winner: Option<usize>,
    /// `(player, describe_move output)`. There is no reasoning here: the harness has
    /// no model behind it. Player Agents supply that, and only they do.
    pub moves: Vec<(usize, String)>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub states_visited: usize,
    pub edges_examined: usize,
    pub terminal_states_found: usize,
    pub crawl_truncated: bool,
    pub reachable_depth: usize,
    pub distinct_scores: usize,
    pub playouts: Vec<Playout>,
    pub num_players: usize,
    pub base_move_count: usize,
    pub player_codes_seen: Vec<usize>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Error).count()
    }
    pub fn warnings(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Warning).count()
    }
    pub fn ok(&self) -> bool {
        self.errors() == 0
    }
}

/// Crawl the reachable state graph breadth-first, checking invariants at every
/// state and edge. Uses `HashSet<X>` for the frontier, so `X: Pure` is enough.
pub fn crawl<G, X, M>(game: &G, cfg: &Config) -> Report
where
    G: CombinatorialGame<X, M>,
    X: Pure,
    M: Pure,
{
    let mut findings: Vec<Finding> = Vec::new();
    let base_moves = game.get_moves();
    let num_players = game.get_num_players();
    let start = game.init_game();

    let base_legality_len = game.get_move_legality(&start).len();
    if base_legality_len != base_moves.len() {
        findings.push(Finding::err(
            "legality_length",
            format!(
                "get_move_legality(init) returned {} weights for {} moves. These are paired by index, \
                 so a mismatch silently drops moves (short vector) or weights (long vector).",
                base_legality_len,
                base_moves.len()
            ),
        ));
    }
    if base_moves.is_empty() {
        findings.push(Finding::err(
            "no_moves",
            "get_moves() is empty, so no game can be played at all.",
        ));
    }

    // evaluate_state is defined in terms of get_num_players; check it once up front.
    let init_scores = game.evaluate_state(&start);
    if init_scores.len() != num_players {
        findings.push(Finding::err(
            "eval_len",
            format!(
                "get_num_players() is {} but evaluate_state(init) returned {} scores.",
                num_players,
                init_scores.len()
            ),
        ));
    }
    if num_players == 0 {
        findings.push(Finding::warn(
            "zero_players",
            "get_num_players() is 0: this is a solitaire score-maximisation game, not a \
             competitive one. The Player Agent 'win by highest score' rule degenerates.",
        ));
    }
    if game.is_terminal(&start) {
        findings.push(Finding::err(
            "terminal_init",
            "init_game() is already terminal, so the game cannot be played.",
        ));
    }

    let mut visited: HashSet<X> = HashSet::new();
    let mut queue: VecDeque<(X, usize)> = VecDeque::new();
    let mut edges_examined = 0usize;
    let mut terminal_states = 0usize;
    let mut truncated = false;
    let mut max_depth_reached = 0usize;
    let mut player_codes: HashSet<usize> = HashSet::new();
    let mut score_vectors: HashSet<Vec<u64>> = HashSet::new();
    let mut checked_states = 0usize;
    let mut nan_reported = 0usize;
    let mut nonfinite_reported = 0usize;
    let mut len_mismatch_reported = 0usize;
    let mut illegal_move_changed_state = 0usize;
    let mut empty_desc = 0usize;
    let mut desc_seen: HashMap<String, usize> = HashMap::new();
    let mut terminal_with_legal = 0usize;
    let mut eval_len_reported = 0usize;
    let mut eval_nonfinite_reported = 0usize;
    let mut legal_noop_reported = 0usize;
    let mut negative_weights = 0usize;

    visited.insert(start.clone());
    queue.push_back((start.clone(), 0));

    while let Some((state, depth)) = queue.pop_front() {
        if visited.len() >= cfg.max_states {
            truncated = true;
            break;
        }
        max_depth_reached = max_depth_reached.max(depth);

        let is_term = game.is_terminal(&state);
        let scores = game.evaluate_state(&state);
        if scores.len() != num_players && eval_len_reported < 5 {
            eval_len_reported += 1;
            findings.push(Finding::err(
                "eval_len",
                format!(
                    "evaluate_state returned {} scores at depth {} but get_num_players() is {}. \
                     The winner rule indexes this vector.",
                    scores.len(),
                    depth,
                    num_players
                ),
            ));
        }
        if scores.iter().any(|s| !s.is_finite()) && eval_nonfinite_reported < 5 {
            eval_nonfinite_reported += 1;
            findings.push(Finding::err(
                "eval_nonfinite",
                format!("evaluate_state returned a non-finite score at depth {}.", depth),
            ));
        }
        score_vectors.insert(scores.iter().map(|s| s.to_bits() as u64).collect());

        let pc = game.get_player(&state);
        player_codes.insert(pc);

        let legality = game.get_move_legality(&state);
        if legality.len() != base_moves.len() && len_mismatch_reported < 5 {
            len_mismatch_reported += 1;
            findings.push(Finding::err(
                "legality_length",
                format!(
                    "get_move_legality returned {} weights for {} moves at depth {}.",
                    legality.len(),
                    base_moves.len(),
                    depth
                ),
            ));
        }

        // Terminal states: the contract says get_moves output is ignored, so we do
        // not expand them. We do report the one case that is always a bug.
        if is_term {
            terminal_states += 1;
            let legal_here = legality.iter().filter(|w| is_legal(**w)).count();
            if legal_here > 0 && terminal_with_legal < 5 {
                terminal_with_legal += 1;
                findings.push(Finding::warn(
                    "terminal_with_legal_moves",
                    format!(
                        "is_terminal is true at depth {} yet get_move_legality reports {} legal \
                         moves. Callers that filter before checking is_terminal will offer moves \
                         in a finished game.",
                        depth, legal_here
                    ),
                ));
            }
            continue;
        }

        if depth >= cfg.max_depth {
            continue;
        }

        for (i, mv) in base_moves.iter().enumerate() {
            let w = match legality.get(i) {
                Some(w) => *w,
                // Short vector: the length error already fired above; treat the
                // missing weight as illegal rather than inventing a NaN report.
                None => continue,
            };
            if w.is_nan() {
                nan_reported += 1;
                if nan_reported <= 5 {
                    findings.push(Finding::err(
                        "legality_nan",
                        format!(
                            "get_move_legality returned NaN for move index {} at depth {}. NaN \
                             compares false to 0.0, so a `weight <= 0.0` filter lets it through as \
                             a legal move.",
                            i, depth
                        ),
                    ));
                }
                continue;
            }
            if !w.is_finite() {
                nonfinite_reported += 1;
                if nonfinite_reported <= 5 {
                    findings.push(Finding::err(
                        "legality_nonfinite",
                        format!("get_move_legality returned {} at index {} depth {}.", w, i, depth),
                    ));
                }
                continue;
            }
            if w < 0.0 {
                negative_weights += 1;
            }
            if !is_legal(w) {
                // Illegal move: try_move must be a no-op, per the contract.
                let via_illegal = game.try_move(&state, mv);
                if via_illegal != state {
                    illegal_move_changed_state += 1;
                    if illegal_move_changed_state <= 5 {
                        findings.push(Finding::err(
                            "illegal_move_advances",
                            format!(
                                "try_move changed the state for a move whose legality weight is {} \
                                 (index {}, depth {}). The contract says an illegal move returns the \
                                 state unchanged.",
                                w, i, depth
                            ),
                        ));
                    }
                }
                edges_examined += 1;
                continue;
            }

            if checked_states < cfg.max_checked_states {
                checked_states += 1;
                let d = game.describe_move(&state, mv);
                if d.trim().is_empty() {
                    empty_desc += 1;
                    if empty_desc <= 5 {
                        findings.push(Finding::err(
                            "empty_describe_move",
                            format!("describe_move returned an empty string for a legal move at depth {}.", depth),
                        ));
                    }
                } else {
                    *desc_seen.entry(d.clone()).or_insert(0) += 1;
                }
                // Determinism: the same (state, move) must map to the same successor.
                let a = game.try_move(&state, mv);
                let b = game.try_move(&state, mv);
                if a != b {
                    findings.push(Finding::err(
                        "nondeterministic_try_move",
                        format!(
                            "try_move returned two different successors for the same state and move \
                             at depth {}. Only an unseeded RNG causes this; seed it and record the \
                             seed in the run metadata.",
                            depth
                        ),
                    ));
                }
            }

            let next = game.try_move(&state, mv);
            edges_examined += 1;
            if next == state && !game.is_terminal(&next) && legal_noop_reported < 5 {
                legal_noop_reported += 1;
                // Self-loop on a legal move: legal only if the game genuinely has
                // pass moves, but it usually means legality and try_move disagree.
                findings.push(Finding::warn(
                    "legal_noop_move",
                    format!(
                        "A legal move (index {}, depth {}) leaves the state unchanged while the \
                         state is not terminal. Legitimate for pass moves; otherwise try_move and \
                         get_move_legality disagree.",
                        i, depth
                    ),
                ));
            }
            if visited.insert(next.clone()) {
                queue.push_back((next, depth + 1));
            }
        }
    }

    let duplicate_descs: Vec<(&String, &usize)> = desc_seen.iter().filter(|(_, c)| **c > 3).collect();
    if !duplicate_descs.is_empty() {
        findings.push(Finding::info(
            "repeated_descriptions",
            format!(
                "{} distinct descriptions were each reused more than 3 times (e.g. \"{}\" x{}). \
                 Repeated text is fine for repeated moves, but it makes the trace harder to read.",
                duplicate_descs.len(),
                duplicate_descs[0].0,
                duplicate_descs[0].1
            ),
        ));
    }
    if negative_weights > 0 {
        findings.push(Finding::info(
            "negative_weights",
            format!(
                "{} negative legality weights were seen. The contract reserves `> 0` for legal \
                 and says nothing about negatives, so their meaning is undefined — a game that \
                 relies on them will confuse any caller that only tests `> 0`.",
                negative_weights
            ),
        ));
    }
    if visited.len() < 2 {
        findings.push(Finding::err(
            "no_successors",
            "The crawl found no successor states: either every move is illegal or try_move \
             returns the input state.",
        ));
    }
    if !truncated && terminal_states == 0 {
        findings.push(Finding::warn(
            "no_terminal_reachable",
            format!(
                "No terminal state was reachable within depth {}. With max_plies at play time the \
                 game will always end by the limit, never by finishing.",
                cfg.max_depth
            ),
        ));
    }
    if truncated {
        findings.push(Finding::info(
            "crawl_truncated",
            format!(
                "Crawl stopped at the max_states cap of {}; reachable-depth {} is a lower bound.",
                cfg.max_states, max_depth_reached
            ),
        ));
    }

    let mut codes: Vec<usize> = player_codes.into_iter().collect();
    codes.sort_unstable();
    let out_of_range: Vec<usize> = codes.iter().copied().filter(|c| *c >= num_players.max(1)).collect();
    if !out_of_range.is_empty() && num_players > 1 {
        findings.push(Finding::info(
            "extra_player_codes",
            format!(
                "get_player returned codes {:?} outside 0..{}. The contract allows extra codes for \
                 'combinations of players may move', but it also means a seat cannot be derived from \
                 the code without documentation.",
                out_of_range, num_players
            ),
        ));
    }

    let playouts = run_playouts(game, cfg, &base_moves, num_players);

    Report {
        findings,
        states_visited: visited.len(),
        edges_examined,
        terminal_states_found: terminal_states,
        crawl_truncated: truncated,
        reachable_depth: max_depth_reached,
        distinct_scores: score_vectors.len(),
        playouts,
        num_players,
        base_move_count: base_moves.len(),
        player_codes_seen: codes,
    }
}

/// Seeded playouts. Random play is the floor: if a game cannot be finished by a
/// random walker, or is trivially finished by one, that is worth knowing before any
/// Player Agent is involved. Greedy play follows each player's own score one ply
/// ahead, which catches games where the evaluation contradicts the move set.
fn run_playouts<G, X, M>(game: &G, cfg: &Config, base_moves: &[M], num_players: usize) -> Vec<Playout>
where
    G: CombinatorialGame<X, M>,
    X: Pure,
    M: Pure,
{
    let mut out = Vec::new();
    let mut rng = Rng::new(cfg.seed);

    for i in 0..cfg.random_plays {
        out.push(playout(game, base_moves, num_players, cfg.max_plies, &mut rng, format!("random-{}", i), false));
    }
    for i in 0..cfg.greedy_plays {
        out.push(playout(game, base_moves, num_players, cfg.max_plies, &mut rng, format!("greedy-{}", i), true));
    }
    out
}

fn playout<G, X, M>(
    game: &G,
    base_moves: &[M],
    num_players: usize,
    max_plies: usize,
    rng: &mut Rng,
    label: String,
    greedy: bool,
) -> Playout
where
    G: CombinatorialGame<X, M>,
    X: Pure,
    M: Pure,
{
    let mut state = game.init_game();
    let mut moves: Vec<(usize, String)> = Vec::new();
    let mut plies = 0usize;

    while !game.is_terminal(&state) && plies < max_plies {
        let legality = game.get_move_legality(&state);
        let legal: Vec<usize> = (0..base_moves.len())
            .filter(|i| legality.get(*i).map(|w| is_legal(*w)).unwrap_or(false))
            .collect();
        if legal.is_empty() {
            break;
        }
        let pick = if greedy {
            let me = game.get_player(&state).min(num_players.saturating_sub(1));
            let mut best = legal[0];
            let mut best_score = f32::NEG_INFINITY;
            for idx in &legal {
                let nxt = game.try_move(&state, &base_moves[*idx]);
                let sc = game
                    .evaluate_state(&nxt)
                    .get(me)
                    .copied()
                    .unwrap_or(f32::NEG_INFINITY);
                // Ties broken by the RNG so greedy playouts are not all identical.
                if sc > best_score || (sc == best_score && rng.below(2) == 0) {
                    best_score = sc;
                    best = *idx;
                }
            }
            best
        } else {
            legal[rng.below(legal.len())]
        };

        let who = game.get_player(&state);
        let desc = game.describe_move(&state, &base_moves[pick]);
        moves.push((who, desc));
        state = game.try_move(&state, &base_moves[pick]);
        plies += 1;
    }

    let terminal = game.is_terminal(&state);
    let scores = game.evaluate_state(&state);
    let winner = if terminal && num_players > 0 {
        let mut best = f32::NEG_INFINITY;
        let mut idx = None;
        let mut tie = false;
        for (i, s) in scores.iter().take(num_players).enumerate() {
            if *s > best {
                best = *s;
                idx = Some(i);
                tie = false;
            } else if *s == best {
                tie = true;
            }
        }
        if tie { None } else { idx }
    } else {
        None
    };

    Playout {
        label,
        plies,
        reached_terminal: terminal,
        hit_ply_cap: !terminal && plies >= max_plies,
        final_scores: scores,
        winner,
        moves,
    }
}
