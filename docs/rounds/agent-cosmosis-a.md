# agent/cosmosis-a

The Cosmósis worker's report, kept current with each commit. Working in Cosmósis's main checkout on `main` (Sau's decision S58). Tasks from Sau's board: C-01, C-02 and C-03 (added November 7, 2026, Sau `87f7ea4`), then 1a-11 once Null's parser (1a-03) is done.

## C-01: the `sau` command's skeleton and `sau check`'s layout stage

**What works.** `crates/sau_cli` is the `sau` command: `--help` lists every command of the spec (section 7), marking all but `check` "not built yet"; `--version`; a misspelt command gets a suggestion. Exit status 0 on success, 1 when errors were found, 2 on wrong usage or an unbuilt command. `sau check [path]` finds the project root (the nearest folder holding `cosmosis.noll`, from the path upwards, S37) and checks the layout (`crates/cosmosis/src/layout.rs`):

- `cosmosis.noll`, `cosmosis.lock`, `project.noll`, `view.noll` and `settings.noll` at the root are files (`cosmosis-layout-not-a-file`);
- a `project` package has `project.noll` (`cosmosis-layout-no-project-file`; needs the manifest's kind, so `sau check` asks for it only once 1a-11 reads the manifest);
- every `.noll` file is readable UTF-8 (`cosmosis-not-utf8`, at the first bad byte's line and column; `cosmosis-io`);
- `media/` and `data/` hold only files named `<sha256 of contents>.<ext>` (`cosmosis-layout-content-name`, `-content-hash`, `-content-folder`), the help naming the exact new name;
- dot-folders (`.sau/`, `.git/`), the root's `renders/` and nested projects are skipped (a note for a nested project).

Diagnostics print as `error[code]: message`, `--> file:line:column`, `help: …` (Null spec, section 18). Until the parser exists, `sau check` says in a note that the manifest and files are not read yet.

**Measured.** `cargo test --workspace`: `tests/layout.rs` 11 tests, one per layout error and for root finding; `sau_cli/tests/check.rs` 6 tests running the built `sau`: help, version, exit 2 for each usage error, exit 0 on a valid project from its root and from a folder inside, exit 1 with the right code and file for each layout error. `just check` exits 0.

## C-02: the manifest's data model, fields and errors

**What works.** `crates/cosmosis/src/manifest.rs`: `Manifest` (name, kind, version, sau, `null-format`, license, dependencies), `Kind`, `Version`, `Dependency`, and `ManifestError` with 23 kinds of error, each with a stable `cosmosis-manifest-…` code, a message and a help (nearest field, kind or property; the kebab-case spelling of a bad name; the line of a repeated field's first use). It reads from `crates/cosmosis/src/tree.rs`, a parser-neutral node tree (nodes, arguments and properties in order, children, byte spans; numbers and handles kept as words as written). Every error is reported, not only the first. 1a-11 only has to turn Null's parse tree into that tree and call `Manifest::read`, then pass the kind to `layout::check`.

Choices made where the spec was silent, recorded in the spec's section 4: only the name and `null-format` are required (as 1a-11's "Done when" implies), so `kind` defaults to `project`; a dependency may give a requirement, a path, or both; `version` needs three parts, `sau` two or three.

**Measured.** `tests/manifest.rs`: 19 tests, every message and help checked word for word: a manifest with a name and `null-format` reads to the expected value; the spec's example reads every field; fields as properties; each kind; and each error (unknown, missing, repeated, wrong type, extra value, block, bad name, bad and newer `null-format`, unknown kind, bad version, each dependency error, several errors at once). Unit tests of positions, rendering and suggestions in `diagnostic.rs` (3).

## C-03: drafts of the slice corpus

**What works.** Six projects in `corpus/slice/`: `one-cube` (the smallest scene), `inline-pyramid` (inline vertices and faces, and a primitive), `table-and-cups` (parents; m, cm, mm; degrees, radians and bare radians; scale), `two-materials` (a shared and a single-use material, Principled BSDF base colour and roughness), `lights-and-cameras` (all four light types, two cameras), `one-file` (everything in `project.noll`, S36). Written to avoid the syntax edges found in 1a-01: one attribute per line, no `at`, `to` or `len` words.

**Measured.** `sau_cli/tests/corpus.rs`: at least five projects, each passing `sau check`; every slice construct (19 entries: scene; object, location, rotation, scale, parent; data mesh, camera and light; mesh, primitive, inline vertices and faces; material, Principled BSDF, base colour, roughness; camera; light) appears in at least one, found in the text with comments removed; a test that the coverage check notices a missing construct.

**Unverified.** None of the corpus is parsed or checked against the schema yet: that is 1a-15, with 1a-06 and 1a-11. Property names not settled by Null's spec are drafts: `vertices` and `faces` (RNA's are `vertices` and `polygons`), the primitive parameters (`size`, `radius`, `depth`, `vertices`, `segments`, `ring-count`, from Blender's add-primitive operators), `type` for a light's kind, and `object @x` lines in a scene for objects in its main collection.

## Next

C-01 to C-03 go to review together; then 1a-11 when `just tasks` lists it as ready.

## Also done

- `just check` and `just fmt` run `cargo fmt`, not `cargo fmt --all`, which would also format a path dependency's files once Cosmósis depends on Null (`021b522`, found by the Null worker in 1a-01).
- `_typos.toml` lists the four deliberate misspellings the suggestion tests use (`chek`, `materal`, `pakage`, `vresion`), so the spelling hook passes without being skipped.

## Review

Reviews of the Null worker's tasks are written in Null's report, `~/Projects/Null/docs/rounds/agent-null-a.md`, under "Review".

- 1a-01, November 7, 2026: approved (Null `67e31b1`). Null's `just verify` passed on `34cf222`; Scowr's own tests 192 of 192 at its tip `770a472`, Scowr untouched.
- 1a-04, November 7, 2026: approved (Null `2c37842`). An independent oracle (Chudnovsky's π, exact rounding) agrees on 9,144 cases, near-tie degrees included; properties pass at 100,000 cases. For the spec: "converted exactly", not "in float64" (no difference for `f32`, 21.5 % of `f64` degree values). For 1a-07: printing in `deg`, `in` and `ft` can drop the unit for `f64` values of 16 or 17 digits.
- 1a-02, November 7, 2026: approved (Null `427986c`). Lossless on the slice files, this repository's 37 corpus files (no diagnostics either) and 20,000 random mutations; the tests catch a one-byte change. For 1a-06: handles in arguments and enum values are not yet checked for kebab-case. For 1a-03: Scowr's example text (`vel=96`) reaches Null's messages.
- 1a-08, 1a-05 and 0-05 (Sau's worker), November 7, 2026: written in `Sau/docs/rounds/agent-sau-a.md` on `agent/sau-a` (Sau `20b296c`). 1a-08 and 0-05 approved; 1a-05 sent back (`MeshPolygon.vertices` listed as a fixed array of 3; a scene's objects missing; four Blender deprecation sentences in Null's MIT file).

- 1a-03, November 7, 2026: sent back (Null `ff7bd48`). A fuzz run with a larger input limit crashed on a false alarm in the shared invariants (the ban on Scowr's `vel=` example fires on a user's `level=`); the depth messages are pinned by no test; fuzzing never tried inputs over 583 bytes. The S59 spec edit matches S59.

- 1a-05, second review, November 7, 2026: approved (Sau `1110fff` on `agent/sau-a`). Dynamic arrays checked both ways in the built Sau, collections added, no Blender prose; the fresh generation equals Null's file.

### 0-03, the agent environment (the lead's), November 7, 2026: sent back

Checked against its "Done when" as its note amends it (the cache measurement moved to 0-05): the board, guides, recipes, hooks, guards, `check` and `verify` work in all three repositories.

**Works, measured.**

- The board: I added rows, claimed, put in review and approved through `just task` and `dev.py task` this session; each commit landed on `main` alone. A note with spaces now reaches the tool as one argument (`just task note NO-SUCH-TASK --note "two words here"` gets as far as "no task").
- Hooks are installed in all three (Sau: pre-commit, commit-msg and four more; Null and Cosmósis: pre-commit, commit-msg), and they bite: the spelling hook stopped a Cosmósis commit of mine until the deliberate misspellings were listed in `_typos.toml`.
- The repository guard, tested on staged files in a throwaway `git clone --shared` of `main` (sparse, in my scratchpad): it refuses a file under `work/`, a `.DS_Store`, a 5 MB file not in LFS (`.txt`, `.py`; LFS types pass), conflict markers, invalid JSON, a new Markdown file outside its home, a Blender file changed without `Sau:`, and `docs/TASKS.md` staged in a worktree; it allows a report in `docs/rounds/`, a Blender change marked `Sau:`, and the board on `main`.
- The build guard: rustc runs through it in Sau, Null and Cosmósis; it held compiles to 2 slots at heavy thermal pressure while I worked, and `just check` waited for heavy pressure to clear.
- `check` and `verify`: Cosmósis's both pass at `8361404` and `ba96e96`; Null's `just verify` passed on exact copies of `34cf222`, `65c6ea7` and `0edf8f4`; Sau's `just verify` passed in `work/wt/sau-a` at `3d709a3` (105 s), running Null's verify for the sibling worktree and skipping the Cosmósis link, as the S58 rules say. `just doctor` reports every tool and hook as present.

**To fix.**

1. **`just task note` replaces a task's notes instead of adding to them** (`if note: r["notes"] = note` in `dev.py`). A note after an approval erases "approved by …", which `just integrate` looks for, so the task silently stops being approved; a send-back note erases the author's commit and evidence. I kept them by hand in my notes on 1a-05 and on this task. Append, as `approve` does.

**Also found.**

- Cosmósis's `AGENTS.md` still told agents to work in a worktree, never on `main`, against S58; fixed in Cosmósis `fcee297`.
- The nightly run has never run (`just doctor`: "last nightly: never run") and is not scheduled; `just nightly-install` is a standing job, so it waits for the user's word. Unverified.
- Blender's tests in `verify` wait for the developer build (V-01), as the task's note says.

### C-01 to C-03, reviewed by agent/sau-a (November 7, 2026)

Read the three rows, this report and the diffs of `8d731e3`, `8361404` and `021b522`. `just verify` passed on an exact copy of `main` at `8361404` (a detached worktree under Sau's `work/lead/`, removed afterwards): 42 tests (diagnostic 3, layout 11, manifest 19, check 6, corpus 3), Clippy and `cargo deny` clean, no fuzz targets yet; 2 minutes 44 seconds, most of it waiting out "heavy" thermal pressure.

- **C-01: approved.** Checked by hand with the built `sau`: `--help` lists the spec's 15 `sau` commands (section 7, `preim` rightly left out); bare `sau` and an unbuilt command exit 2; a project with a folder named `settings.noll`, an upper-case hash name in `media/` and a broken `.noll` link exits 1 with all three coded diagnostics naming their files; a corpus project exits 0. The one-test-per-error rule holds: every layout error has a CLI test except `cosmosis-layout-no-project-file`, which needs the manifest's kind and is tested in `tests/layout.rs`.
- **C-02: approved.** Every wrong field of the "Done when" (unknown, missing, repeated, wrong type, bad name, unsupported format) has a word-for-word test. Probed outside the tests: a dependency with both a requirement and `path=` reads; `sau 0.1.0` reads; `version 0.1` is `cosmosis-manifest-version`; `null-format 0` is `cosmosis-manifest-null-format`.
- **C-03: approved.** Six projects, each passing `sau check`; the coverage test finds all 19 constructs.
- **`021b522`: fine.** Plain `cargo fmt --check` still fails on a misformatted file in either member (checked in both).

Nits, none blocking:

1. `sau check /no/such/path` exits 1 (`cosmosis-no-such-path`); a path that does not exist is closer to wrong usage (2).
2. `null-format 99999999999` (over `u32`) is reported as "not a format number", not as a newer format.
3. An unknown field written twice gives two errors, `unknown-field` and then `repeated-field`; the second adds nothing.
4. The coverage test matches `camera` and `light` at the start of any line, so the scene's `camera @shot-cam` alone would satisfy "camera": it would not notice the camera datablocks going missing.
5. Finding a nested project uses `exists()` (follows links) but `find_root` uses `symlink_metadata`, so a broken `cosmosis.noll` link counts as a root but not as a nested project.
6. Release builds print "stripping debug info with `rust-objcopy` failed: signal: 6" for every binary: the toolchain, not this code, but worth a look.

Spec choices made by the author where the spec was silent, recorded in section 4 without a decision number. Flagged for the lead and the user, not decided here: `kind` defaults to `project`; a dependency may give a version, a `path=`, or both; `version` has three parts, `sau` two or three.
