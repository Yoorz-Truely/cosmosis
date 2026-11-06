# Cosmósis agent guide

Cosmósis is Sau's project framework and owns the `sau` command ([README](README.md)). This file is the router for agents. Nothing is built yet.

## Always

- Read the [specification](docs/spec/cosmosis-v0.md) before changing what a project folder, manifest, package or command means.
- Decisions the user settled are not reopened.
- Original code only, with permissive dependencies checked first. Never copy GPL-family code, including Blender's: Cosmósis is MIT and starts Sau as a separate program.
- Doc-only commits need only a Markdown check; code commits need `cargo fmt`, `cargo clippy` and `cargo test` to pass.
- Commit focused changes; never push unless the user asks.
- Sau (`~/Projects/Sau`), Null (`~/Projects/Null`) and Baarton (`~/Projects/Baarton`) are separate repositories. Change them only when the task says so.

## Task routes

| Task | Read |
| --- | --- |
| Layout, manifest, packages, templates, commands | `docs/spec/cosmosis-v0.md` |
| What goes inside the files | Null's `docs/spec/null-v0.md` |
| Where this fits in the product; the plan for this repository | Sau's `docs/ROADMAP.md` and `docs/REPOSITORIES.md` |
| The framework Cosmósis is modelled on | Baarton's `docs/spec/baarton-v0.md` |
