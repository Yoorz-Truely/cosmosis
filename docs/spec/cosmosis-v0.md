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
  collections/                 collections shared between scenes or packaged
  materials/                   shared materials
  nodes/                       shared node groups: geometry, shader, compositor
  actions/                     shared actions
  rigs/                        armatures and their bone collections
  worlds/                      worlds and their HDRIs
  scripts/                     optional Python generators (Null, section 13) and text datablocks
  checks/                      checks on the scene and renders (Null, section 14)
  media/<sha256>.<ext>         images, video, sound, fonts, volumes, caches, bakes
  data/<sha256>.<kind>         level 0 geometry too large to inline (Null, section 9)
  view.noll                      where the project was left: workspaces, viewport, selection, open files
  settings.noll                  optional settings for this project only, over the user's own
  renders/                     rendered images and exports, by default
  .sau/                        cache: the .blend snapshot, thumbnails, the ID map, undo history
  .gitignore
```

### Conventions

- **The root** is the folder holding `cosmosis.noll`. A project of kind `project` also needs `project.noll`; every other file and folder is optional.
- **Composition.** A scene file lists its collections and objects by handle; a collection lists its children. File names never decide order or membership, so reorganising is moving lines, never renaming files.
- **Components.** Data used by only one object (its mesh, its modifiers, a material nobody else uses) is written in that object's file, as a component keeps its own state. Data used by two or more lives in its type folder. Sau moves a block between the two as sharing changes, as one write-back (proposed; NULL.md, section 20).
- **One datablock, one place.** A handle is defined once in the project. Defining it twice is an error that names both files.
- **`view.noll`** holds interface state an AI may read and edit (which camera the viewport looks through, what is selected, which files are open) and is not in Git.

### Version control

- Every file except `.sau/`, `view.noll` and `renders/` is source and belongs in Git. Cosmósis writes a `.gitignore` saying so.
- `media/` and `data/` are content-addressed, so they suit Git LFS; Cosmósis's `.gitattributes` template marks them for it.
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

## 5. Packages

A package is a Cosmósis project of another kind, with a manifest, versions, and a licence report. Packages come from folders, Git repositories and, later, the registry, **Genesis-Preim** (named by the user on November 6, 2026), used through `preim` commands as npm is beside Next.js: `preim add <package>` adds one and updates the lockfile, `preim search` and `preim publish` find and share them; handles from it are addressed as `@package:handle` (Null, section 7). Blender's own asset libraries and the Essentials bundle convert into packages with `sau convert`. Blender add-ons stay Blender extensions and are not Cosmósis packages.

## 6. Templates

`sau new <template> <path>` starts a project from a template package: `empty`, `product-shot`, `character`, `environment`, `motion-graphics`, `architectural`, `game-asset`. Each template is a complete project with settings pinned, checks included and a short README.

## 7. Commands

| Command | Does |
| --- | --- |
| `sau new` | Start a project from a template |
| `sau open <project>` | Open a project in Sau |
| `sau dev <project>` | Like `npx convex dev`: watch the project, check every save, apply it to the open Sau or a headless one, and report changes and errors as they happen |
| `sau watch <query>` | Subscribe to a live value (an object's world position, a polygon count, the diagnostics, a capture from a camera) and print it each time it changes |
| `sau check` | Parse and check every file, with diagnostics |
| `sau fmt` | Print files in canonical form |
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
