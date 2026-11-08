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

## 1a-11: the manifest through Null's parser

**What works.** `crates/cosmosis/src/source.rs` reads a Null file with Null's parser (`null_syntax`, a new path dependency, MIT; with it Scowr's `scowr_syntax` and `scowr_errors`, MIT) into Cosmósis's parser-neutral tree: live nodes only (`/-` left out), a value the parser found missing adds nothing, and the keyword attributes `at`, `to` and `len` become the two arguments they are in Null. Null's diagnostics (`null-syntax`, `null-name`) come through with file, line, column and fix; Cosmósis's `Severity` gains `Warning` to carry Null's. `Manifest::parse(text)` reports Null's diagnostics alone when there are any, and otherwise reads the fields (C-02's rules, settled as S59). `sau check` now reads the manifest first and passes its kind to the layout check (so a `project` package without `project.noll` is an error), parses every other `.noll` file with Null's parser, and ends with a note that checking against Blender's schema is Null's checker (1a-06). Its summary names the package: `package "one-cube" (project, Null format 1): 6 Null files parse, 0 content files`.

**Measured.** `cargo test --workspace`: 55 tests. New: 3 unit tests of the conversion (a manifest's tree with spans into the text; `/-` and keyword attributes; Null's diagnostics with code, position and fix); `the_specs_manifest_example_parses_and_checks` reads the spec's section 4 example from the spec itself through the parser; `manifest_text_errors_are_positioned`; and five CLI tests: a manifest with a name and `null-format` exits 0 and names the package; a wrong field (`licence`) is `cosmosis-manifest-unknown-field` at `cosmosis.noll:3:3` with "did you mean `license`?"; a broken manifest fails with `null-syntax`, `null-name`, `cosmosis-manifest-newer-format` or `cosmosis-manifest-no-package` at its position; a project without `project.noll` fails; a syntax error or a non-kebab name in another file fails with Null's code at its line. The six corpus drafts pass `sau check` with their manifests read and every file parsed. `just check` passes, the licence gate included over the new path dependencies.

**Unverified.** The files are parsed, not checked against Blender's schema (1a-06). Cosmósis builds against Null's and Scowr's main checkouts by path, as they stand: their uncommitted work is in the build too (Null's worker had 1a-06 changes open when this was checked).

## 1a-15: the slice corpus through `sau check`

**What works.** `sau check` now checks what every Null file means, with Null's checker (`null_check`, a new path dependency on Null; with it `serde` and `serde_json`, MIT OR Apache-2.0) against Null's `schema/null-schema.json`, compiled in (`crates/cosmosis/src/schema_check.rs`). All of a project's files are checked together, so a handle defined in one file resolves in another; `cosmosis.noll` is left to the manifest reader. A file the parser already found wrong reports only the parser's errors, as the manifest does, so one mistake gives one error. The six corpus projects pass it whole. The nine errors the checker found in them are fixed: four cubes written `size=[2m 2m 2m]` or `size=[1m 1m 1m]` now have one `size`; two meshes' `faces` are `polygons` (S16, RNA's name); three materials reach `output-material`, which the schema has since 1a-05b. The table keeps its proportions in its object's `scale` as it checks today; it can move to the add operator's `scale` (`primitive cube size=1m scale=[…]`, 1a-05c) once the checker takes it (1a-06b).

**Measured.** `sau check` on each of the six projects exits 0 (7, 6, 6, 2, 8 and 8 Null files checked). `cargo test --workspace`: the corpus test runs `sau check` on every project and fails on any error; the coverage test now looks for `polygons`. `just check` and `just verify` pass.

**Sent back by agent/null-a, and fixed (`da80091`).** The coverage test matched words, so a scene's `camera @cam` line counted as a camera and an object's `material @clay` slot as a material; a missing datablock would have gone unnoticed. Each construct is now a path of nodes from the top level of a file (`material` > `nodes` > `node principled-bsdf` > `roughness`, say), walked over the nodes Null's parser reads through `cosmosis::source`, so commented-out nodes do not count either. New self-tests over a two-file project with every construct: dropping the camera datablock while the scene's camera line and the object's `data camera` stay leaves exactly "camera" missing; dropping the material while the object's slot stays leaves the material and its three node constructs missing; commenting out `parent` (both `/-` and `//`) leaves "object parent" missing. The six projects still write every construct. `just check` and `just verify` pass.

## C-04: an entry gone since the listing is not a broken link (`4c9d68b`)

**What was wrong.** Found by agent/null-a in 1a-13a: live reload saw false errors 4 times in 21 runs. Editors save by writing a temporary file and renaming it over the original. `layout::walk` lists a folder and then reads each entry. An entry gone in between failed `fs::metadata`, and was reported as "cannot read the file: it is a broken link". A `.noll` file gone between `metadata` and `read` gave "No such file or directory".

**The fix** (`crates/cosmosis/src/layout.rs`):

- An entry whose `metadata` fails is a broken link only if `symlink_metadata` still finds it; otherwise it is skipped.
- A `.noll` file, a content file or a sub-folder gone since the listing (`NotFound`) is skipped too. A content file is counted only once it has been read.
- Every other read error is reported as before.

**Measured.**

- `a_file_renamed_over_another_while_walking_is_not_a_broken_link`: one thread saves `objects/cube.noll` by renaming over it, as fast as it can, from a hidden `.tmp` and from a `.noll` temporary file. Meanwhile 200 checks walk the folder.
  - Before the fix it failed with 13 false reports, both kinds.
  - After it, there are none, in 5 runs out of 5.
  - Every check still read `objects/cube.noll`.
- `a_real_broken_link_is_still_reported`: a link to a missing file gives exactly one "broken link" error.
- `just check` and `just verify` pass.

**Not this task's.** A temporary file named with `.noll` that is still there when it is read is read as a source. So, for a moment, it can define the same object twice. Editors' temporary names (`.x.swp`, `x~`, `.x.tmp`) do not end in `.noll`, so this only matters for an unusual editor.

## C-05: a project ignores editors' and the system's files (`63d05e3`)

**What was wrong.** Found by agent/null-a reviewing C-04. Under 300 checks and 139,056 saves by rename, 39 checks saw a temporary file ending in `.noll` (`cube.noll.saving.noll`), read before its rename, as a second definition of every datablock in it.

**The fix.**

- `layout::is_ignored(name)` skips, in the walk and in content folders, names:
  - starting with `.` or `~`;
  - ending in `~` or `#`;
  - ending in `.tmp`, `.temp`, `.bak`, `.swp`, `.swo`, `.swx`, `.crswap` or `.vsctmp`, in any case;
  - holding `.sb-` (macOS's safe save), `___jb_` (JetBrains) or `.noll.` (a copy of a Null file under another name);
  - and macOS's `(A Document Being Saved By …)` folders.

  It applies to files and folders alike, before anything is read.
- The spec lists them, in section 2, "Ignored names".
- VS Code's temporary suffix `.vsctmp` is my reading of its atomic save, not checked against its source. A web search did not confirm it ([VS Code issue 15111](https://github.com/Microsoft/vscode/issues/15111) discusses atomic saves without naming it). Since the name does not end in `.noll`, it would not be read in any case.

**Measured.**

- **`checks_during_saves_by_rename_report_nothing`** (cosmosis tests): 300 checks of a copy of `one-cube.sau`, layout and Null's checker both, while a thread saves `objects/cube.noll` by renaming over it from the review's three names: `cube.noll.saving.noll`, `.cube.noll.tmp` and `cube.noll~`.
  - With the old rule, in which only a leading `.` is ignored, it fails in 3 runs of 3, with 52, 50 and 74 "defined twice" reports.
  - With the new rule it passes, 3 runs of 3.
- **`sau check` itself.** Run 300 times from the release build, while a script made 150,480 such saves in 20 s: 0 of 300 failed.
- **Sau's live reload** (`sau_core::live`, approved 1a-13a, built against this Cosmósis). Under 116,538 saves in 20 s, each with a different location so that every reload publishes, it published 15,710 snapshots, none with a problem.
- **The names.**
  - `editors_and_system_files_are_ignored_even_ending_in_noll`: 16 such names, each defining `@cube` again, give no diagnostic, and only the project's own files are read.
  - `ordinary_names_are_not_ignored`: `my.cube.noll`, `tmp.noll`, `café.noll` and others are still read.
- `just check` and `just verify` pass.

## 1d-04a: the manifest complete (`bea7dab`)

**What was there.** C-02 and 1a-11 already read every field of the spec's section 4 (S37, S45, S59): the name, `kind`, `version`, `sau`, `null-format`, `license` and `dependencies`. They had word-for-word tests of the messages, mostly on hand-made trees. No change to the reader was needed.

**What this adds** (`crates/cosmosis/tests/manifest_fields.rs`):

- **`every_field_reads_and_every_wrong_value_is_its_error`**: 59 cases, each a whole manifest read from text through Null's parser, as `sau check` reads it. Each field's right forms come first, then each wrong form with the code it gives:
  - the name: missing, a word, not kebab-case, a `name` field;
  - the one package node: none, two, a stray node;
  - `kind`: each of the seven kinds, the default, an unknown kind, a string, a list;
  - `version`: three parts; two, four, a leading zero, a letter, a string;
  - `sau`: two or three parts; one, four, a string;
  - `null-format`: 1; missing, 0, 1.5, −1, 2 (newer), a string;
  - `license`: a string; a word, a list;
  - `dependencies`: a requirement, a path, both, none; neither, a bad requirement, a bare word, an unknown property, a repeat, a capitalised name, a block;
  - any field: repeated, misspelt, without a value, with two values, with a block, and written as properties on the package's line.
- **`every_field_has_a_case_and_every_error_is_met`**: every field in `manifest::FIELDS`, plus the name and `null-format`, has a case. Every `cosmosis-manifest-…` code in the reader's source is given by at least one case.
- **`the_specs_fields_and_the_codes_are_the_same_list`**: the fields of section 4's example are exactly `manifest::FIELDS`, and the spec's reading rules name each one.

**Found on the way.** Two errors are met differently from text than from hand-made trees:

- A capitalised dependency name (`Trees`) is Null's own `null-name`: the parser finds it first, as it does for every node.
- `dependencies trees`, with a word where the block goes, is `cosmosis-manifest-type`.

The reader's `cosmosis-manifest-bad-name` is still met, by the package's name.

**Measured.** All three tests pass. `just verify` passes. The first run of `just check` failed without saying why, with no change in between; the rerun on the same committed tree passed, and I did not find the cause.

**Sent back by agent/null-a, and fixed (`883d07b`).** Section 4 has two new lines:

- `cosmosis.noll` is a Null file first: its syntax errors and non-kebab-case node names are Null's own (`null-syntax`, `null-name`), reported before the manifest is read. A dependency's name is a node's name, so `Trees "^1"` is `null-name`.
- Versions, `sau` and `null-format` have no leading zeros, as semantic versioning writes them.

The review's nits, taken:

- An unquoted `path=../Trees` is suggested as written, `trees path="../Trees"`, no longer as `../trees`.
- A `null-format` too large for any integer is "newer than this Cosmósis reads", not "not a format number".
- `path=""` is its own error, `cosmosis-manifest-empty-path`, "the dependency `trees` has an empty `path=`".

Each has a test; the table of cases gains the empty path and the 20-digit format.

**C-05's nit.** macOS's save folder, `(A Document Being Saved By TextEdit)`, is localised, so a project now ignores any name wholly in parentheses. The spec says so; the test adds a Spanish one.

`just check` and `just verify` pass.

## 1d-04b: the folder layout complete (S36, S66)

**What changed.**

- **A handle defined twice names both files.** Null's checker already marks where a handle was first defined, with a secondary label, but `schema_check` dropped it. Cosmósis's `Diagnostic` now has `related` places, printed as `  ::: file:line:column: first defined here` under the `-->` line. Any secondary place Null gives comes through the same way.
- **`.sau/ids.noll` is read** (S61 (2), S66). It is the one source file in the cache folder `.sau/`, which stays skipped otherwise. A folder of that name is a `cosmosis-layout-not-a-file` error.
- **Files whose forms are their own.** `settings.noll`, `view.noll` and `.sau/ids.noll` go through Null's parser for their syntax, but not through the schema check (`layout::holds_datablocks`), until their forms are specified (S66). Before this change, any content in `settings.noll` or `view.noll` was a `null-kind` error.
- **The templates ship** in `templates/project/` as `gitignore` and `gitattributes`, named so they don't act on this repository. `cosmosis::templates` holds them:
  - the `.gitignore` covers `.sau/*`, then `!.sau/ids.noll`, `/view.noll` and `/renders/`;
  - the `.gitattributes` puts `/media/**` and `/data/**` in LFS.

  Writing them into a new project is the new row 1d-04e.
- **Spec section 2** has:
  - the `.sau/ids.noll` line in the tree;
  - "names both files, each with its line";
  - "References need no imports";
  - the files that hold no datablocks;
  - the version-control rule with its one exception;
  - the templates.

  The CHANGELOG has the line.

**Measured.** `crates/sau_cli/tests/layout_rules.rs` has one `sau check` test per rule and per error. Each error is asserted with its code, file, line and column:
- a free layout (any folder, any name) that resolves across files;
- a handle defined twice, at `objects/b.noll:2:8` with `objects/a.noll:1:8` named;
- a reference found in another file, and one that resolves nowhere at `objects/cam.noll:2:15`;
- the cache skipped, even a broken `.noll` in it, but not `ids.noll`;
- `ids.noll`'s syntax error at its line, a non-UTF-8 byte at its line, and the folder case;
- `settings.noll` and `view.noll` with arbitrary content clean, and their syntax errors at their lines;
- the two templates, as Git reads them: `git check-ignore` in a fresh repository ignores exactly the four cache and output paths and keeps `.sau/ids.noll`, `settings.noll` and the sources; `git check-attr` gives `filter: lfs` for `media/` and `data/` only.

Each of the layout's earlier rules and errors already had a test, now listed with these:
- root names that must be files;
- `project.noll`;
- UTF-8;
- the three content-folder errors;
- nested projects, `renders/`, broken links;
- C-05's ignored names.


**After review** (agent/null-a, Null `182677a`):
- **A broken link at `.sau/ids.noll`** is now a `cosmosis-io` error, "it is a broken link", as a broken link anywhere else in the project is. Before, `exists()` was false for it, so `sau check` passed silently.
- **A `.sau` that is a link to a folder** is not followed for the map, as `walk` follows no link to a folder; a `cosmosis-layout-link` note says so. Section 2 says both.
- **Tests:** the broken link (exit 1 at `.sau/ids.noll`), and a linked `.sau` holding a broken map (not read, exit 0, the note).

## C-06: `sau check` on the schema of all project data

**What changed.** `schema_check` compiles in Null's `schema/null-schema-project.json` (Sau 1b-03b, read by 1b-03e's checker) instead of the slice's `null-schema.json`: one line, and the module's doc comment.

**Measured** (`crates/sau_cli/tests/project_schema.rs`):

- **The corpus still checks clean.** All six slice projects pass `sau check` on the project schema.
- **A type the slice's schema never knew is checked.** A world misspelt as `colr` on a copy of one-cube is now `null-unknown-property … did you mean `color`?` at `world.noll:2:3`. With the slice's schema, the kind itself was refused (`null-kind`). A clean world passes.

**The row's "Done when"** asks for a misspelt modifier property. The project schema holds every modifier's properties (`bevel-modifier` and the others), but the line that writes a modifier, `modifier @bevel bevel { … }`, is a sub-object form the checker reads only from Sau 1d-07. Until then it is "an object has no `modifier`" (the third test shows it is an error, not a silent pass). So I showed the same thing, a misspelling the slice's schema could not know, on a world. The modifier case belongs with 1d-07: lead's call whether to amend this row's wording.

## Next

C-06 in review.

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

### 1a-11, reviewed by agent/sau-b (November 7, 2026): approved

Read the row, this report's 1a-11 section and the diff of `2aac2a6`. `just verify` passed on clean detached copies: Cosmósis at `2aac2a6`, beside Null at its `main` (`21da7a5`, without the Null worker's uncommitted 1a-06 work) and Scowr's main checkout, under Sau's `work/lead/` and removed afterwards; Clippy, the licence gate over the new path dependencies, 52 tests in the check run and the 4,096-case release run, no fuzz targets; 58 seconds.

The "Done when" holds, checked by hand with the built `sau`: a manifest with a name and `null-format` passes and names the package; `licence "MIT"` is `cosmosis-manifest-unknown-field` at `cosmosis.noll:3:3`, "did you mean `license`?"; fields as properties on the package line (`package "p" null-format=1 kind=project`) read; an empty manifest is `cosmosis-manifest-no-package`; the kind reaches the layout check (`kind material` without `project.noll` passes, the default `project` without it fails); an unclosed block in another file is `null-syntax` at `objects/b.noll:1:13`.

Nits, none blocking:

1. **A latent panic.** `Manifest::parse` returns `Err` whenever Null reports any diagnostic, warnings and notes included, but `sau check` counts only errors, so a manifest whose only diagnostic is a warning would reach `manifest.expect("no errors means the manifest was read")` and panic. Null's parser gives no warnings today (none of `null_syntax`, `scowr_syntax` or `scowr_errors` makes one), so it cannot happen yet; returning `Err` only on errors, and passing warnings on, closes it before Null gains one.
2. "1 Null file parse": the summary's verb does not follow the count.
3. A second `package` node in a file other than the manifest, and a handle that is not kebab-case (`object @Bad`), pass: right for the parser stage, but worth a line for 1a-06's checker.
