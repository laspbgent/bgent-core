#!/usr/bin/env python3
"""
Self-test for the integration step.

Runs `integrate.py` once per fixture in `fixtures/` and asserts the harness verdict
matches the one written in that fixture's header. This is the thing to run after
touching the harness, the CLI, or the module contract: it proves the checks still
catch what they claim to catch, in both directions.

Fixtures are installed into the real games module (src/games — the code-artefacts
submodule), because that is the only directory the compiler reads. The baseline game
is snapshotted first and restored afterwards, including on failure or Ctrl-C, so this
script never leaves a deliberately-broken game behind.

A consequence worth knowing: `git status` in the submodule will show modified files
while this runs. That is this script, not a mistake.

Usage
-----
    python scripts/verify_fixtures.py              # all fixtures
    python scripts/verify_fixtures.py nan_weights  # just one

Requires cargo on PATH and the games submodule mounted. Exits non-zero if any fixture
disagrees with its header.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CORE = os.path.dirname(HERE)
FIXTURES = os.path.join(CORE, "fixtures")
GAMES = os.path.join(CORE, "src", "games")

VERDICT_RE = re.compile(r"^//! Expected harness verdict:\s*(.+)$", re.MULTILINE)


def expected_verdict(path: str) -> tuple[str, list[str]]:
    header = open(path, encoding="utf-8").read(2000)
    m = VERDICT_RE.search(header)
    if not m:
        raise SystemExit(f"{os.path.basename(path)}: no 'Expected harness verdict' line")
    text = m.group(1).strip()
    status = text.split()[0].upper().rstrip("—-").strip()
    codes = re.findall(r"`([a-z_]+)`", text)
    return status, codes


def snapshot() -> dict[str, bytes]:
    """Everything integrate.py writes, so it can be put back exactly."""
    snap = {}
    for name in ("current.rs", "mod.rs"):
        p = os.path.join(GAMES, name)
        snap[name] = open(p, "rb").read() if os.path.isfile(p) else None
    return snap


def restore(snap: dict[str, bytes]) -> None:
    for name, data in snap.items():
        p = os.path.join(GAMES, name)
        if data is None:
            if os.path.isfile(p):
                os.remove(p)
        else:
            open(p, "wb").write(data)


def run_fixture(slug: str) -> tuple[int, str]:
    src = os.path.join(FIXTURES, f"{slug}.rs")
    r = subprocess.run(
        [sys.executable, os.path.join(HERE, "integrate.py"),
         "--run-id", f"fx-{slug}", "--from-file", src],
        cwd=CORE, capture_output=True, text=True,
    )
    return r.returncode, r.stdout + r.stderr


def observed(output: str) -> tuple[str, list[str]]:
    if "BUILD FAILED" in output:
        return "FAIL", ["build_failed"]
    if "static check failed" in output or "does not satisfy the module contract" in output:
        return "FAIL", ["static_check"]
    m = re.search(r"harness:\s*\d+ states,\s*\d+ terminal,\s*(\d+) error\(s\)", output)
    if not m:
        return "UNKNOWN", []
    errors = int(m.group(1))
    codes = re.findall(r"^\s+(?:ERROR|WARN |INFO )\s+\[([a-z_]+)\]", output, re.MULTILINE)
    status = "FAIL" if errors else ("WARN" if "WARN " in output else "PASS")
    return status, codes


def main() -> int:
    if not os.path.isdir(FIXTURES):
        raise SystemExit(f"no fixtures folder at {FIXTURES}")
    if not os.path.isfile(os.path.join(GAMES, "mod.rs")):
        raise SystemExit(
            f"games module not mounted at {GAMES}.\n"
            "Run: git submodule update --init --recursive"
        )

    only = sys.argv[1:] or None
    slugs = sorted(
        f[:-3] for f in os.listdir(FIXTURES) if f.endswith(".rs") and (not only or f[:-3] in only)
    )
    if not slugs:
        raise SystemExit("no fixtures matched")

    failures = 0
    print(f"verifying {len(slugs)} fixture(s) against the real games module\n")

    snap = snapshot()
    try:
        for slug in slugs:
            want_status, want_codes = expected_verdict(os.path.join(FIXTURES, f"{slug}.rs"))
            rc, out = run_fixture(slug)
            got_status, got_codes = observed(out)

            missing = [c for c in want_codes if c not in got_codes]
            want_rc = 1 if want_status == "FAIL" else 0
            ok = got_status == want_status and not missing and rc == want_rc

            mark = "ok  " if ok else "FAIL"
            print(f"{mark} {slug:26} expected {want_status:4} got {got_status:4}", end="")
            if missing:
                print(f"  missing findings: {missing}", end="")
            print()

            if not ok:
                failures += 1
                print(f"      expected codes {want_codes}, got {got_codes}, exit {rc}")
                for line in out.strip().splitlines()[-14:]:
                    print(f"      | {line}")
    finally:
        restore(snap)
        print(f"\nbaseline game restored ({sum(1 for v in snap.values() if v is not None)} files)")

    print()
    if failures:
        print(f"{failures} fixture(s) disagreed with their header")
        return 1
    print("all fixtures behaved as documented")
    return 0


if __name__ == "__main__":
    sys.exit(main())
