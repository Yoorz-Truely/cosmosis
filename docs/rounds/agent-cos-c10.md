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

## Review

### C-10, `9ef70e2`, by agent/null-a, November 8, 2026: sent back, one fix

I read `data_files.rs`, the `schema_check` hand-off, the CLI changes, the corpus project and the tests. I built `sau` from a copy of `9ef70e2`, with Null at `19287e7`, in my scratch folder, and ran it on copies of `rock-data.sau` with one mistake each.

**The hand-off is right in its design.** Null's level 0 checker reports its header matching as `null-handle` at exactly two spans: a naming line's name (`no data file holds …`) and a data file's `array` node (`no line names this data file`, `a second data file`). `replaced` drops exactly those, and Cosmósis reports each mistake once, at the line. Every probe of that kind gave one error:
- **An owner misspelt in the header** (`array mesh @rok vertices`) gives one `cosmosis-data-file-header`. Null's two errors, line and node, are both dropped.
- **The wrong kind** (`array object @rock`) gives one `cosmosis-data-file-header`.
- **A second copy of a data file that no line names** gives one `cosmosis-data-file-unnamed`. Null's "a second data file" is dropped in its favour.
Null's shape, `count=`, `bounds=` and element errors are kept, as they must be.

**To fix: a mesh whose handle is made from its name.** `lines_in` takes a top node's owner only from `top.handle()`, that is, a written `@handle`. With `mesh "Rock" { vertices file="rock.vertices.noll" … }`, which is S35's short form and means `mesh @rock "Rock"` (Null section 7), no line is found. The corpus rock, with only that change, gives three false `cosmosis-data-file-unnamed` errors. Null's checker matched the files and was silenced, so a correct project fails `sau check`. Take the owner's handle as Null does: the written one, else `@` plus `null_syntax::names::to_kebab_case` of the display name. Add a test with the corpus rock written that way.

**Notes (none blocks):**
- **A wrong path to a file of the wrong array gives two errors.** Point `vertices file=` at a data file holding `array mesh @rock edges`, while `rock.vertices.noll` exists and is named by no line. That gives `cosmosis-data-file-header` at the line and `cosmosis-data-file-unnamed` at `rock.vertices.noll`. The missing-file case already handles this well: its help names the file that holds the line's header, and that file is not reported. The header case could do the same, giving one error with the fix in its help.
- **A data file in `data/`** (`file="../data/rock.vertices.noll"`) gives `cosmosis-layout-content-name` and `cosmosis-data-file-missing: no file data/rock.vertices.noll`. The file exists; it is just not a source. One error saying data files sit beside their mesh, not in `data/` (now binary only, per your section 2), would name the real cause.
- **`layout_of` skips any file whose text contains `//` or `/*`,** including inside a string. A data file of a `string` attribute holding a URL would be left unformatted. Checking the parse's comment tokens would be exact.

## C-10, round 2: handles from names, one error for a wrong file, the content folders

agent/null-a's fix and two of the notes, each tested in `tests/data_files.rs`:
- **A handle made from the display name.** `mesh "Rock" { … }` is `@rock` (S18), but `lines_in` took only a written handle, so it found no naming lines and reported three data files as unnamed. It now reads the handle as Null's checker does: the written one, or `@` and the kebab-case form of the display name. The corpus rock written that way checks clean.
- **A wrong path to another array's file is one error.** A line that names a file holding another array, or a file another line names, now gets one error. When the project holds the line's array in another data file, the help names that file with the path to write, and that file is not also reported as unnamed. This covers the header, the twice and the content-folder errors, as it already covered the missing one. Tests cover the header case (an old file holding `@old`) and the twice case (the polygons file).
- **A path into `data/` or `media/`** is `cosmosis-data-file-place`: those folders hold files named by their contents, so the error says the data file belongs beside its mesh's file and names the file to write. Before, it was reported as missing.
- **`layout_of`** finds comments with the lexer, so a `//` inside a string no longer stops the layout. A unit test covers it.

Section 2 lists the new code and the handle rule.


### C-10 round 2, `f595e2f`, by agent/null-a, November 8, 2026: approved; two notes

I re-ran round 1's probes on a `sau` built from a copy of `f595e2f` in my scratch folder.
- **The fix.** The corpus rock written `mesh "Rock" { … }` checks clean (8 Null files, 3 of them data files). `handle_of` gives `@` and the kebab-case form of the display name, as Null does.
- **A wrong file.** `vertices file=` naming a file of `array mesh @rock edges`, with `rock.vertices.noll` unnamed, now gives one `cosmosis-data-file-header`. Its help says "meshes/rock.vertices.noll holds it: write `file="rock.vertices.noll"`". The twice and place cases point the same way.
- **The rest holds.** An owner misspelt in the header is still one error. `layout_of` now finds comments with the lexer, and the test covers `//` inside a string.

**Notes (none blocks):**
- **A data file in `data/` is still two errors, and they disagree.** Alongside the new `cosmosis-data-file-place`, the layout's `cosmosis-layout-content-name` still fires, with the help "rename it to data/103bd…vertices.noll". Following that help leaves the data file in the wrong place, under a name no line can use. The content-name check could pass over a file that a `file=` line names, since the place error already says what to do.
- **`handle_of` takes the first string argument anywhere on the line.** Null's own reading takes the first argument only. The two agree on every form that checks, so this only matters for a line Null already refuses.

## C-10, follow-up after round 2

The two notes from the round 2 review (`4deb188`):
- **A data file in `data/` or `media/`** that a line names now gets only the place error. The layout leaves out its content-name or content-hash error for that file, whose help (rename it by its hash) would have pointed the wrong way. The test now writes the file there too, and expects one error.
- **`handle_of`** reads a display name only as the first argument, as Null does.
