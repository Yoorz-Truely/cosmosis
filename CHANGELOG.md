# Changelog

## Unreleased

- November 7, 2026: `sau check` checks every Null file against Blender's schema with Null's checker (`null_check`, by path), after the parser; a file the parser already found wrong reports only that. The six slice projects in `corpus/slice/` pass it: cubes with one `size`, `polygons` for inline faces, materials reaching `output-material` (Sau task 1a-15).
- November 7, 2026: `sau check` reads `cosmosis.noll` with Null's parser and checks its fields, passes the package's kind to the layout check (a project needs `project.noll`), and parses every Null file, reporting Null's own diagnostics (Sau task 1a-11). Cosmósis depends on Null's `null_syntax` by path.
- November 7, 2026: the `sau` command's first command, `sau check`, up to the layout stage: it finds a project's root, checks that every `.noll` file is UTF-8 text and that `media/` and `data/` files are named by their SHA-256, and reports diagnostics with stable `cosmosis-` codes (Sau task C-01). The `cosmosis.noll` manifest's data model and every error reading it, from a parser-neutral tree that Null's parser will fill (C-02; spec, section 4). Six draft projects of phase 1a's slice in `corpus/slice/` (C-03).
- November 7, 2026: renamed from Shoon to Cosmósis, written `cosmosis` where an accent cannot go: `cosmosis.noll`, `cosmosis.lock`, the spec `docs/spec/cosmosis-v0.md`, the folder `~/Projects/Cosmosis`.
- November 6, 2026: the repository is created. The draft v0 specification moves here from Sau's `docs/SHOON.md`. The name Shoon was chosen by the user.
- November 7, 2026: the folder layout complete (S36, S66): a handle defined twice names both files and lines, a reference is found in any file, `.sau/ids.noll` is read (and kept in Git) while the rest of `.sau/` is skipped, `settings.noll` and `view.noll` are checked for syntax only, and the `.gitignore` and `.gitattributes` templates ship in `templates/project/`. Sau task 1d-04b.
- November 8, 2026: `sau check` checks against Null's schema of all project data (`null-schema-project.json`), so every datablock type's properties are checked, not only the slice's. Sau task C-06.
