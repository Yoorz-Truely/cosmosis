#!/bin/bash
# Builds Sau's Rust crates against this checkout (Sau task 0-12), so a change to an API Sau uses
# fails here, in the repository that made it, before it merges, not later in Sau's build.
# `just verify` runs it; it is skipped when there is no ../Sau beside this checkout.
#
# Usage: tools/sau-crosscheck.sh <Null|Cosmosis>, the name of this repository.
#
# Sau's crates take Null and Cosmósis by path (`../../../../Null` from `sau/crates/sau_core`), and
# Cargo reads such paths as written, without resolving links. So the check builds them from a
# folder of links: Sau, this checkout under its own name, and the other repositories beside it.
# Sau's own Cargo.lock is used as it is (`--locked`): if this repository gained a dependency Sau's
# lock does not list, that fails too, and Sau's lock needs the line. The same file is in Null and
# Cosmósis (`tools/`); Sau's check keeps the copies equal.
set -euo pipefail
name=${1:?usage: tools/sau-crosscheck.sh <Null|Cosmosis>}
root=$(git rev-parse --show-toplevel)
sau=$root/../Sau
if [ ! -f "$sau/sau/crates/Cargo.toml" ]; then
  echo "sau-crosscheck: no ../Sau/sau/crates beside this checkout; skipped"
  exit 0
fi
work=$root/target/sau-crosscheck
tree=$work/tree
rm -rf "$tree"
mkdir -p "$tree"
ln -s "$(cd "$sau" && pwd -P)" "$tree/Sau"
ln -s "$root" "$tree/$name"
for other in Null Cosmosis Scowr; do
  if [ "$other" != "$name" ] && [ -e "$root/../$other" ]; then
    ln -s "$(cd "$root/../$other" && pwd -P)" "$tree/$other"
  fi
done
echo "sau-crosscheck: Sau's crates ($(git -C "$sau" rev-parse --short HEAD)) against this $name"
if ! RUSTC_WRAPPER="$root/tools/build-guard.py" CARGO_TARGET_DIR="$work/target" \
  cargo check --locked --workspace --all-targets --quiet \
  --manifest-path "$tree/Sau/sau/crates/Cargo.toml"; then
  echo "sau-crosscheck: Sau's crates do not build against this $name. Either a change here" \
    "breaks an API Sau uses (keep it, or say what Sau must change), or this $name gained a" \
    "dependency Sau's sau/crates/Cargo.lock does not list. Sau is the lead's to change." >&2
  exit 1
fi
echo "sau-crosscheck: passed"
