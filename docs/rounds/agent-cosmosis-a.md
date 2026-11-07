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
