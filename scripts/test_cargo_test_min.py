#!/usr/bin/env python3
"""Tests for scripts/cargo_test_min.sh: it passes when the test results add up to at least the minimum, and fails
when they do not, when nothing ran, when cargo fails, and on a minimum that is not a positive integer. A fake
`cargo` on PATH prints the result lines, so nothing is built."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "cargo_test_min.sh"


def run(min_: str, out: str, code: int = 0) -> subprocess.CompletedProcess:
    with tempfile.TemporaryDirectory() as d:
        cargo = Path(d) / "cargo"
        cargo.write_text(f"#!/bin/sh\ncat <<'OUT'\n{out}OUT\nexit {code}\n")
        cargo.chmod(0o755)
        env = {**os.environ, "PATH": f"{d}:{os.environ['PATH']}"}
        return subprocess.run(["bash", str(SCRIPT), min_, "--release"], capture_output=True, text=True, env=env,
                              timeout=30)


RESULTS = "test result: ok. 6 passed; 0 failed\ntest result: ok. 52 passed; 0 failed\n"


class Minimum(unittest.TestCase):
    def test_enough_tests_pass(self):
        p = run("58", RESULTS)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("tests passed: 58 (at least 58)", p.stdout)

    def test_fewer_tests_fail(self):
        p = run("59", RESULTS)
        self.assertEqual(p.returncode, 1)
        self.assertIn("passed 58 tests, fewer than 59", p.stderr)

    def test_a_run_that_executed_nothing_fails(self):
        p = run("1", "running 0 tests\n")
        self.assertEqual(p.returncode, 1)

    def test_a_failing_cargo_fails(self):
        p = run("1", RESULTS, code=101)
        self.assertNotEqual(p.returncode, 0)

    def test_a_bad_minimum_fails_at_once(self):
        for m in ("0", "x", "-3"):
            self.assertEqual(run(m, RESULTS).returncode, 2, m)


if __name__ == "__main__":
    unittest.main()
