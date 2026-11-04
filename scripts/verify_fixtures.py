#!/usr/bin/env python3
"""
Self-test for the integration step.

Runs `integrate.py` once per fixture in `fixtures/` and asserts the harness verdict
matches the one written in that fixture's header. This is the thing to run after
touching the harness, the CLI, or the module contract: it proves the checks still
catch what they claim to catch, in both directions.

Fixtures are installed into a scratch copy of the games module, never into the real
one, so a run of this script cannot leave a deliberately-broken game in the crate.

Usage
-----
    python scripts/verify_fixtures.py              # all fixtures
    python scripts/verify_fixtures.py nan_weights  # just one

Requires cargo on PATH. Exits non-zero if any fixture disagrees with its header.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CORE = os.path.dirname(HERE)
FIXTURES = os.path.join(CORE, "fixtures")

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


def scratch_module() -> str:
    """A throwaway games module directory, so fixtures never touch the submodule."""
    d = tempfile.mkdtemp(prefix="bgent-fixtures-")
    open(os.path.join(d, "mod.rs"), "w").write("pub mod current;\n")
    open(os.path.join(d, "current.rs"), "w").write("// empty until a fixture is installed\n")
    os.makedirs(os.path.join(d, "runs"), exist_ok=True)
    return d


def run_fixture(slug: str, module_dir: str) -> tuple[int, str]:
    src = os.path.join(FIXTURES, f"{slug}.rs")
    r = subprocess.run(
        [sys.executable, os.path.join(HERE, "integrate.py"),
         "--run-id", f"fx-{slug}", "--from-file", src,
         "--modules-dir", module_dir],
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

    only = sys.argv[1:] or None
    slugs = sorted(
        f[:-3] for f in os.listdir(FIXTURES) if f.endswith(".rs") and (not only or f[:-3] in only)
    )
    if not slugs:
        raise SystemExit("no fixtures matched")

    module_dir = scratch_module()
    failures = 0
    print(f"verifying {len(slugs)} fixture(s) against a scratch games module\n")

    try:
        for slug in slugs:
            want_status, want_codes = expected_verdict(os.path.join(FIXTURES, f"{slug}.rs"))
            rc, out = run_fixture(slug, module_dir)
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
        shutil.rmtree(module_dir, ignore_errors=True)

    print()
    if failures:
        print(f"{failures} fixture(s) disagreed with their header")
        return 1
    print("all fixtures behaved as documented")
    return 0


if __name__ == "__main__":
    sys.exit(main())
