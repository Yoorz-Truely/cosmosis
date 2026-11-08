# agent/cos-1d04d

agent/cosmosis-a's branch for 1d-04d, the `null-format` number and its migrations (S45), in `work/wt/cos-1d04d/`, November 9, 2026. Tests and lints only, no build (S69). Both repositories were cut from agent/batch-3b's tips (Cosmósis `52683eb`, Null `bdeb309`), since the row needs 1d-01c. Main is merged once batch 3b lands.

## 1d-04d: the Null format and its migrations

**Null** (`null_check::migrate`):
- `FORMAT` is 1, the format this Null reads. `MIGRATIONS` is empty, since v0's format is the first.
- A `Migration` rewrites one file's text from format `from` into `from + 1`.
- `steps` gives the chain from one format to another. A gap is refused, and so is a newer format, since migrations only go forward.
- `migrate` runs the chain on a file, and refuses a file with syntax errors.
- `rename_properties` is the step that a migration generated from a Blender rename takes. Each `Rename` names a datablock kind, the blocks inside it (`&["render"]`) and the property's old and new names. It changes property lines and `old=` entries there and nothing else, and leaves a file it already rewrote as it is.
- `docs/rfcs/README.md` says that each RFC raising the format adds its step there.

**Cosmósis** (`cosmosis::migrate`):
- `format_of` reads the manifest's `null-format`, `0` included.
- `project` runs the steps over every source except `cosmosis.noll`, `.sau/ids.noll` and `view.noll`, whose forms are Cosmósis's and Sau's own (S66). It writes the newest format into the manifest. Nothing is migrated when a format has no step (`cosmosis-migrate-no-step`) or a file does not parse (`cosmosis-migrate-unparsed`).
- `write` stages every changed file beside its target, through a link as write-back writes. Only then does it replace them, with the manifest last, so a stop part-way leaves the old format named and a second migration finishes it.
- `NEWEST_NULL_FORMAT` is now Null's `FORMAT`.
- The manifest's messages name Sau rather than Cosmósis: "newer than this Sau reads (format 1)".

**`sau migrate [path]`:**
- A project in this format: "nothing to migrate", exit 0.
- A newer or malformed format: the manifest's error.
- An older format: refused while the layout has errors. Otherwise the project is migrated and written, and each file is listed.
- It is in the help and in section 7's table. Section 4 says how migrations work.

**A decision of mine: no error for an older readable format yet.** The manifest still reads only formats 1 and up, and 1 is the newest, so no format can be older and still read. Instead of an error no case could give, a compile-time assertion in `manifest.rs` stops the build once `NEWEST_NULL_FORMAT` goes above 1. It names what is then needed: an error for an older format that names `sau migrate`. When that happens, `sau check` and Sau's load refuse an unmigrated project instead of reading old text as new.

**Not done, for Sau:** Sau's open does not offer the migration. `sau_core::project::load` gets the manifest's errors, as `sau check` does. Offering the migration on open is Sau's row.

**Tests:**
- **Null**, 4 in `tests/migrate.rs`, plus a doctest:
  - a rename changes only what it names, including an `old=` entry, a nested block's line and an object named `@focal-length` left alone;
  - a second run changes nothing;
  - two steps run in order;
  - a gap and a newer format are refused, with the message;
  - syntax errors are refused.
- **Cosmósis**, 5 in `cosmosis/tests/migrate.rs`:
  - a test migration lifts a format-0 project to format 1. The camera's line is the only change, the manifest reads, every file checks against the schema, no temporary file is left, and a second migration has nothing to do;
  - a migration stopped before the manifest finishes the second time;
  - format 0 with Null's own (empty) migrations is refused, with nothing written;
  - a file with syntax errors stops it;
  - `format_of` finds the format on the line or in the block, and refuses `01`, words and strings.
- **`sau migrate`**, 4 in `sau_cli/tests/migrate.rs`:
  - format 1: nothing to do;
  - format 2: refused, naming format 1, by `sau migrate` and `sau check`;
  - format 0: refused, with nothing written;
  - its help and usage.

`just check` passes in both repositories.
