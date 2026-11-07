#!/bin/bash
# Run every fuzz target in fuzz/ for the given number of seconds each (default 60).
# Usage: tools/fuzz.sh [seconds]   (`just verify` runs it). Needs cargo-fuzz and a nightly
# toolchain; until the first fuzz target exists (task 1a-03 in Sau's board) it does nothing.
set -euo pipefail
seconds=${1:-60}
if [ ! -d fuzz/fuzz_targets ]; then
  echo "fuzz: no fuzz targets yet"
  exit 0
fi
command -v cargo-fuzz >/dev/null || {
  echo "fuzz: cargo-fuzz is missing: cargo install cargo-fuzz, and rustup toolchain install nightly" >&2
  exit 1
}
for target in fuzz/fuzz_targets/*.rs; do
  name=$(basename "$target" .rs)
  echo "fuzz: $name for ${seconds}s"
  cargo +nightly fuzz run "$name" -- -max_total_time="$seconds"
done
