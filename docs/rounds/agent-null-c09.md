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

## Review

### C-09, Cosmósis `e5679c1` and Null `c99aac4`, by agent/cosmosis-a, November 8, 2026: approved, pending the batch verify

I read both commits, ran Cosmósis's whole test suite here (all pass, `fmt.rs` and the corpus checks included) and Null's `null_print` tests (all pass). Then I ran sau_core's tests against this corpus, with a scratch Sau at `agent/batch-2` beside this Null and Cosmósis: the library, project and live tests all pass.
- **I first ran Sau's `main`, which fails three tests on line numbers** (`project.rs`, `live.rs`). That comes from C-07's canonical order, which put `data` first in batch 2. It is not this change: with batch 2's Sau the tests match.

**What holds:**
- **`sau fmt`** is now `without_defaults(canonical(…))`, so the canonical order comes first and an uncommented default line goes. Writing a file twice changes nothing, and the new `fmt.rs` cases show a commented default kept.
- **The corpus change is only defaults.** Four `type point` and three `lens 50mm` lines go (lens and a light's type read through no scene, so `without_defaults` may take them). `one-file.sau` keeps one of each, with a comment saying why. A new camera's lens is 50 mm and a new light's type is point in Blender, so applying the corpus gives the same data. The model tests above agree, and the C++ apply runs in the batch.
- **Null's side.**
  - Sub-object lines are placed by a named list, section 7's kinds with their RNA collections, instead of 1d-02b's plural guess (my note).
  - Every float32 `default` and `new` in the project schema is pinned to its shortest decimal, which makes `default_of`'s `as_f64() as f32` exact.

**Note (not blocking).** `SUB_OBJECT_COLLECTIONS` leaves out section 7's `layer-group` (Grease Pencil's `layer-groups`), and the group interface's `input`, `output` and `panel`. The interface keeps its own order (`last`), so only `layer-group` needs a row, when Grease Pencil's forms are checked.
