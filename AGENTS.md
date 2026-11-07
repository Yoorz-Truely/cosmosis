# Cosmósis agent guide

Cosmósis is Sau's project framework and owns the `sau` command ([README](README.md)). This file is the router for agents. Building starts with Sau's phase 1a; the shared workflow, task board and limits are Sau's.

## Always

- Read the [specification](docs/spec/cosmosis-v0.md) before changing what a project folder, manifest, package or command means.
- Decisions the user settled are not reopened.
- Original code only, with permissive dependencies checked first. Never copy GPL-family code, including Blender's: Cosmósis is MIT and starts Sau as a separate program.
- Work from Sau's task board (`../Sau/docs/TASKS.md`, `just tasks` in Sau): claim a task before starting it, hold at most three, and move each through `review` and `done` (`../Sau/docs/agents/workflow.md`). Read Sau's `HANDOFF.md` for the current state.
- Work in your own worktree on your own branch, never in the main checkout or on `main` unless you are the integrator: `just wt-new <name> --repos cosmosis` in Sau makes one (`../Sau/docs/agents/parallel.md`).
- Commit every working change as soon as it works and `just check` exits 0 on its own (format, Clippy with warnings as errors, tests); a commit of documents alone needs only the git hooks. Never push, never bypass the hooks (`just hooks` installs them).
- Every compile goes through the build guard (`tools/build-guard.py`, an identical copy of Sau's), which keeps the shared Mac cool. Do not bypass it.
- Sau (`~/Projects/Sau`), Null (`~/Projects/Null`) and Baarton (`~/Projects/Baarton`) are separate repositories. Change them only when the task says so.

## Task routes

| Task | Read |
| --- | --- |
| Layout, manifest, packages, templates, commands | `docs/spec/cosmosis-v0.md` |
| What goes inside the files | Null's `docs/spec/null-v0.md` |
| Where this fits in the product; the plan for this repository | Sau's `docs/ROADMAP.md` and `docs/REPOSITORIES.md` |
| The framework Cosmósis is modelled on | Baarton's `docs/spec/baarton-v0.md` |
| Taking and finishing work; commits; reports | Sau's `docs/agents/workflow.md` and `docs/TASKS.md` |
| Worktrees, machine limits, merging | Sau's `docs/agents/parallel.md` |
| What each repository owns; licences | Sau's `docs/agents/boundaries.md` |
