#!/bin/bash
# Builds Sau's Rust crates against this checkout (Sau task 0-12), so a change to an API Sau uses
# fails here, in the repository that made it, before it merges, not later in Sau's build.
# `just verify` runs it; it is skipped when there is no ../Sau beside this checkout.
#
# Usage: tools/sau-crosscheck.sh <Null|Cosmosis>, the name of this repository.
#
# Sau's crates take Null and Cosmósis by path (`../../../../Null` from `sau/crates/sau_core`), and
# Cargo reads such paths as written, without resolving links. So the check builds them in a folder
# holding Sau's committed `sau/` (its HEAD, not its working tree, which may be mid-merge or hold
# someone's edits), with this checkout, under its own name, and the other repositories beside it
# as links. Sau's own Cargo.lock is used as it is (`--locked`): if this repository gained a
# dependency Sau's lock does not list, that fails too, and Sau's lock needs the line. The same
# file is in Null and Cosmósis (`tools/`).
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
head=$(git -C "$sau" rev-parse HEAD)
# Sau's committed files, extracted again only when its HEAD moves, so later runs build only what
# changed. A real folder, not a link: the compiler resolves `include_str!` paths through links.
if [ "$(cat "$tree/Sau/.sau-crosscheck-head" 2>/dev/null)" != "$head" ]; then
  rm -rf "$tree/Sau"
  mkdir -p "$tree/Sau"
  paths=(sau)
  if git -C "$sau" cat-file -e HEAD:source/blender/blenkernel/intern/unit.cc 2>/dev/null; then
    paths+=(source/blender/blenkernel/intern/unit.cc) # read by sau_core's build (task 1c-01g)
  fi
  git -C "$sau" archive HEAD "${paths[@]}" | tar -x -C "$tree/Sau"
  echo "$head" >"$tree/Sau/.sau-crosscheck-head"
fi
for other in Null Cosmosis Scowr; do
  rm -f "$tree/$other"
  if [ "$other" = "$name" ]; then
    ln -s "$root" "$tree/$other"
  elif [ -e "$root/../$other" ]; then
    ln -s "$(cd "$root/../$other" && pwd -P)" "$tree/$other"
  fi
done
echo "sau-crosscheck: Sau's crates at ${head:0:8} (committed) against this $name"
if ! RUSTC_WRAPPER="$root/tools/build-guard.py" CARGO_TARGET_DIR="$work/target" \
  cargo check --locked --workspace --all-targets --quiet \
  --manifest-path "$tree/Sau/sau/crates/Cargo.toml"; then
  echo "sau-crosscheck: Sau's crates do not build against this $name. Either a change here" \
    "breaks an API Sau uses (keep it, or say what Sau must change), or this $name gained a" \
    "dependency Sau's sau/crates/Cargo.lock does not list. Sau is the lead's to change." >&2
  exit 1
fi
echo "sau-crosscheck: passed"
