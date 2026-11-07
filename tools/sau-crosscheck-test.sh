#!/bin/bash
# Tests tools/sau-crosscheck.sh (Sau task 0-12) in throwaway clones of this checkout: a public
# signature Sau's crates call, changed, fails the check with the compiler's error; with no Sau
# beside the clone, the check is skipped and passes. Slow (it builds Sau's crates twice), so it
# is run by hand: tools/sau-crosscheck-test.sh <Null|Cosmosis>. The same file is in Null and
# Cosmósis.
set -euo pipefail
name=${1:?usage: tools/sau-crosscheck-test.sh <Null|Cosmosis>}
root=$(git rev-parse --show-toplevel)
case $name in
# A function Sau's sau_core calls given one more parameter, which this repository's own
# libraries still build with: only Sau's call breaks.
Null) file=compiler/null_print/src/project.rs before='pub fn write_transform(' after='pub fn write_transform(_extra: (), ' ;;
Cosmosis) file=crates/cosmosis/src/layout.rs before='pub fn find_root(' after='pub fn find_root(_extra: (), ' ;;
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
grep -q "$before" "$scratch/with/$name/$file"
sed -i '' "s/$before/$after/" "$scratch/with/$name/$file"
if out=$(cd "$scratch/with/$name" && tools/sau-crosscheck.sh "$name" 2>&1); then
  echo "sau-crosscheck-test: a changed signature passed:" >&2
  echo "$out" >&2
  exit 1
fi
# The compiler's error, in Sau's crate: this repository itself still builds.
grep -q "^error\[E" <<<"$out" && grep -q -e "--> sau_core/src/" <<<"$out" || {
  echo "sau-crosscheck-test: failed without the compiler's error in Sau's sau_core:" >&2
  echo "$out" >&2
  exit 1
}
echo "sau-crosscheck-test: a changed signature fails: $(grep -m1 '^error\[E' <<<"$out")"

# With no Sau beside it: skipped, and passing.
mkdir "$scratch/without"
git clone -q --no-hardlinks "$root" "$scratch/without/$name"
out=$(cd "$scratch/without/$name" && tools/sau-crosscheck.sh "$name")
grep -q "skipped" <<<"$out"
echo "sau-crosscheck-test: with no Sau beside it, skipped: $out"
echo "sau-crosscheck-test: passed"
