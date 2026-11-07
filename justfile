# Everyday commands. Run `just` to list them. The workflow, board and limits are Sau's:
# ../Sau/AGENTS.md and ../Sau/docs/agents/.

# List the recipes
default:
    @just --list

# The commit gate for code: format, lint, test and the licence gate, through the build guard
check:
    python3 tools/build-guard.py wait
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo deny --offline check licenses bans sources
    @echo "check: passed"

# The full gate before review and integration, and nightly: check, deeper property tests, fuzzing
verify:
    just check
    PROPTEST_CASES=4096 cargo test --workspace --locked --release
    tools/fuzz.sh 60
    @echo "verify: passed"

# Format everything
fmt:
    cargo fmt --all

# Install the git hooks for this clone and all its worktrees
hooks:
    lefthook install

# Thermal pressure, load and compile slots in use
heat:
    python3 tools/build-guard.py status
