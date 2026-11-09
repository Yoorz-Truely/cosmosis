#!/bin/bash
# Tests tools/sau-crosscheck.sh (Sau task 0-12) in throwaway clones of this checkout: a public
# signature Sau's crates call, changed, fails the check with the compiler's error; a dependency
# added to a crate Sau uses fails it with the change Sau's Cargo.lock needs, which, applied, lets
# Sau's crates build `--locked`, while Sau's own lock is untouched (task 0-20); with no Sau beside
# the clone, the check is skipped and passes. Slow (it builds Sau's crates twice), so it
# is run by hand: tools/sau-crosscheck-test.sh <Null|Cosmosis>. The same file is in Null and
# Cosmósis.
set -euo pipefail
# Run from a hook or a rebase, git's GIT_* variables would send the clones' commands to the real
# repository: clear them all first (Sau task 0-33).
while IFS= read -r v; do unset "$v"; done < <(compgen -e | grep '^GIT_' || true)
name=${1:?usage: tools/sau-crosscheck-test.sh <Null|Cosmosis>}
root=$(git rev-parse --show-toplevel)
case $name in
# A function Sau's sau_core calls given one more parameter, which this repository's own
# libraries still build with: only Sau's call breaks.
# And a crate Sau uses, given a dependency Sau's lock already holds (so it resolves offline), on
# which that crate did not depend: Sau's lock needs one line more.
Null) file=compiler/null_print/src/project.rs fn=write_transform crate=compiler/null_syntax ;;
Cosmosis) file=crates/cosmosis/src/layout.rs fn=find_root crate=crates/cosmosis ;;
*) echo "usage: tools/sau-crosscheck-test.sh <Null|Cosmosis>" >&2 && exit 2 ;;
esac
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# With Sau beside it: the changed signature fails, with rustc's error.
mkdir "$scratch/with"
git clone -q --no-hardlinks "$root" "$scratch/with/$name"
for other in Sau Null Cosmosis Scowr; do
  if [ "$other" != "$name" ] && [ -e "$root/../$other" ]; then
    ln -s "$(cd "$root/../$other" && pwd -P)" "$scratch/with/$other"
  fi
done
grep -q "pub fn $fn(" "$scratch/with/$name/$file"
# The unchanged clone passes first, so the failure below is the change's.
(cd "$scratch/with/$name" && tools/sau-crosscheck.sh "$name" >/dev/null)
sed -i '' "s/pub fn $fn(/pub fn $fn(_extra: (), /" "$scratch/with/$name/$file"
if out=$(cd "$scratch/with/$name" && tools/sau-crosscheck.sh "$name" 2>&1); then
  echo "sau-crosscheck-test: a changed signature passed:" >&2
  echo "$out" >&2
  exit 1
fi
# The compiler's error, in Sau's crate, on the call to the changed function: this repository
# itself still builds.
grep -q "^error\[E0061\]" <<<"$out" && grep -q -e "--> sau_core/src/" <<<"$out" && grep -q "$fn(" <<<"$out" || {
  echo "sau-crosscheck-test: failed without the compiler's error in Sau's sau_core:" >&2
  echo "$out" >&2
  exit 1
}
echo "sau-crosscheck-test: a changed signature fails: $(grep -m1 '^error\[E' <<<"$out")"

# A new dependency: the check fails with the lock diff and changes no lock of Sau's; the diff,
# applied to Sau's committed lock, lets Sau's crates build --locked against this change.
git -C "$scratch/with/$name" checkout -q -- "$file"
sau_lock=$(cd "$root/../Sau/sau/crates" && pwd -P)/Cargo.lock
before=$(shasum "$sau_lock")
manifest=$scratch/with/$name/$crate/Cargo.toml
grep -q '^serde_json' "$manifest" && {
  echo "sau-crosscheck-test: $crate already depends on serde_json; pick another" >&2
  exit 1
}
sed -i '' 's/^\[dependencies\]$/[dependencies]\nserde_json = "1"/' "$manifest"
grep -q '^serde_json = "1"$' "$manifest"
if out=$(cd "$scratch/with/$name" && tools/sau-crosscheck.sh "$name" 2>&1); then
  echo "sau-crosscheck-test: a new dependency passed:" >&2
  echo "$out" >&2
  exit 1
fi
diff=$scratch/with/$name/target/sau-crosscheck/Cargo.lock.diff
if ! grep -q "needs this change" <<<"$out" || ! grep -q '^+ "serde_json",$' "$diff"; then
  echo "sau-crosscheck-test: failed without the lock diff:" >&2
  echo "$out" >&2
  exit 1
fi
[ "$(shasum "$sau_lock")" = "$before" ] || {
  echo "sau-crosscheck-test: the check changed Sau's own Cargo.lock" >&2
  exit 1
}
tree=$scratch/with/$name/target/sau-crosscheck/tree
(cd "$tree/Sau" && git apply "$diff")
RUSTC_WRAPPER="$root/tools/build-guard.py" CARGO_TARGET_DIR="$scratch/with/$name/target/sau-crosscheck/target" \
  cargo check --locked --workspace --all-targets --quiet --manifest-path "$tree/Sau/sau/crates/Cargo.toml"
echo "sau-crosscheck-test: a new dependency fails with the lock diff ($(grep -c '^[+-] ' "$diff") line(s)), which, applied, builds --locked"

# With no Sau beside it: skipped, and passing.
mkdir "$scratch/without"
git clone -q --no-hardlinks "$root" "$scratch/without/$name"
out=$(cd "$scratch/without/$name" && tools/sau-crosscheck.sh "$name")
grep -q "skipped" <<<"$out"
echo "sau-crosscheck-test: with no Sau beside it, skipped: $out"
echo "sau-crosscheck-test: passed"
