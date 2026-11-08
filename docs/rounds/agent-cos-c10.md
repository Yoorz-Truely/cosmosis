# agent/cos-c10

agent/cosmosis-a's branch for C-10, in `work/wt/cos-c10/`. It is on branch `agent/cos-c10` in Cosmósis, cut from `agent/null-c09` at its review (`b09074c`), which holds C-07, C-09 and 1d-07a's check. Null is a detached worktree at agent/null-1d06b's `19287e7` (1d-06b round 2, approved), which reads level 0 data files. Sau and Scowr are links to their main checkouts. November 8, 2026. Tests and lints only.

## C-10: level 0 data files in the layout

My proposal from the 1d-06b review. Null's checker matches a data file to its line by the `array` node's owner and array name. Whether the line's `file=` path names that file is Cosmósis's question, since only Cosmósis knows the paths.

**What changed.**
- **`cosmosis::data_files` (new).**
  - `is_data_file(text)`: the file's one live node is `array`.
  - `check(sources)` resolves each `file=` from the folder of the file that names it, normalising `.` and `..`, and checks the target against the line. Its errors, each at the line's `file=` value:
    - `cosmosis-data-file-outside`: the path leaves the project.
    - `cosmosis-data-file-missing`: it names no file. When the project holds a data file with this line's header somewhere else, the help names that file with the path to write, and that file is then not also reported as unnamed.
    - `cosmosis-data-file-not-data`: it names a file that is not a data file.
    - `cosmosis-data-file-header`: the data file's `array` node names another owner or array, with the node as a related place.
    - `cosmosis-data-file-twice`: a second line names a file already named, with the first line as a related place.
  - **A data file no line names** is `cosmosis-data-file-unnamed`, at the file.
  - **A line's array** is the line's own name for `vertices`, `edges` and `polygons`, and its handle otherwise (`attribute @heat`, `uv-map @uv-map`), as in Null's checker.
- **Null's matching errors are replaced, not doubled.** `schema_check` drops Null's `null-handle` diagnostics at a naming line's name and at a data file's `array` node, the two spans where Null reports its header matching. Cosmósis's errors take their place, so one mistake gives one error, with a cause that follows the path rather than the header. Null's other diagnostics of data files are kept: shape, `count=`, `bounds=` and the elements.
- **`sau fmt`** writes a data file as its `array` line, one element per line indented two spaces with single spaces inside, and `]` (`data_files::layout_of`). A data file holding a comment is left as it is, since the layout would move the comment.
- **`sau check`** counts data files apart: "8 Null files check, 3 of them data files". A data file is never taken for a datablock file.
- **Corpus.** `corpus/project/rock-data.sau` is a rock mesh whose vertices, polygons and a float attribute (with `-0`, by S75's order) are all in data files. It checks clean and is canonical, so the corpus tests cover it as well.
- **Spec, section 2.** The layout tree shows a data file beside its mesh's file. `data/` is now for binary data a project opts into, since S74 (2) moved level 0 text out of it. A new convention, "Data files", gives the rule and the six codes.

**Tests.** In `crates/sau_cli/tests/data_files.rs`:
- **The corpus rock** checks clean, with its data files counted.
- **A data file in another folder**, named with `../`, checks clean.
- **Each mistake is exactly one error at its line:**
  - a missing file, with no copy anywhere;
  - a missing file whose array is held elsewhere, where the help names it;
  - a header with another array, and one with another owner;
  - a file named by two lines;
  - a data file no line names;
  - a path out of the project;
  - a path to a datablock file.
- **`sau fmt`** turns a one-line data file and an uneven one into the corpus's layout, and `sau fmt --check` passes on the corpus.
- **Unit tests** cover path resolution, what a data file is, and the layout.

**Not done here.** Binary data files (`file="scan.vertices.bin"`) are not specified yet (Null, section 9), so a path to one is reported as missing or as not a data file.

**Checks.** `just check` passes, and the sau_cli and cosmosis tests pass.
