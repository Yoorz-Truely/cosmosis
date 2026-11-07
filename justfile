# Everyday commands. Run `just` to list them. The workflow, board and limits are Sau's:
# ../Sau/AGENTS.md and ../Sau/docs/agents/.

# List the recipes
default:
    @just --list

# The commit gate for code: format, lint, test and the licence gate, through the build guard
check:
    #!/usr/bin/env bash
    # One script, so the check's start and its pass share a key ($$). Only a pass that runs to
    # the end is recorded; a code commit needs one on its tree (tools/check-record.py, task 0-10).
    set -euo pipefail
    python3 tools/check-record.py start $$
    python3 tools/check-record.py selftest
    python3 tools/build-guard.py wait
    cargo fmt --check
    # The pre-commit fixers' formatters in check mode, so a tree that passes is one they leave alone.
    git ls-files -co --exclude-standard -z -- '*.toml' | RUST_LOG=warn xargs -0 taplo fmt --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo deny --offline check licenses bans sources
    python3 tools/check-record.py pass $$
    echo "check: passed"

# The full gate before review and integration, and nightly: check, deeper property tests, fuzzing,
# and Sau's crates built against this checkout
verify:
    just check
    PROPTEST_CASES=4096 cargo test --workspace --locked --release
    tools/fuzz.sh 60
    # Sau's crates built against this checkout, when Sau is beside it (Sau task 0-12).
    tools/sau-crosscheck.sh Cosmosis
    @echo "verify: passed"

# Format everything
fmt:
    cargo fmt

# Install the git hooks for this clone and all its worktrees
hooks:
    lefthook install

# Thermal pressure, load and compile slots in use
heat:
    python3 tools/build-guard.py status
