# Checks no ci / fuzz workflow runs, spliced into scripts/preflight.sh by its generator.
# The public text check (.github/workflows/public-text.yml) runs ALICE-Law's checker at
# the pinned revision over the tree and the commits since main.

# @quick
step "eol.yml / line endings (tests, then check)"
# text files are LF in the index and the working tree (.github/workflows/eol.yml, shared by the ALICE crates)
python3 scripts/test_check_eol.py
python3 scripts/check_eol.py

step "public-text.yml / tests of scripts/public_text.sh (no network)"
python3 scripts/test_public_text.py

step "public-text.yml / tests of scripts/cargo_test_min.sh (no build)"
python3 scripts/test_cargo_test_min.py

step "public-text.yml / public text (tree at HEAD, then the commits since main)"
# without origin/main (a new repository) the base is the root commit
scripts/public_text.sh "$(git merge-base origin/main HEAD 2>/dev/null || git rev-list --max-parents=0 HEAD | tail -1)"
