#!/usr/bin/env bash
# Run `cargo test <arguments>` and fail unless at least <min> tests passed in total (every test binary and the
# doctests): a run that executes nothing, or loses a test file, would otherwise pass.
#
# usage: scripts/cargo_test_min.sh <min> [cargo test arguments...]
set -euo pipefail
min=${1:?usage: scripts/cargo_test_min.sh <min> [cargo test arguments...]}
shift
case "$min" in '' | *[!0-9]* | 0) echo "error: <min> must be a positive integer (got '$min')" >&2; exit 2 ;; esac
log=$(mktemp)
trap 'rm -f "$log"' EXIT
cargo test "$@" 2>&1 | tee "$log"
passed=$(sed -nE 's/^test result: ok\. ([0-9]+) passed.*/\1/p' "$log" | awk '{s += $1} END {print s + 0}')
echo "tests passed: $passed (at least $min)"
if [ "$passed" -lt "$min" ]; then
  echo "error: cargo test $* passed $passed tests, fewer than $min" >&2
  exit 1
fi
