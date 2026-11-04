//! `bgent` — player-free entry point.
//!
//! Everything the Rules Agent's output can be checked against without a server:
//! the contract crawl and the seeded playouts. No network, no Player Agents, no
//! Design Agent.
//!
//! ```text
//! bgent check [--json] [--out FILE] [--depth N] [--states N] [--seed S]
//!             [--random-plays N] [--greedy-plays N] [--max-plies N]
//! ```
//!
//! Exit codes: 0 = no errors, 1 = at least one error found, 2 = bad usage.

use std::process::ExitCode;

use bgent::games::current::{new_game, Game, Move, State};
use bgent::harness::{crawl, Config, Report, Severity};

const USAGE: &str = "\
bgent — check a generated game without running players

USAGE:
    bgent check [OPTIONS]

OPTIONS:
    --json              emit the JSON report on stdout instead of a summary
    --out FILE          also write the JSON report to FILE
    --depth N           max crawl depth                (default 6)
    --states N          max distinct states visited    (default 20000)
    --seed S            playout seed                   (default 749079)
    --random-plays N    seeded random playouts         (default 20)
    --greedy-plays N    seeded greedy playouts         (default 5)
    --max-plies N       per-playout move cap           (default 400)
    --quiet             findings only, no summary block
    -h, --help          this text
";

struct Args {
    json: bool,
    out: Option<String>,
    quiet: bool,
    cfg: Config,
}

fn parse() -> Result<Args, String> {
    let mut a = Args { json: false, out: None, quiet: false, cfg: Config::default() };
    let mut it = std::env::args().skip(1);

    while let Some(arg) = it.next() {
        match arg.as_str() {
            // Accepted for symmetry with the documented command form.
            "check" => {}
            "-h" | "--help" => {
                print!("{}", USAGE);
                std::process::exit(0);
            }
            "--json" => a.json = true,
            "--quiet" => a.quiet = true,
            "--out" => a.out = Some(value(&mut it, "--out")?),
            "--depth" => {
                a.cfg.max_depth = value(&mut it, "--depth")?.parse().map_err(|_| "--depth must be a number")?
            }
            "--states" => {
                a.cfg.max_states = value(&mut it, "--states")?.parse().map_err(|_| "--states must be a number")?
            }
            "--seed" => a.cfg.seed = value(&mut it, "--seed")?.parse().map_err(|_| "--seed must be a number")?,
            "--random-plays" => {
                a.cfg.random_plays =
                    value(&mut it, "--random-plays")?.parse().map_err(|_| "--random-plays must be a number")?
            }
            "--greedy-plays" => {
                a.cfg.greedy_plays =
                    value(&mut it, "--greedy-plays")?.parse().map_err(|_| "--greedy-plays must be a number")?
            }
            "--max-plies" => {
                a.cfg.max_plies =
                    value(&mut it, "--max-plies")?.parse().map_err(|_| "--max-plies must be a number")?
            }
            other => return Err(format!("unknown argument: {}", other)),
        }
    }
    Ok(a)
}

fn value(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    it.next().ok_or_else(|| format!("{} needs a value", flag))
}

fn sev_tag(s: Severity) -> &'static str {
    match s {
        Severity::Error => "ERROR",
        Severity::Warning => "WARN ",
        Severity::Info => "INFO ",
    }
}

fn print_summary(r: &Report, cfg: &Config) {
    println!("game");
    println!("  players         {}", r.num_players);
    println!("  base moves      {}", r.base_move_count);
    println!("  player codes    {:?}", r.player_codes_seen);

    println!("crawl");
    println!("  states visited  {}{}", r.states_visited, if r.crawl_truncated { " (cap hit)" } else { "" });
    println!("  edges examined  {}", r.edges_examined);
    println!("  terminal states {}", r.terminal_states_found);
    println!("  depth reached   {}", r.reachable_depth);
    println!("  distinct scores {}", r.distinct_scores);

    println!("playouts  (seed {})", cfg.seed);
    let done = r.playouts.iter().filter(|p| p.reached_terminal).count();
    println!("  reached a terminal state {}/{}", done, r.playouts.len());
    for p in &r.playouts {
        let end = if p.reached_terminal {
            match p.winner {
                Some(w) => format!("won by player {}", w),
                None => "tied / no winner".to_string(),
            }
        } else if p.hit_ply_cap {
            format!("stopped at the {}-move cap", cfg.max_plies)
        } else {
            "stalled: no legal moves but not terminal".to_string()
        };
        println!("  {:<10} {:>4} plies  {}", p.label, p.plies, end);
    }

    println!();
    if r.findings.is_empty() {
        println!("no findings");
    } else {
        for f in &r.findings {
            println!("{} [{}] {}", sev_tag(f.severity), f.code, f.detail);
        }
    }
    println!();
    println!(
        "{} error(s), {} warning(s) — {}",
        r.errors(),
        r.warnings(),
        if r.ok() { "PASS" } else { "FAIL" }
    );
}

fn main() -> ExitCode {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}\n\n{}", e, USAGE);
            return ExitCode::from(2);
        }
    };

    let game: Game = new_game();
    let report = crawl::<Game, State, Move>(&game, &args.cfg);
    let json = report.to_json(args.cfg.seed);

    if let Some(path) = &args.out {
        if let Err(e) = std::fs::write(path, &json) {
            eprintln!("error: could not write {}: {}", path, e);
            return ExitCode::from(2);
        }
    }
    if args.json {
        print!("{}", json);
    } else if args.quiet {
        // Findings only: for CI logs, where the summary block is noise.
        for f in &report.findings {
            println!("{} [{}] {}", sev_tag(f.severity), f.code, f.detail);
        }
    } else {
        print_summary(&report, &args.cfg);
    }

    if report.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
