#!/usr/bin/env python3
"""
BGent integration step.

This is the only supported way to put generated game code into the crate. It is
deliberately boring and deliberately strict: the Rules Agent writes a single file,
and everything else about how that file becomes part of a buildable crate is decided
here, not by the agent.

What it does, in order:

  1. Verifies src/combinatorial_game.rs is byte-identical to the pinned contract.
     A drifted contract invalidates every result, so this is a hard stop.
  2. Fetches the run's game source (from a file, or from the code-artefacts
     submodule at a given ref) and statically checks it against the module
     contract before spending compiler time on it.
  3. Installs it as `current.rs` in the games module — which is the `code-artefacts`
     submodule mounted at src/games/ — and writes that module's `mod.rs` so only
     `current` is compiled. Historical runs stay in runs/ and are never declared.
  4. Builds. Non-zero exit stops here and prints the compiler output — this is
     what the canned-failure test mode exercises.
  5. Runs the player-free harness (`bgent check`) and writes report.json.

Usage
-----
    python scripts/integrate.py --run-id 20261104T1700Z-tictactoe \
        --from-file ../code-artefacts/runs/20261104T1700Z-tictactoe/game.rs

    python scripts/integrate.py --run-id <run-id> --artefact-ref run/<run-id>

There is exactly one location the compiler reads — `src/games`, the code-artefacts
submodule — so there is exactly one place a run can be installed. If you have an
artefacts checkout elsewhere, mount it at `src/games` rather than pointing here.

    python scripts/integrate.py --run-id fail-test --canned-failure

Exit codes
----------
    0  integrated, built, harness found no errors
    1  build failed, static check failed, or harness found errors
    2  usage / environment problem (bad path, drifted contract, missing cargo)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import textwrap

HERE = os.path.dirname(os.path.abspath(__file__))
CORE = os.path.dirname(HERE)

CONTRACT = os.path.join(CORE, "src", "combinatorial_game.rs")
CONTRACT_HASH = os.path.join(HERE, "contract.sha256")
# The games module is the code-artefacts submodule, mounted here. This path is not
# configurable: `pub mod games;` in lib.rs resolves to exactly this directory, so
# installing a run anywhere else would compile the old game and pass while judging the
# wrong code.
GAMES_DIR = os.path.join(CORE, "src", "games")
SUBMODULE = GAMES_DIR

GAMES_MOD_BODY = """\
//! BGent game module.
//!
//! REWRITTEN BY scripts/integrate.py — do not edit by hand.
//!
//! Only `current` is compiled. `runs/` holds every artefact ever produced and is
//! never declared here, so one broken historical run cannot break the build and
//! compile time does not grow with history.
//!
//! REWRITTEN BY scripts/integrate.py — do not edit by hand.
//!
//! `current.rs` must define, at minimum:
//!   State, Move and Game (as type aliases, structs or enums)
//!   pub fn new_game() -> Game
//!   impl CombinatorialGame<State, Move> for Game

pub mod current;
"""

# The binary has to name a concrete State/Move/Game to instantiate the harness with,
# because the contract uses generic parameters rather than associated types. Whether
# the game writes those as `pub type`, `pub struct` or `pub enum` is its own choice.
_TYPE = r"pub\s+(?:type|struct|enum)\s+{}\b"
REQUIRED_SYMBOLS = [
    ("pub State (type, struct or enum)", _TYPE.format("State")),
    ("pub Move (type, struct or enum)", _TYPE.format("Move")),
    ("pub Game (type, struct or enum)", _TYPE.format("Game")),
    ("pub fn new_game() -> Game", r"\bpub\s+fn\s+new_game\s*\("),
    ("impl CombinatorialGame<State, Move> for Game", r"\bimpl\s+[A-Za-z0-9_:<>, \n]*CombinatorialGame\s*<"),
]

# Things that would let a generated game quietly escape its own file's scope.
FORBIDDEN_PATTERNS = [
    (r"#\s*\[\s*path\s*=", "uses #[path] to pull in another file; a run must be one self-contained file"),
    (r"\binclude!\s*\(", "uses include!, which hides code outside the reviewed file"),
    (r"\bstd::process\b", "touches the process; a game must not spawn or exit anything"),
    (r"\bstd::env\b", "reads the environment; results would depend on the host, not the game"),
    (r"\bunsafe\b", "uses unsafe code; the contract has no need for it"),
]


def die(msg: str, code: int = 2) -> "NoReturn":
    print(f"\n!! {msg}", file=sys.stderr)
    sys.exit(code)


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def step(n: int, msg: str) -> None:
    print(f"\n[{n}] {msg}")


# --------------------------------------------------------------------------- #
# 1. contract integrity
# --------------------------------------------------------------------------- #
def verify_contract() -> None:
    if not os.path.isfile(CONTRACT):
        die(f"contract not found at {CONTRACT}")
    if not os.path.isfile(CONTRACT_HASH):
        die(f"pinned hash not found at {CONTRACT_HASH}")
    expected = open(CONTRACT_HASH).read().split()[0].strip()
    actual = sha256(CONTRACT)
    if actual != expected:
        die(
            "src/combinatorial_game.rs does not match the pinned contract.\n"
            f"  expected {expected}\n  actual   {actual}\n"
            "The contract is immutable: a change to it invalidates every generated game. "
            "If the change is intentional, update scripts/contract.sha256 in the same commit "
            "and treat it as a version bump of the whole system."
        )
    print(f"    contract ok ({actual[:16]}…)")


# --------------------------------------------------------------------------- #
# 2. obtain and statically check the run's source
# --------------------------------------------------------------------------- #
def canned_failure_source(run_id: str) -> str:
    """Deliberately invalid Rust, for exercising the compile gate.

    Deliberately *passes* the static check: the whole point of this fixture is to
    prove the compiler gate works, so nothing cheaper than rustc may reject it.
    """
    return textwrap.dedent(
        f"""\
        //! CANNED FAILURE artefact ({run_id}).
        //!
        //! Invalid on purpose. Exists to prove the integration step stops on a build
        //! failure and never reaches the harness or a player.

        use crate::combinatorial_game::{{CombinatorialGame, Pure}};

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
        pub struct State;

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
        pub struct Move;

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct BrokenGame;

        pub type Game = BrokenGame;

        impl Pure for State {{}}
        impl Pure for Move {{}}
        impl Pure for BrokenGame {{}}

        pub fn new_game() -> Game {{
            BrokenGame
        }}

        impl CombinatorialGame<State, Move> for BrokenGame {{
            fn get_moves(&self) -> Vec<Move> {{
                vec![Move]
            }}

            fn get_move_legality(&self, _state: &State) -> Vec<f32> {{
                // The error: `Vec<i32>` where `Vec<f32>` is required.
                let weights: Vec<i32> = vec![1];
                weights
            }}

            fn try_move(&self, state: &State, _mv: &Move) -> State {{
                *state
            }}

            fn describe_move(&self, _state: &State, _mv: &Move) -> String {{
                String::from("noop")
            }}

            fn is_terminal(&self, _state: &State) -> bool {{
                false
            }}

            fn init_game(&self) -> State {{
                State
            }}

            fn get_player(&self, _state: &State) -> usize {{
                0
            }}

            fn get_num_players(&self) -> usize {{
                1
            }}

            fn evaluate_state(&self, _state: &State) -> Vec<f32> {{
                vec![0.0]
            }}
        }}
        """
    )


def fetch_from_submodule(ref: str, run_id: str) -> str:
    """Resolve code-artefacts at `ref` and return the run's game.rs contents.

    Only needed when the submodule's working tree is not already on the desired ref;
    `--from-file` is the normal path.
    """
    if not os.path.isdir(SUBMODULE):
        die(
            f"code-artefacts submodule is not checked out at {SUBMODULE}.\n"
            "Clone with --recurse-submodules, or run:\n"
            "  git submodule update --init --recursive"
        )
    # The submodule may be checked out anywhere; we only need this one ref's file.
    r = subprocess.run(["git", "fetch", "origin", ref], cwd=SUBMODULE,
                       capture_output=True, text=True)
    target = ref
    if r.returncode == 0:
        target = "FETCH_HEAD"
    else:
        # `fetch <ref>` fails for a branch already present or a local-only ref;
        # fall back to using the ref directly.
        print(f"    (fetch of {ref} failed, trying it as a local ref)")
    r = subprocess.run(["git", "show", f"{target}:runs/{run_id}/game.rs"],
                       cwd=SUBMODULE, capture_output=True, text=True)
    if r.returncode != 0:
        die(
            f"could not read runs/{run_id}/game.rs at {ref} in code-artefacts.\n"
            f"{r.stderr.strip()}"
        )
    return r.stdout


def resolve_games_dir() -> str:
    """The directory the games module lives in: the code-artefacts submodule."""
    d = GAMES_DIR
    if not os.path.isdir(d):
        die(
            f"games module directory not found: {d}\n"
            "It is the code-artefacts submodule. Mount it with:\n"
            f"  git submodule update --init --recursive      (expects {GAMES_DIR})\n"
            "CI clones code-artefacts straight into core/src/games."
        )
    # A populated module directory is the whole precondition for compiling a game, so
    # check it is the right repository rather than an empty mount point.
    marker = os.path.join(d, "mod.rs")
    if not os.path.isfile(marker):
        die(
            f"{d} does not look like the games module (no mod.rs).\n"
            "That file is committed in code-artefacts and declares `pub mod current;`."
        )
    runs = os.path.join(d, "runs")
    if not os.path.isdir(runs):
        print(f"    note: no runs/ directory in {d}; installing a game from elsewhere")
    return d


def _function_body(src: str, name: str) -> str | None:
    """Return the text between the braces of `fn name`, or None if not found."""
    m = re.search(r"\bfn\s+" + re.escape(name) + r"\b[^{]*\{", src)
    if not m:
        return None
    depth, i = 1, m.end()
    start = i
    while i < len(src) and depth:
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
        i += 1
    return src[start:i]


def static_check(src: str) -> None:
    """Cheap structural checks, so a missing alias fails in one line instead of
    thirty lines of rustc output.

    Only *structural* problems are fatal here. Anything that is a judgement about
    the game itself — a suspiciously hard-coded legality length, an empty move list
    — is reported as a note and left to the harness, which has a better message and
    a report file. Two checks that disagree about the same fault is worse than one.
    """
    problems = []
    notes = []

    for label, pattern in REQUIRED_SYMBOLS:
        if not re.search(pattern, src):
            problems.append(f"missing `{label}`")

    for pattern, why in FORBIDDEN_PATTERNS:
        if re.search(pattern, src):
            problems.append(why)

    body = _function_body(src, "get_move_legality")
    if body is not None and "get_moves" not in body:
        notes.append(
            "get_move_legality never mentions get_moves, so its result is probably not sized "
            "from the move list. The two vectors are paired by index; the harness will say so "
            "if the lengths disagree."
        )

    if re.search(r"\bfn\s+get_moves\b[^{]*\{\s*vec!\s*\[\s*\]\s*\}", src):
        notes.append("get_moves returns an empty vector; the harness will report it")

    if problems:
        print("\n    static check failed:")
        for p in problems:
            print(f"      - {p}")
        die("the run's game.rs does not satisfy the module contract; nothing was built", 1)

    for n in notes:
        print(f"    note: {n}")
    print("    static check ok")


# --------------------------------------------------------------------------- #
# 3. install
# --------------------------------------------------------------------------- #
def install(src: str, run_id: str, games_dir: str) -> None:
    current = os.path.join(games_dir, "current.rs")
    games_mod = os.path.join(games_dir, "mod.rs")
    header = (
        "//! CURRENT GAME — installed by scripts/integrate.py.\n"
        f"//! run: {run_id}\n"
        "//! from: runs/" + run_id + "/game.rs in code-artefacts\n"
        "//!\n"
        "//! DO NOT EDIT. The next integration overwrites this file.\n\n"
    )
    with open(current, "w", encoding="utf-8") as f:
        f.write(header + src)

    with open(games_mod, "w", encoding="utf-8") as f:
        f.write(GAMES_MOD_BODY)
    rel = os.path.relpath(current, CORE)
    if not rel.replace(os.sep, "/").startswith("src/games/"):
        die(f"refusing to install outside the games module: {rel}", 2)
    print(f"    installed {rel} ({len(src)} bytes)")


# --------------------------------------------------------------------------- #
# 4/5. build and check
# --------------------------------------------------------------------------- #
def run(cmd, **kw):
    print(f"    $ {' '.join(cmd)}")
    return subprocess.run(cmd, cwd=CORE, **kw)


def build() -> None:
    r = run(["cargo", "build", "--quiet"], capture_output=True, text=True)
    if r.returncode != 0:
        print("\n---- compiler output ----", file=sys.stderr)
        print(r.stderr.rstrip(), file=sys.stderr)
        print("-------------------------", file=sys.stderr)
        die("BUILD FAILED — stopping before the harness and before any player is started", 1)
    if r.stderr.strip():
        # Warnings matter for readability, which is a stated goal for this code.
        print("    build ok, with warnings:")
        for line in r.stderr.strip().splitlines():
            print(f"      {line}")
    else:
        print("    build ok, no warnings")


def check(out_dir: str | None, seed: int) -> int:
    report_path = None
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
        report_path = os.path.join(out_dir, "report.json")

    cmd = ["cargo", "run", "--quiet", "--", "check", "--json", "--seed", str(seed)]
    if report_path:
        cmd += ["--out", report_path]
    r = run(cmd, capture_output=True, text=True)

    payload = r.stdout.strip()
    if r.returncode not in (0, 1) and not payload:
        print(f"\n---- the harness crashed (exit {r.returncode}) ----", file=sys.stderr)
        print(r.stderr.rstrip(), file=sys.stderr)
        return 1
    if not payload:
        print("\n---- harness produced no output ----", file=sys.stderr)
        print(r.stderr.rstrip(), file=sys.stderr)
        return 1

    try:
        data = json.loads(payload)
    except json.JSONDecodeError:
        print("harness output was not valid JSON:", file=sys.stderr)
        print(payload[:2000], file=sys.stderr)
        return 1

    counts = data.get("counts", {})
    print(
        f"    harness: {data['crawl']['states_visited']} states, "
        f"{data['crawl']['terminal_states']} terminal, "
        f"{counts.get('errors', 0)} error(s), {counts.get('warnings', 0)} warning(s)"
    )
    for f in data.get("findings", []):
        tag = {"error": "ERROR", "warning": "WARN ", "info": "INFO "}.get(f["severity"], "?")
        print(f"      {tag} [{f['code']}] {f['detail']}")
    if report_path:
        print(f"    report written to {os.path.relpath(report_path, CORE)}")

    return 0 if data.get("ok") else 1


# --------------------------------------------------------------------------- #
def main() -> int:
    ap = argparse.ArgumentParser(description="Integrate a BGent game artefact into the crate.")
    ap.add_argument("--run-id", required=True, help="run id, e.g. 20261104T1700Z-tictactoe")
    ap.add_argument("--from-file", help="read the game source from this path")
    ap.add_argument("--artefact-ref", help="ref in code-artefacts (default: run/<run-id>)")
    ap.add_argument("--canned-failure", action="store_true",
                    help="install deliberately invalid Rust to test the compile gate")
    ap.add_argument("--no-check", action="store_true", help="build only, skip the harness")
    ap.add_argument("--no-build", action="store_true",
                    help="verify the contract and install the file, then stop "
                         "(useful for eyeballing current.rs before spending a build)")
    ap.add_argument("--out-dir", help="where to write report.json (default: <core>/runs/<run-id>)")
    ap.add_argument("--seed", type=int, default=0x0B6E17, help="harness seed")
    a = ap.parse_args()

    if shutil.which("cargo") is None:
        die("cargo is not on PATH")

    print(f"BGent integrate — run {a.run_id}")

    step(1, "verifying the contract and the games module")
    verify_contract()
    games_dir = resolve_games_dir()

    step(2, "reading the run's game source")
    if a.canned_failure:
        src = canned_failure_source(a.run_id)
        print("    CANNED FAILURE mode: using deliberately invalid Rust")
    elif a.from_file:
        if not os.path.isfile(a.from_file):
            die(f"--from-file not found: {a.from_file}")
        src = open(a.from_file, encoding="utf-8").read()
        print(f"    read {a.from_file}")
        static_check(src)
    else:
        ref = a.artefact_ref or f"run/{a.run_id}"
        src = fetch_from_submodule(ref, a.run_id)
        print(f"    read runs/{a.run_id}/game.rs at {ref}")
        static_check(src)

    step(3, "installing into the games module")
    install(src, a.run_id, games_dir)

    if a.no_build:
        print("\nInstalled (build skipped by --no-build).")
        return 0

    step(4, "building")
    build()

    if a.no_check:
        print("\nBuild ok (harness skipped by --no-check).")
        return 0

    step(5, "running the player-free harness")
    # Resolve against the caller's cwd *now*, because cargo runs with cwd=CORE and
    # a relative --out path would land somewhere unexpected.
    out_dir = os.path.abspath(a.out_dir) if a.out_dir else os.path.join(CORE, "runs", a.run_id)
    rc = check(out_dir, a.seed)

    print()
    if rc == 0:
        print("PASS — game builds and the harness found no errors.")
    else:
        print("FAIL — game builds but the harness found errors (see findings above).")
    return rc


if __name__ == "__main__":
    sys.exit(main())
