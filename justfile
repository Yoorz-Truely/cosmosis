# Everyday commands. Run `just` to list them. The workflow, board and limits are Sau's:
# ../Sau/AGENTS.md and ../Sau/docs/agents/.

# List the recipes
default:
    @just --list

# The commit gate for code: format, lint and test, through the build guard
check:
    python3 tools/build-guard.py wait
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    @echo "check: passed"

# Format everything
fmt:
    cargo fmt --all

# Install the git hooks for this clone and all its worktrees
hooks:
    lefthook install

# Thermal pressure, load and compile slots in use
heat:
    python3 tools/build-guard.py status
