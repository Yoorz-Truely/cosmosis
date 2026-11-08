# agent/null-c09

The report of agent/null-a working as `agent/null-c09` in `work/wt/null-c09/`. Cosmósis is on branch `agent/null-c09`, stacked on `agent/cos-c07` (`sau fmt`), with Null's `agent/null-1d07a` merged in for its flipped modifier test, since this Null checks modifiers. Null is on its own branch `agent/null-c09`, stacked on `agent/null-1d02b`, for two follow-ups from 1d-02b's review. November 8, 2026. Lints and tests only.

## C-09: `sau fmt` leaves defaults out, and the corpus follows

**Cosmósis.**
- `sau fmt` writes each datablock file as `null_print::without_defaults(canonical(text))`. A line of its own holding its property's default goes after the canonical order, unless a comment is on it or just above it (Null spec, section 15).
- The help text and the spec's `sau fmt` row say so.
- **The corpus is formatted again.** Four `type point` lines (a point light is the default) and three `lens 50mm` lines (a camera's default) go. Nothing else changes.
- **One-file.sau keeps one of each, with a comment.** It is the project where "every slice construct appears once", so it keeps `lens 50mm // the default, written to show the lens in millimetres` and `type point // the default, written to show a light's type`.
- Every corpus project passes `sau fmt --check`.

**Merged: `agent/null-1d07a`'s Cosmósis commit** (`c9cae5c`). This Null's checker reads modifiers (1d-07a), so C-06's test that a bevel's misspelt property "waits for its form" now fails. 1d-07a's own Cosmósis commit flips it, so this branch merges it. CHANGELOG.md conflicted only because this branch has no `merge=union` attribute yet; both entries are kept.

**Null** (`agent/null-c09`), from the 1d-02b review:
- **Section 7's list in place of the plural guess.** A sub-object line stands where its RNA collection stands, by a named list: `modifier` at `modifiers`, `constraint` at `constraints`, `bone` at `bones`, `bone-collection` at `collections`, `shape-key` at `key-blocks`, `vertex-group` at `vertex-groups`, `uv-map` at `uv-layers`, `attribute` at `attributes`, `view-layer` at `view-layers`, `slot` at `slots`, `layer` at `layers`, `strip` at `strips`, `track` at `nla-tracks`, `marker` at `timeline-markers`. A struct without that collection leaves the line where it was written.
- **The float default pinned.** A test checks every float32 `default` and `new` in the project schema, over 1,000 values: each reads back exactly through `as_f64() as f32`, being either the float32's shortest decimal or its exact value. `default_of` relies on that.

**Tests:**
- **Cosmósis `fmt.rs` (new):** a default line goes, a commented one stays, `--check` finds the change first, and a second run changes nothing.
- The corpus fmt tests (shuffled files come back byte for byte, comments survive, `--check` passes) pass on the reformatted corpus, and so do Cosmósis's other tests.
- **Null:** null_print's tests pass with the named list.

**Checks:** Null's and Cosmósis's `cargo test` and clippy are clean, and both lint-only `just check`s pass.
