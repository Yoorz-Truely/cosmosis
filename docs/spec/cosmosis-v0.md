# Cosmósis: Sau's project framework

Draft v0, November 6, 2026. Nothing is built yet. The name Shoon was chosen by the user on November 6, 2026, and changed to Cosmósis on November 7, 2026. It lived in Sau as `docs/SHOON.md` until it moved to this repository on November 6, 2026, as Baarton's spec did. Where it fits in the product: Sau's `docs/ROADMAP.md`.

Cosmósis is the standard every Sau project follows: how a project folder is laid out, which language goes where, how projects declare and share packages, how a new project starts from a template, and which commands check, test, convert, render and deliver it. It plays the part Next.js plays for web apps, and Baarton for Crescend.

| Web development | Sau |
| --- | --- |
| JavaScript or TypeScript, the language | **Null**, the language (`Null/docs/spec/null-v0.md`) |
| React, the runtime that compares and applies changes | **Sau**, the application, Blender's model and live reload |
| Next.js: layout, conventions, packages, dev server, `create-next-app` | **Cosmósis**, this standard |

Cosmósis is deliberately thin: a short standard plus the commands that apply it, not a second language or a runtime.

## 1. Principles

1. **A project is a folder anyone can read.** People, AI and outside tools can understand and change it without Sau open.
2. **Conventions, and composition like React.** Folders have conventional homes, and composing files decide what is in a scene, the way a parent component decides which children render.
3. **Reproducible.** The same files and the same lockfile give the same scene and, with fixed seeds, the same render on any machine with the same Sau.
4. **Packages are first-class.** Materials, node groups, models, rigs, brushes, HDRIs and templates are shared between projects as versioned packages.
5. **Null first.** Everything that can be Null is Null. Other languages appear only where Null does not fit (section 3).

## 2. The project folder

```text
Harbor at Dusk.sau/            a folder; macOS shows it as one document
  cosmosis.noll                  the manifest: name, Sau and Null versions, dependencies
  cosmosis.lock                exact versions and hashes of every package used
  project.noll                   project settings: units, colour management, scenes
  scenes/                      one file per scene: frame range, render settings, its collections
  objects/                     objects, with data, modifiers and constraints used only by them
    rock.vertices.noll           a mesh's array too large to inline, beside the mesh's file
  collections/                 collections shared between scenes or packaged
  materials/                   shared materials
  nodes/                       shared node groups: geometry, shader, compositor
  actions/                     shared actions
  rigs/                        armatures and their bone collections
  worlds/                      worlds and their HDRIs
  scripts/                     optional Python generators (Null, section 13) and text datablocks
  checks/                      checks on the scene and renders (Null, section 14)
  media/<sha256>.<ext>         images, video, sound, fonts, volumes, caches, bakes
  data/<sha256>.<kind>         binary data a project opts into, such as a scan (Null, section 9)
  view.noll                      where the project was left: workspaces, viewport, selection, open files
  settings.noll                  optional settings for this project only, over the user's own
  renders/                     rendered images and exports, by default
  .sau/                        cache: the .blend snapshot, thumbnails, undo history
    ids.noll                     the map of handles to persistent IDs: source, kept in Git
  .blender_project/
    project.toml                 Blender's own project file, named as the package (S45)
  .gitignore
```

### Conventions

- **The root** is the folder holding `cosmosis.noll`. A project of kind `project` also needs `project.noll`; every other file and folder is optional.
- **Composition.** A scene file lists its collections and objects by handle; a collection lists its children. File names never decide order or membership, so reorganising is moving lines, never renaming files.
- **Components.** Data used by only one object (its mesh, its modifiers, a material nobody else uses) is written in that object's file, as a component keeps its own state. Data used by two or more lives in its type folder. Sau moves a block between the two as sharing changes, as one write-back (proposed; NULL.md, section 20).
- **One datablock, one place.** A handle is defined once in the project for its kind. Defining it twice is an error that names both files, each with its line.
- **Also a Blender project** (S45; Sau task 1d-04c). `.blender_project/project.toml` is Blender's own project file, so Blender's Project Settings show the project. Its name is the package's, from `cosmosis.noll`, which is the source: Sau writes the file when the manifest's name differs, changing only its top-level `name` and keeping what Blender's settings add (an OpenColorIO config, a splash image, variables, asset libraries), so a name changed in Project Settings is replaced at the next sync. The file is source and belongs in Git. A project without the folder (the corpus) is still a Blender project while Sau has it open, in memory only; Sau creates no file for it.
- **References need no imports.** A handle defined in any file is found from every other (S36); a reference that names nothing is an error at its line.
- **Files that hold no datablocks.** `cosmosis.noll` is the manifest (section 4). `settings.noll`, `view.noll` and `.sau/ids.noll` are Null text with forms of their own, not yet specified; until then they are checked only as UTF-8 text that Null's parser reads (Sau's decision S66). `.sau/ids.noll` is the one file in `.sau/` that is read (Sau's decisions S61 (2) and S66; its form is Sau's task 1b-01f). A link to nothing there is an error, as anywhere in the project; a `.sau` that is a link to a folder is not followed, as no link to a folder is, so the map is not read through it (a note says so).
- **Data files** (Null, section 9; Sau's decision S74 (2); task C-10). A mesh's array too large to write inline is a Null file of its own, one element per line, holding one `array` node (`array mesh @rock vertices [ … ]`), named by its line's `file=` with a path from the folder of the file that names it (`vertices file="rock.vertices.noll" count=… bounds=…`), usually beside it. It is never read as a datablock file, only through its line, and `sau fmt` writes it one element per line. `sau check` reports, each at the line: a path that leaves the project (`cosmosis-data-file-outside`), names a file in `media/` or `data/`, whose files are named by their contents (`cosmosis-data-file-place`), names no file (`cosmosis-data-file-missing`) or names a file that is not a data file (`cosmosis-data-file-not-data`); a data file whose `array` node names another owner or array (`cosmosis-data-file-header`); and a data file named by two lines (`cosmosis-data-file-twice`). Where the project holds the line's array in another data file, the help names it, and that file is not reported again. A line's owner is its datablock's handle as Null reads it, written or made from the display name (S18). A data file no line names is an error at the file (`cosmosis-data-file-unnamed`). These take the place of Null's own matching by the `array` node, so each mistake is one error.
- **`view.noll`** holds interface state an AI may read and edit (which camera the viewport looks through, what is selected, which files are open) and is not in Git.

### Ignored names

A project ignores files and folders that editors and the system make beside its own, even when their names end in `.noll`. Editors save by writing a temporary file and renaming it over the original; a check that read the temporary file would see its datablocks defined twice. Ignored are names:

- starting with `.` (hidden: `.DS_Store`, macOS's `._x`, Vim's `.x.swp`, Emacs's `.#x`, `.sau/`) or with `~` (`~$x`, `~x`);
- ending in `~` (backups) or `#` (Emacs's `#x#`);
- ending in `.tmp`, `.temp`, `.bak`, `.swp`, `.swo`, `.swx`, `.crswap` or `.vsctmp` (VS Code's atomic save), in any case;
- holding `.sb-` (macOS's safe save, `x.sb-1a2b3c4d-AbCdEf`), `___jb_` (JetBrains' safe write) or `.noll.` (a copy of a Null file under another name, such as `x.noll.saving.noll`);
- of a folder wholly in parentheses, as macOS names the folder it saves through, `(A Document Being Saved By TextEdit)` in English and in other words in other languages.

So a project's own files are never named so. An entry that disappears between a folder's listing and its reading is skipped too, as a temporary file renamed away has (task C-04).

### Version control

- Every file except `.sau/`, `view.noll` and `renders/` is source and belongs in Git; in `.sau/`, `ids.noll` is source too, since a project cloned without it would give its datablocks new persistent IDs (S66). Cosmósis ships a `.gitignore` saying so (`templates/project/gitignore`: `.sau/*`, `!.sau/ids.noll`, `/view.noll`, `/renders/`).
- `media/` and `data/` are content-addressed, so they suit Git LFS; Cosmósis's `.gitattributes` template (`templates/project/gitattributes`) marks them for it. `sau new` writes both into a new project (task 1d-04e).
- A merge driver for `.noll` files merges by handle rather than by line where Git's line merge fails.

## 3. Languages and where they go

| What | Language | Where |
| --- | --- | --- |
| The project: everything a `.blend` would hold | Null | every `.noll` file |
| Procedural geometry and shading | Blender's node systems, written in Null | `nodes/` and inside materials and objects |
| Computed parts that nodes cannot express | Python, as generators whose output is Null | `scripts/` |
| Shaders written as code | OSL, as Blender reads it | `scripts/` or `nodes/`, referenced from a script node |
| Driver expressions | Blender's simple expressions | on the driven property |
| Settings, keymaps, themes | Null | `settings.noll`, or the user's own folder |

## 4. The manifest

```null
// cosmosis.noll
package "harbor-at-dusk" {
  kind project              // project, material, node-group, model, rig, template, library
  version 0.1.0
  sau 0.1
  null-format 1
  license "All rights reserved"

  dependencies {
    cosmosis-materials "^1.2"  // a package from the registry
    my-trees path="../Trees.sau"
  }
}
```

The manifest pins the Sau version and Null format a project was written for, so an older project opens with a migration rather than a surprise.

How it is read (November 7, 2026, Sau task C-02; `crates/cosmosis/src/manifest.rs`):

- The name, the quoted string after `package`, and `null-format` are required: they are all a manifest needs (S37, S45). Every other field is optional, and `kind` is `project` unless written. Package and dependency names are kebab-case (S16).
- `version` is `major.minor.patch`; `sau` is `major.minor` or `major.minor.patch`; both are whole numbers written bare, without leading zeros, as semantic versioning writes them (`0.1.0`, `1.10.2`, never `01.2.3`). `null-format` is a whole number from 1, also without leading zeros; a format newer than this Sau reads is an error naming the format it reads, however large the number, and an older one is migrated (below). `license` is a quoted string.
- A dependency names a quoted version requirement (`"^1.2"`, `">=1, <2"`), a `path=` to a folder, or both. An empty `path=""` is an error.
- Any field may be written on the package's line (`package "harbor" null-format=1`), as everywhere in Null.
- **Migrations** (S45, Sau task 1d-04d). `sau migrate [path]` rewrites a project of an older Null format in the format this Sau reads. Null's steps (`null_check::migrate`) run one format at a time over every Null file but `cosmosis.noll`, `.sau/ids.noll` and `view.noll`, whose forms are Cosmósis's and Sau's own, and the new format is then written into `cosmosis.noll`. A step generated from a Blender rename changes only the property it names (`null_check::migrate::rename_properties`), and leaves a file it already rewrote as it is. It lifts one format at a time: each step's changed files are staged beside their targets before any is replaced, and then the manifest's format is raised by one, so a stop part-way leaves that step's format named and a second `sau migrate` repeats that step alone, never an earlier one. One step's renames may not chain or swap (`a` to `b` with `b` to `c`): a chain is two steps. Refused, with nothing written: a newer format (`cosmosis-manifest-newer-format`), a format no step reads (`cosmosis-migrate-no-step`), and a file with syntax errors (`cosmosis-migrate-unparsed`). Format 1 is v0's and the first, so Sau ships no migration yet: a step comes with each RFC that raises the format (Null's `docs/rfcs/README.md`). Until a project is migrated, the reader takes only the format it reads.
- Reading is strict (S24): a field not listed here, a value of the wrong form or a field written twice is an error with a stable code starting `cosmosis-manifest-` and, where one exists, a suggestion.
- `cosmosis.noll` is a Null file first. Its syntax errors and node names that are not kebab-case are Null's own errors (`null-syntax`, `null-name`), reported before the manifest is read, and the file is not read for fields until they are fixed. A dependency's name is a node's name, so `Trees "^1"` is Null's `null-name`; the package's name is a quoted string, which Cosmósis checks (`cosmosis-manifest-bad-name`).

## 5. Packages

A package is a Cosmósis project of another kind, with a manifest, versions, and a licence report. Packages come from folders, Git repositories and, later, the registry, **Genesis-Preim** (named by the user on November 6, 2026), used through `preim` commands as npm is beside Next.js: `preim add <package>` adds one and updates the lockfile, `preim search` and `preim publish` find and share them; handles from it are addressed as `@package:handle` (Null, section 7). Blender's own asset libraries and the Essentials bundle convert into packages with `sau convert`. Blender add-ons stay Blender extensions and are not Cosmósis packages.

## 6. Templates

`sau new <template> <path>` starts a project from a template package: `empty`, `product-shot`, `character`, `environment`, `motion-graphics`, `architectural`, `game-asset`. Each template is a complete project with settings pinned, checks included and a short README.

`sau new empty <path>` is built (Sau task 1d-04e). It makes the folder, which must not exist yet or be empty, with:

- `cosmosis.noll`, the package named after the folder in kebab case (`Harbor at Dusk.sau` gives `harbor-at-dusk`);
- a `project.noll`;
- the `.gitignore` and `.gitattributes` of section 2's "Version control";
- `.blender_project/project.toml`, naming the Blender project as the package (task 1d-04c).

The project it makes checks clean. The other templates come with their packages.

## 7. Commands

| Command | Does |
| --- | --- |
| `sau new` | Start a project from a template |
| `sau open <project>` | Open a project in Sau |
| `sau dev <project>` | Like `npx convex dev`: watch the project, check every save, apply it to the open Sau or a headless one, and report changes and errors as they happen |
| `sau watch <query>` | Subscribe to a live value (an object's world position, a polygon count, the diagnostics, a capture from a camera) and print it each time it changes |
| `sau check` | Parse and check every file, with diagnostics: against Null's schema of all project data, and a datablock's `raw` block against Null's raw table, `schema/null-raw.json` (Null, section 19; Sau task C-11), for an unknown field, a value its DNA type cannot hold or a list of another length. While Null ships the table's placeholder, which lists no struct until a build of Sau writes it, `raw` blocks are not checked and a note at the first says so (`cosmosis-raw-unchecked`) |
| `sau fmt` | Write the project's Null files in canonical form (built, Sau task C-07): Null's canonical printer, `null_print::canonical`, for every datablock file, in Blender's schema order with comments kept, then a line holding its property's default left out unless a comment says why it is written (`null_print::without_defaults`, Sau task C-09); the manifest's fields in section 4's order; `settings.noll`, `view.noll` and `.sau/ids.noll` in the formatter's layout only. Formatting twice changes nothing. `--check` writes nothing and fails, listing them, when a file is not canonical. The corpus is kept canonical |
| `sau rename @old @new [path]` | Rename a datablock's handle in every file that names it (built, Sau task 1b-01h, decision S17), with Null's `null_check::rename` (Null, section 7, "Renaming a handle"). The datablock keeps its persistent ID: `.sau/ids.noll`, when the project has one, holds the ID under the new handle. Every check comes before any write: each changed file's text is staged beside its target, written through a link as write-back writes it, then renamed over it. The map goes first, never through a link, and then the files in their paths' order, so a running Sau that loads between two renames sees either nothing changed (the map ahead of the files) or files that fail the check, never every file renamed with the map behind. An entry's block of sub-object entries (S73) moves with it. Refused, with nothing written: a handle two kinds share, a new handle the kind already has, a file with syntax errors, a map that exists but cannot be read or has a line it cannot use, and a `.sau` or map that is a link |
| `sau migrate [path]` | Rewrite a project of an older Null format in the format this Sau reads (built, Sau task 1d-04d, decision S45): section 4, "Migrations" |
| `sau convert` | `.blend` to a project and back; asset libraries to packages |
| `sau import` / `sau export` | Any format Blender reads or writes, into or out of the project |
| `sau render` | Render stills, animations and passes, headless |
| `sau bake` | Simulations, lighting and texture bakes into `media/` |
| `sau test` | Run the checks |
| `sau diff` | Compare two commits as text and as renders |
| `sau lift` | Lift geometry or images to higher levels (Null, section 9) |
| `preim add` / `preim remove` / `preim search` / `preim publish` | Manage packages (Genesis-Preim) |
| `sau lsp` | Start the language server |

### Actions on the open project

Files describe; commands act. These reach the running Sau through the gateway, the same for a person in a terminal, an AI and Sau's buttons: `sau frame 120`, `sau play`, `sau stop`, `sau view camera @shot-cam`, `sau view frame @pier`, `sau select @pier`, `sau shot` (a viewport capture), `sau probe 640 360` (what is at a pixel), `sau op <operator> [args]` (any Blender operator by name).

## 8. Decisions

Settled by the user on November 6, 2026:

1. **The name** is Cosmósis (November 7, 2026; first Shoon). Where an accent cannot go (commands, crates, file and folder names, package names) it is written `cosmosis`; everywhere people read it, Cosmósis.

Proposed, waiting for the user: the folder layout above and the component rule for single-use data. The registry's name is settled: Genesis-Preim.
