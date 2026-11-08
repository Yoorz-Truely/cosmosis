# agent/cos-c11

agent/cosmosis-a's branch for C-11, in `work/wt/cos-c11/`.
- **Cosmósis:** branch `agent/cos-c11`, cut from `agent/null-c09` at its review (`b09074c`).
- **Null:** branch `agent/cos-c11`, cut from agent/sau-y's `4badb11` (1d-05b round 2, approved, with the raw table and the checker's `raw` blocks). C-09's Null side, `c99aac4`, is merged in, since Cosmósis's `sau fmt` needs `null_print::without_defaults`.
- **Sau and Scowr:** links to their main checkouts.

November 8, 2026. Tests and lints only.

## C-11: `sau check` loads `null-raw.json`

**What changed.**
- **`schema_check` loads the raw table with the schema.** `raw_table()` reads Null's `schema/null-raw.json`, compiled in by path as the project schema is, and `schema()` adds it with `Schema::with_raw`. A datablock's `raw` block is then held to DNA by Null's checker:
  - a field the table does not list for the datablock's struct is refused;
  - so is a value its type cannot hold;
  - so is a list of another length.
- **The placeholder.** Null ships a placeholder table, with no structs, until a build of Sau writes the real one (`just schema-sync`). Read as a table, it would refuse every raw field, so `raw_table()` is `None` for it. `raw` blocks then go unchecked, and `sau check` puts one note, `cosmosis-raw-unchecked`, at the first block, naming the placeholder and how to replace it. The note fails nothing.
- **`check_with(schema, sources)`** checks against a schema the caller gives; `check` uses the shipped one. The tests use `check_with` with a table of their own, which stands in for the build's.
- **Corpus.** `corpus/project/raw-fields.sau` is section 19's buoy: `restore_mode`, `col_mask`, `collision_boundtype` and `constinv`, `Object` fields of Blender 5.3's SDNA that no RNA property reads. It checks clean and is canonical, so the corpus tests cover it too. With today's placeholder, `sau check` passes it with the note.
- **The spec's `sau check` row** says so.

**Tests.** In `crates/cosmosis/tests/raw.rs`, against a table listing the corpus's four fields by their SDNA types:
- **The corpus buoy** checks clean.
- **An unknown field** (`restore_mood`) is one `null-unknown-property` error, at its line.
- **Out-of-range values** are each one error, at their lines: `col_mask 70000`, `collision_boundtype 256` and `restore_mode 2147483648`. So is a `constinv` row short.
- **Without a table,** the note sits at the first `raw` block, and with one there is no note. The shipped schema carries a table exactly when Null ships one.

**Not done here.**
- **`sau check` on the real table.** The Done-when wants `sau check` itself to report an unknown field. It can only do so against a real table, which no build has written yet, so the tests reach the same code through `check_with` with a table of their own. The first `just schema-sync` after a build turns this on in `sau check`. At that point the corpus buoy must check clean against the real table too, and the corpus test will show it.

**Checks.** `just check` passes.
