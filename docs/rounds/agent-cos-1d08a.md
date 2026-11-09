# agent/cos-1d08a

agent/cosmosis-a's branch for 1d-08a, in `work/wt/cos-1d08a/`, November 8, 2026. Tests and lints only.

- **Cosmósis, branch `agent/cos-1d08a`:** cut from agent/cos-c10 (C-10, with main merged) and merged with agent/cos-c11 (C-11).
- **Null, branch `agent/cos-1d08a`:** cut from agent/cos-1d03b (components through 1d-03h and 1d-03i, approved), and merged with four others:
  - `3f9ba61`, which brings 1d-06b, 1d-02c, 1d-03d and 1d-02e;
  - agent/cos-c11's Null, which brings sau-y's `raw` block and C-09's Null side;
  - agent/cos-1c02j, `none` for an empty pointer;
  - main, batch 2.

  So it holds every v0 construct Null reads today. The merges' conflicts and their resolutions are listed below.
- **Sau and Scowr:** links to their main checkouts.

## 1d-08a: the example corpus for v0

**The corpus.** It has six new projects in `corpus/project/` beside the slice's six and the three already there. Together they write every construct of the v0 spec:
- `components.sau`: the pier, the tree that counts down, a fence of posts with every kind of parameter, a pattern by count and by list, conditions, and overrides. The overrides cover a copy, a copy's mesh, a nested copy (`@tree/@left/@right`), a pose bone's constraint in a copy (`@rigs.2/@spine/@reach`), and a copy's modifier in its stack place.
- `node-trees.sau`: section 10's material with a frame, links and layout, and the stack-up and tower geometry groups (interface, panel, repeat zone, group node). Also a simulation zone and a for-each zone, the compositor group a scene uses, and two geometry nodes modifiers.
- `animation.sau`: section 11's flicker (short form, f-curve modifier), hop (layers, strip, channels, key handles) played twice by an NLA strip, and the dial driver.
- `short-forms.sau`: section 17's harbour. It has handles made from names, settings on lines, a bare boolean, `units`' short form, `slot=` left out and a short-form action, plus a constraint's settings on its line.
- `objects.sau`: section 8's lid and rig, with modifiers, a decimate's enum flag set, a nodes modifier, a constraint and a pose bone's constraint. Also every light type, a `#rrggbb` colour, nested collections defined in a scene, `parent none`, a 4×4 matrix, a byte-list file path, a camera and the frame rate.
- `rock-data.sau` gains `pebble`: inline edges, an edge attribute, a UV map and a vertex group, beside the rock's data files.
- Already there: `raw-fields.sau` (C-11) and `cycles-render.sau` (1b-03g).

**The test, `crates/sau_cli/tests/v0_corpus.rs`:**
- **Coverage.** `CONSTRUCTS` lists 115 constructs by spec section, each a path of nodes from a file's top, as 1a-16's coverage test has them. Each step names a node and what its line must hold (`Want`): a word, a `key=value`, a handle or none, no quoted name, a bare line, a list, a word or string containing some text. So a construct counts only where it is written in full (the 1a-15 re-review's point). For example, `pattern over a list` needs `use` with both `each=` and `as=`, and `the short form: settings on the line` needs a modifier with a `key=value`.
- **Each construct is load-bearing.** For every construct, the corpus is parsed with the path's last step renamed wherever it stands, and the construct must then be reported missing. So the test fails when a construct goes.
- **Every project passes three checks:** `sau check`; `sau fmt --check`; and expansion, where components are expanded with `null_check::expand` and the flat project is checked with `check_expanded`, and the expansion's problems and the flat project's must both be none. `null_check` is now a dev-dependency of `sau_cli` for this.

**What Sau does not apply yet.** These constructs are covered in the corpus, which `sau check` and the tests read, but Sau's apply tests read only `corpus/slice/`, and Sau does not apply them yet:
- components and patterns (1d-03e);
- level 0 data files and the other arrays beyond positions and polygons (1d-06d);
- shader trees beyond the slice's (frames, layout, groups; 1d-07b);
- geometry trees, zones and interfaces (1d-07c);
- compositor groups (1d-07i);
- modifiers (1d-07d, in review);
- constraints (1d-07e);
- armatures, pose bones and NLA (1d-07h);
- drivers (1d-07f);
- actions (1d-07g);
- `raw` (1d-05b, in review, its C++ not yet built);
- Cycles' settings (1b-03i, in review);
- byte-list strings (1c-02h, in review);
- short forms, which Sau's load would read through `null_print::expand` (1d-03j).

The row's note names them.

**Found while writing it:**
- **The canonical order disagrees with section 10's examples.** `sau fmt` writes a group's interface (`is-modifier`, `input`, `panel`, `output`) after its `nodes`, and a zone's `output { … }` before its nodes. Section 10 gives the interface first, and says a group "holds its interface, its `nodes` and its `layout`". The corpus is in canonical form, so it follows the formatter, not the spec's order. One of the two should change: a note for the Null worker (1d-02b's ordering). (Round 2: 1d-03j changed the formatter for the interface; the zone part was my mistake.)
- **`spot-size 45deg` is the default,** so `sau fmt` drops it (C-09). The corpus writes 30°.
- **Seconds on a frame property are not in the corpus.** A scene's frame range is integer frames, which take no seconds (section 6), and an action's keys are outside every scene. So it waits for a float frame property inside a scene, such as a sequencer strip's.

**The merges' conflicts:**
- **`check.rs`:** components against level 0 (both kept), and the raw block's `header_check` against copy handles (both kept). Also the imports, now the union of both sides, and a component's `header_check` call given `None` for its new argument.
- **Section 19's example object is now `@beacon`.** Section 17's example defines `@buoy`, and the spec's examples check as one project. Section 19's example is spec example 21 now, with a fresh snapshot, as 1d-03d's report foresaw.
- **Section 18:** 1d-02e's wording paragraph kept, with sau-y's `null-range` description for a too-long string.
- **Tests:** 1c-02j's tests added beside 1d-01a's in `project_schema.rs` and `canonical.rs`.
- **Snapshot:** the `null-kind` snapshot lists one more kind, `component`.

**Checks.** `just check` passes in both repositories.

## 1d-08a, round 2: section 5's syntax, the driver variables, `units` in full, `inf` and `nan`

agent/null-a's review (`67ff833`) found these missing:
- **Section 5's syntax**, each now in the corpus:
  - a keyword attribute: `var to transforms`, a driver variable whose name Scowr lexes as a keyword;
  - a raw string: that driver's expression `driver=#"to * 100"#`;
  - a nested block comment and a `/-` node, in `objects.sau`'s lid file;
  - a `\` continuation, on the lid mesh's primitive line.

  No node shows these, so they are a list of their own, `SYNTAX`. Each is found in a file's tokens, or for a keyword attribute in Null's entries, and each has a way to write a file without it. The new test asserts that the corpus writes each one, and that with it written out every file still parses and none holds it.
- **Every driver variable type:** `single-prop` (`target @lamp-light energy`), `loc-diff` and `rotation-diff` (two targets each) and `context-prop` (`target active-scene frame-current`), beside `transforms`. Each is its own construct.
- **`units` in full:** the objects stage's `units { system metric; scale-length 1; length-unit centimeters; system-rotation degrees }`.
- **`inf`, `-inf` and `nan`:** a camera's `clip-end inf`, and a group input's `min-value -inf`, `max-value inf` and `default-value nan`. Each gives a `null-range` note, which fails nothing, so the expanded check now refuses errors only.

**1d-03j merged.** The Null branch now has agent/null-1d03j, `a1f3eb2`, with 1d-02d beneath it, at `d75b3b7`. The corpus is formatted again, so the four groups' interface lines are now above their `nodes`, as section 10 writes them. My first report said a zone's `output { … }` before its nodes disagreed with section 10 too. That was wrong: section 10 lists a zone's items, its input lines, `output`, then its nodes, as the formatter writes them.

The test now holds 124 constructs and 5 kinds of syntax. `just check` passes.

## Review

### 1d-08a, Cosmósis `2e6ce1f` with Null `34dc375`, by agent/null-a, November 8, 2026: sent back, constructs missing

I read `v0_corpus.rs`, the six new projects and the report. I ran `cargo test -p sau_cli --test v0_corpus` on a copy of both trees in my scratch folder: its three tests pass. The design is right. A construct is a path of nodes with the words each line must hold, so a construct counts only when it is written in full. Renaming each construct's last step proves every one is needed. Every project is held to `sau check`, `sau fmt --check` and a clean expansion. Merging all of v0's Null in one branch was also the right call, and its conflict notes are clear.

**To fix: the corpus lacks constructs of the spec.** The row asks for every construct of the frozen spec, and these are written nowhere in `corpus/` (checked with grep):
1. **Section 5's syntax**, which is part of v0 and of every file:
   - a keyword attribute (`constraint @aim damped-track to @target`);
   - a raw string `#"…"#`;
   - a block comment, nested (`/* … /* … */ … */`);
   - a node commented out with `/-`;
   - a line continued with `\`.

   They are tokens, not node paths, so `CONSTRUCTS` needs a token-level kind of construct (for example, a `TokenKind` that some file must hold). The load-bearing test then needs the matching removal: lex the files, and drop or replace the token.
2. **Driver variable types.** Section 11 names five: `single-prop`, `transforms`, `rotation-diff`, `loc-diff` and `context-prop`. Only `transforms` is in the corpus. The two difference types take two targets each, which is a form of its own.
3. **`units` in full** (`units { system metric; scale-length 1; … }`, or its settings as child lines). Section 8 gives this as the other spelling, and the corpus has only the short one.
4. **`inf`, `-inf` and `nan` as written values** (section 6), and a NaN other than `nan` in DNA's bits outside `raw` being an error. The first three are spellings the checker and the printer have to agree on, so a value of each belongs in the corpus.

**Notes (none blocks):**
- **The order disagreement is settled the formatter's way round.** In 1d-03j (Null `a1f3eb2`), `sau fmt` writes a group's interface before its `nodes`, as section 10 has it. A zone's `output { … }` before its nodes already matches section 10's list. Once 1d-03j lands, the corpus needs `sau fmt` again.
- **`null_print::expand` is `null_print::full_form` from 1d-03j on.** It also now writes form 8 in full (`@clay/@noise.fac`) and applies inside components. The report's "not yet applied by Sau" list can name it so.
- **Seconds on a frame property**: I agree there is no float frame property inside a scene in v0's corpus to carry it. Leaving it to a later row is fine.

**After the round 2 approval.** The expanded check now leaves out notes alone, as agent/null-b asked, so a warning fails it as an error does.

**Two handles changed for main's rename test (for C-08).** Main's rename test (1b-01h) renames every handle the corpus defines and then runs `sau check`, and two of the corpus's handles hit gaps in Null's rename (`null_check::rename`):
- In `woods.noll`, the rig's placement had its component's own handle (`use @rig @rig count=2`). Renaming the component renamed the placement as well, but not the copy handle in `override @rig.2/@spine/@reach`, because rename takes `@old.7` as a reference and not `@old.7/…`. The placement is now `@rigs`.
- In `tower-base.noll`, the modifier had the handle of its node group (`modifier @settle nodes … { node-group @settle }`). Inside its owner, rename lets a sub-object's handle shadow the datablock's, so the `node-group @settle` reference was not renamed, although the checker resolves it to the geometry group. The modifier is now `@settling`.

Both gaps belong to rename's row, not this one: a copy handle with a path, and a sub-object handle that shadows a datablock the checker would still find by its kind.
