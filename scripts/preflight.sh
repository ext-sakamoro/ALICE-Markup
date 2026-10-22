#!/usr/bin/env bash
# scripts/preflight.sh — local reproduction of the CI gates before `git push`.
# Every command is the one CI runs. A step this file does not cover is a step
# that can only fail remotely — when a workflow changes, regenerate this file
# in the same commit (checks no workflow runs go in scripts/preflight.local.sh).
# Run `--quick` before every push.
#
# usage: scripts/preflight.sh [--quick]   (--quick skips the test / bench suites)
# generator rev d8e28ae2dcf08c36be130cbb16ec28e4c40619f1
# local steps: scripts/preflight.local.sh 12d7c2e663fb46a516dbf9aaca32743bff08c309
set -euo pipefail
cd "$(dirname "$0")/.."
# --- heavy slot (begin) ---
# one heavy run at a time on this machine: a lock the kernel holds (flock on $ALICE_HEAVY_SLOT_DIR/lock, default
# $HOME/.cache/alice-heavy-slot) for as long as the holder lives. A small keeper process (perl) takes it and exits
# within a second of the holder's death, so a killed holder frees the slot and no child of the holder keeps it.
# `info` only names the holder for the waiting line. A wait past ALICE_HEAVY_SLOT_TIMEOUT seconds (default 7200)
# fails; the holder's own nested runs (ALICE_HEAVY_SLOT_HELD) run inside it.
heavy_slot_dir() { printf '%s\n' "${ALICE_HEAVY_SLOT_DIR:-$HOME/.cache/alice-heavy-slot}"; }
heavy_slot_acquire() {
  local label=$1 tool=$2 d line info err limit="${ALICE_HEAVY_SLOT_TIMEOUT:-7200}"
  d=$(heavy_slot_dir)
  if [ -n "${ALICE_HEAVY_SLOT_HELD:-}" ] && [ "$(sed -n 1p "$d/info" 2>/dev/null)" = "$ALICE_HEAVY_SLOT_HELD" ] \
     && kill -0 "$ALICE_HEAVY_SLOT_HELD" 2>/dev/null; then
    return 0
  fi
  if ! err=$(mkdir -p "$d" 2>&1) || ! err=$({ : >> "$d/lock"; } 2>&1); then
    echo "heavy slot: cannot create $d: ${err:-not writable}" >&2
    return 1
  fi
  exec 8< <(exec perl -MFcntl=:flock -e '
    my ($file, $limit) = @ARGV; my $parent = getppid(); $| = 1;
    open(my $fh, ">>", $file) or do { print "error $!\n"; exit 2 };
    print "pid $$\n";
    my $t = 0;
    until (flock($fh, LOCK_EX | LOCK_NB)) {
      if ($t >= $limit) { print "timeout\n"; exit 1 }
      print "wait $t\n" if $t % 30 == 0;
      sleep 1; $t++;
      exit 3 if getppid() != $parent;
    }
    print "held\n"; close(STDOUT);
    sleep 1 while getppid() == $parent;
  ' "$d/lock" "$limit")
  while IFS= read -r line <&8; do
    case "$line" in
      "pid "*) HEAVY_SLOT_KEEPER=${line#pid } ;;
      "wait "*)
        info=$(cat "$d/info" 2>/dev/null || true)
        echo "heavy slot: waiting for $(printf '%s\n' "$info" | sed -n 3p) pid $(printf '%s\n' "$info" | sed -n 1p) ($(printf '%s\n' "$info" | sed -n 4p))" >&2
        ;;
      held)
        exec 8<&-
        printf '%s\n%s\n%s\n%s\n' "$$" "$(date +%s)" "$label" "$tool" > "$d/info"
        ALICE_HEAVY_SLOT_HELD=$$
        export ALICE_HEAVY_SLOT_HELD
        return 0
        ;;
      timeout)
        exec 8<&-
        info=$(cat "$d/info" 2>/dev/null || true)
        echo "heavy slot: waited ${limit}s for $(printf '%s\n' "$info" | sed -n 3p) pid $(printf '%s\n' "$info" | sed -n 1p); giving up (ALICE_HEAVY_SLOT_TIMEOUT=$limit)" >&2
        return 1
        ;;
      "error "*)
        exec 8<&-
        echo "heavy slot: cannot open $d/lock: ${line#error }" >&2
        return 1
        ;;
    esac
  done
  exec 8<&-
  echo "heavy slot: the lock keeper stopped (is perl on PATH?)" >&2
  return 1
}
heavy_slot_release() {
  local d
  d=$(heavy_slot_dir)
  if [ "${ALICE_HEAVY_SLOT_HELD:-}" = "$$" ] && [ -n "${HEAVY_SLOT_KEEPER:-}" ]; then
    # the info first: once the keeper is gone the next holder writes its own
    rm -f "$d/info"
    kill "$HEAVY_SLOT_KEEPER" 2>/dev/null || true
    HEAVY_SLOT_KEEPER=
  fi
}
# --- heavy slot (end) ---
heavy_slot_acquire "$(basename "$PWD")" preflight || exit 1
trap heavy_slot_release EXIT

quick=0
[[ "${1:-}" == "--quick" ]] && quick=1

step() { printf '\n\033[1;34m== %s\033[0m\n' "$*"; }
# `cargo clippy` reuses fresh `cargo check` artifacts and then lints nothing;
# touching the crate roots invalidates only this repo's fingerprints.
relint() { git ls-files | grep -E '(^|/)src/(lib|main)\.rs$' | xargs -r touch; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1 ($2)" >&2; exit 1; }; }
has_toolchain() { rustup toolchain list | grep -q "^$1"; }

# Steps CI runs that this file cannot reproduce locally (they can only fail remotely):
#   - fuzz.yml:fuzz:Fuzz for 5 minutes [target=parse] (runner-only (open-ended or unprovable-bounded fuzz run))
#   - fuzz.yml:fuzz:Fuzz for 5 minutes [target=split_more] (runner-only (open-ended or unprovable-bounded fuzz run))


step "ci.yml / clippy: Clippy (all targets)"
relint
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo clippy --all-targets -- -D warnings )

step "ci.yml / targets: Build (thumbv7em-none-eabihf)"
rustup target list --installed | grep -q '^thumbv7em-none-eabihf$' || rustup target add thumbv7em-none-eabihf
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo build --lib --target thumbv7em-none-eabihf )

step "ci.yml / targets: Clippy (thumbv7em-none-eabihf)"
relint
rustup target list --installed | grep -q '^thumbv7em-none-eabihf$' || rustup target add thumbv7em-none-eabihf
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo clippy --lib --target thumbv7em-none-eabihf -- -D warnings )

step "ci.yml / targets: Build (wasm32-unknown-unknown)"
rustup target list --installed | grep -q '^wasm32-unknown-unknown$' || rustup target add wasm32-unknown-unknown
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo build --lib --target wasm32-unknown-unknown )

step "ci.yml / fmt: Check formatting"
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo fmt -- --check )

step "ci.yml / doc: Doc (warnings are errors)"
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings" RUSTDOCFLAGS="-D warnings"; cargo doc --no-deps )

step "ci.yml / fuzz-build: Build fuzz targets"
if has_toolchain nightly && cargo +nightly fuzz --version >/dev/null 2>&1; then
  ( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo +nightly build --manifest-path fuzz/Cargo.toml --bins )
else
  echo 'PREFLIGHT-SKIP: ci.yml / fuzz-build: Build fuzz targets: nightly / cargo-fuzz not installed' >&2
fi

step "fuzz.yml / build every fuzz target (nightly; the replay needs the runner)"
if has_toolchain nightly && cargo +nightly fuzz --version >/dev/null 2>&1; then
  (cd fuzz && cargo +nightly fuzz build)
else
  echo 'PREFLIGHT-SKIP: fuzz.yml / build every fuzz target: nightly / cargo-fuzz not installed' >&2
fi

step "public-text.yml / tests of scripts/public_text.sh (no network)"
python3 scripts/test_public_text.py

step "public-text.yml / public text (tree at HEAD, then the commits since main)"
# without origin/main (a new repository) the base is the root commit
scripts/public_text.sh "$(git merge-base origin/main HEAD 2>/dev/null || git rev-list --max-parents=0 HEAD | tail -1)"

if [[ $quick -eq 1 ]]; then
  echo; echo "preflight --quick OK (test / bench suites skipped)"; exit 0
fi

step "ci.yml / test: Test"
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo test )

step "ci.yml / test: Test (release)"
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo test --release )

step "ci.yml / doc: Doc tests"
( export CARGO_TERM_COLOR="always" RUSTFLAGS="-Dwarnings"; cargo test --doc )

echo; echo "preflight OK"
