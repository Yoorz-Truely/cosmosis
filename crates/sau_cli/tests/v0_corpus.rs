//! The example corpus for v0 (Sau task 1d-08a): Cosmósis projects in `corpus/slice/` and
//! `corpus/project/` that together write every construct of Null's v0 spec. Each project passes
//! `sau check`, is canonical (`sau fmt --check`), and expands, components and all, to a flat
//! project Null's checker finds clean. Coverage is found by walking the nodes Null's parser
//! reads, from the top of a file down, each step a node's name and the words its line must
//! hold, so a construct counts only where it is written in full; and the test fails when a
//! construct goes missing ([`each_construct_is_load_bearing`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cosmosis::source;
use cosmosis::tree::{Entry, Node, ValueKind};
use null_syntax::{FileId, TokenKind};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

/// Every project of the corpus, slice and beyond.
fn projects() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for part in ["slice", "project"] {
        for e in fs::read_dir(corpus().join(part)).unwrap() {
            let p = e.unwrap().path();
            if p.join("cosmosis.noll").is_file() {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

/// Every `.noll` file under `dir` but the manifest, in path order.
fn noll_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            noll_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "noll") && !p.ends_with("cosmosis.noll") {
            out.push(p);
        }
    }
}

/// What a step's line must hold besides its name.
#[derive(Clone, Copy, Debug)]
enum Want {
    /// This word among its arguments.
    Word(&'static str),
    /// A `key=value` with this key.
    Prop(&'static str),
    /// A handle as its first argument.
    Handle,
    /// No handle: the line's handle is made from its name.
    NoHandle,
    /// No quoted string among its arguments.
    NoName,
    /// No argument and no block: a bare boolean.
    Bare,
    /// No `key=value` with this key.
    NoProp(&'static str),
    /// A list among its arguments.
    List,
    /// A word holding this text (`*`, `.`, `/`, `#`).
    WordWith(&'static str),
    /// A quoted string holding this text.
    StrWith(&'static str),
    /// Some `key=value`.
    AnyProp,
}

fn words(n: &Node) -> impl Iterator<Item = &str> {
    fn walk<'a>(v: &'a cosmosis::tree::Value, out: &mut Vec<&'a str>) {
        match &v.kind {
            ValueKind::Word(w) => out.push(w),
            ValueKind::List(items) => items.iter().for_each(|i| walk(i, out)),
            ValueKind::Str(_) => {}
        }
    }
    let mut out = Vec::new();
    for a in n.arguments() {
        walk(a, &mut out);
    }
    for p in n.properties() {
        walk(&p.value, &mut out);
    }
    out.into_iter()
}

fn holds(n: &Node, want: Want) -> bool {
    let args: Vec<&ValueKind> = n.arguments().map(|a| &a.kind).collect();
    let has_prop = |k: &str| n.properties().any(|p| p.key == k);
    match want {
        Want::Word(w) => args.iter().any(|a| matches!(a, ValueKind::Word(x) if x == w)),
        Want::Prop(k) => has_prop(k),
        Want::NoProp(k) => !has_prop(k),
        Want::Handle => matches!(args.first(), Some(ValueKind::Word(w)) if w.starts_with('@')),
        Want::NoHandle => !args
            .iter()
            .any(|a| matches!(a, ValueKind::Word(w) if w.starts_with('@'))),
        Want::NoName => !args.iter().any(|a| matches!(a, ValueKind::Str(_))),
        Want::Bare => n.entries.is_empty() && n.children.is_none(),
        Want::List => args.iter().any(|a| matches!(a, ValueKind::List(_))),
        Want::WordWith(t) => words(n).any(|w| w.contains(t)),
        Want::StrWith(t) => n
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Argument(v) if matches!(&v.kind, ValueKind::Str(s) if s.contains(t)))),
        Want::AnyProp => n.properties().next().is_some(),
    }
}

/// One step down Null's tree: a node's name, and what its line holds.
type Step = (&'static str, &'static [Want]);

use Want::*;

/// Every construct of Null's v0 spec, by section, with the path of nodes that writes it.
const CONSTRUCTS: &[(&str, &[Step])] = &[
    // Section 8: the project, scenes and collections.
    ("project", &[("project", &[])]),
    ("project units", &[("project", &[]), ("units", &[])]),
    ("scene", &[("scene", &[Handle])]),
    ("scene frame rate", &[("scene", &[]), ("fps", &[])]),
    (
        "scene frame rate base",
        &[("scene", &[]), ("fps-base", &[])],
    ),
    ("scene camera", &[("scene", &[]), ("camera", &[Handle])]),
    (
        "scene compositor",
        &[("scene", &[]), ("compositing-node-group", &[])],
    ),
    (
        "scene's Cycles settings",
        &[("scene", &[]), ("cycles", &[])],
    ),
    (
        "scene member object",
        &[("scene", &[]), ("object", &[Handle])],
    ),
    ("collection", &[("collection", &[Handle])]),
    (
        "collection defined in a scene, holding another",
        &[
            ("scene", &[]),
            ("collection", &[]),
            ("collection", &[]),
            ("object", &[]),
        ],
    ),
    // Section 8: objects.
    ("object", &[("object", &[Handle])]),
    ("object location", &[("object", &[]), ("location", &[List])]),
    (
        "object rotation",
        &[("object", &[]), ("rotation-euler", &[])],
    ),
    ("object scale", &[("object", &[]), ("scale", &[])]),
    ("object parent", &[("object", &[]), ("parent", &[Handle])]),
    (
        "an empty pointer",
        &[("object", &[]), ("parent", &[Word("none")])],
    ),
    (
        "a 4x4 matrix",
        &[("object", &[]), ("matrix-parent-inverse", &[List])],
    ),
    (
        "object data: mesh",
        &[("object", &[]), ("data", &[Word("mesh")])],
    ),
    (
        "object data: camera",
        &[("object", &[]), ("data", &[Word("camera")])],
    ),
    (
        "object data: light",
        &[("object", &[]), ("data", &[Word("light")])],
    ),
    (
        "object data: armature",
        &[("object", &[]), ("data", &[Word("armature")])],
    ),
    ("material slot", &[("object", &[]), ("material", &[Handle])]),
    ("modifier", &[("object", &[]), ("modifier", &[Handle])]),
    (
        "an enum flag set",
        &[("object", &[]), ("modifier", &[]), ("delimit", &[List])],
    ),
    (
        "geometry nodes modifier",
        &[
            ("object", &[]),
            ("modifier", &[Word("nodes")]),
            ("node-group", &[]),
        ],
    ),
    ("constraint", &[("object", &[]), ("constraint", &[Handle])]),
    ("pose bone", &[("object", &[]), ("bone", &[Handle])]),
    (
        "pose bone constraint",
        &[("object", &[]), ("bone", &[]), ("constraint", &[])],
    ),
    (
        "object's Cycles settings",
        &[("object", &[]), ("cycles", &[])],
    ),
    ("raw DNA fields", &[("object", &[]), ("raw", &[])]),
    (
        "raw float bits",
        &[
            ("object", &[]),
            ("raw", &[]),
            ("constinv", &[WordWith("0x")]),
        ],
    ),
    ("camera", &[("camera", &[])]),
    ("camera lens", &[("camera", &[]), ("lens", &[])]),
    ("light", &[("light", &[])]),
    ("point light", &[("light", &[]), ("type", &[Word("point")])]),
    ("sun light", &[("light", &[]), ("type", &[Word("sun")])]),
    ("spot light", &[("light", &[]), ("type", &[Word("spot")])]),
    ("area light", &[("light", &[]), ("type", &[Word("area")])]),
    (
        "a colour as #rrggbb",
        &[("light", &[]), ("color", &[WordWith("#")])],
    ),
    ("infinity", &[("camera", &[]), ("clip-end", &[Word("inf")])]),
    (
        "minus infinity",
        &[
            ("geometry", &[]),
            ("input", &[]),
            ("min-value", &[Word("-inf")]),
        ],
    ),
    (
        "not a number",
        &[
            ("geometry", &[]),
            ("input", &[]),
            ("default-value", &[Word("nan")]),
        ],
    ),
    ("armature", &[("armature", &[])]),
    ("world", &[("world", &[])]),
    (
        "a string as its bytes",
        &[("image", &[]), ("filepath", &[List])],
    ),
    // Section 8: components.
    ("component", &[("component", &[Handle])]),
    ("parameter", &[("component", &[]), ("param", &[])]),
    (
        "parameter typed by name",
        &[("component", &[]), ("param", &[Word("length")])],
    ),
    (
        "parameter of a handle",
        &[("component", &[]), ("param", &[Word("material")])],
    ),
    (
        "parameter from a list",
        &[("component", &[]), ("param", &[]), ("one-of", &[List])],
    ),
    (
        "countdown parameter",
        &[("component", &[]), ("param", &[]), ("countdown", &[])],
    ),
    (
        "condition",
        &[
            ("component", &[]),
            ("object", &[]),
            ("if", &[StrWith("==")]),
        ],
    ),
    (
        "condition's else",
        &[("component", &[]), ("object", &[]), ("else", &[])],
    ),
    (
        "arithmetic in a word",
        &[
            ("component", &[]),
            ("mesh", &[]),
            ("primitive", &[WordWith("*")]),
        ],
    ),
    (
        "arithmetic quoted",
        &[
            ("component", &[]),
            ("if", &[]),
            ("use", &[]),
            ("depth", &[StrWith(" - ")]),
        ],
    ),
    ("placement", &[("collection", &[]), ("use", &[])]),
    (
        "pattern by count",
        &[("collection", &[]), ("use", &[Prop("count")])],
    ),
    (
        "pattern over a list",
        &[("collection", &[]), ("use", &[Prop("each"), Prop("as")])],
    ),
    (
        "pattern's n",
        &[
            ("collection", &[]),
            ("use", &[Prop("count")]),
            ("location", &[WordWith("n*")]),
        ],
    ),
    (
        "placement in a component",
        &[("component", &[]), ("use", &[])],
    ),
    (
        "recursion under a condition",
        &[("component", &[]), ("if", &[]), ("use", &[])],
    ),
    (
        "override of a copy",
        &[("collection", &[]), ("override", &[WordWith(".")])],
    ),
    (
        "override of a copy's datablock",
        &[
            ("collection", &[]),
            ("override", &[WordWith("/@post-mesh")]),
        ],
    ),
    (
        "override of a nested copy",
        &[("collection", &[]), ("override", &[WordWith("/@left/")])],
    ),
    (
        "override of a sub-object's sub-object",
        &[("collection", &[]), ("override", &[WordWith("/@spine/")])],
    ),
    (
        "override of a copy's modifier",
        &[("collection", &[]), ("override", &[]), ("modifier", &[])],
    ),
    // Section 9: level 0.
    ("mesh as a primitive", &[("mesh", &[]), ("primitive", &[])]),
    ("inline vertices", &[("mesh", &[]), ("vertices", &[List])]),
    ("inline polygons", &[("mesh", &[]), ("polygons", &[List])]),
    ("inline edges", &[("mesh", &[]), ("edges", &[List])]),
    ("inline attribute", &[("mesh", &[]), ("attribute", &[List])]),
    ("UV map", &[("mesh", &[]), ("uv-map", &[List])]),
    (
        "vertex group",
        &[("mesh", &[]), ("vertex-group", &[]), ("weights", &[List])],
    ),
    (
        "vertices in a data file",
        &[
            ("mesh", &[]),
            ("vertices", &[Prop("file"), Prop("count"), Prop("bounds")]),
        ],
    ),
    (
        "polygons in a data file",
        &[("mesh", &[]), ("polygons", &[Prop("file"), Prop("count")])],
    ),
    (
        "attribute in a data file",
        &[("mesh", &[]), ("attribute", &[Prop("file")])],
    ),
    ("data file", &[("array", &[Word("mesh"), List])]),
    // Section 10: node trees.
    ("material", &[("material", &[])]),
    (
        "a material's nodes",
        &[("material", &[]), ("nodes", &[]), ("node", &[Handle])],
    ),
    (
        "Principled BSDF",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[Word("bsdf-principled")]),
        ],
    ),
    (
        "base colour",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[]),
            ("base-color", &[]),
        ],
    ),
    (
        "roughness",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[]),
            ("roughness", &[]),
        ],
    ),
    (
        "a link",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[]),
            ("surface", &[WordWith(".")]),
        ],
    ),
    (
        "frame",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[Word("frame")]),
        ],
    ),
    (
        "a node in a frame",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[]),
            ("parent", &[]),
        ],
    ),
    ("layout", &[("material", &[]), ("layout", &[])]),
    ("geometry group", &[("geometry", &[Handle])]),
    ("group input", &[("geometry", &[]), ("input", &[Handle])]),
    ("group output", &[("geometry", &[]), ("output", &[Handle])]),
    (
        "interface panel",
        &[("geometry", &[]), ("panel", &[]), ("input", &[])],
    ),
    (
        "group node",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("node", &[Word("group")]),
            ("node-tree", &[]),
        ],
    ),
    (
        "a list of links",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("zone", &[]),
            ("node", &[]),
            ("geometry", &[List]),
        ],
    ),
    (
        "repeat zone",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("zone", &[Word("repeat")]),
            ("repeat-item", &[]),
        ],
    ),
    (
        "simulation zone",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("zone", &[Word("simulation")]),
            ("state-item", &[]),
        ],
    ),
    (
        "for-each zone",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("zone", &[Word("foreach-geometry-element")]),
            ("generation-item", &[]),
        ],
    ),
    (
        "a zone's output",
        &[
            ("geometry", &[]),
            ("nodes", &[]),
            ("zone", &[]),
            ("output", &[]),
        ],
    ),
    ("a zone's layout", &[("geometry", &[]), ("layout", &[])]),
    ("compositor group", &[("compositor", &[]), ("nodes", &[])]),
    // Section 11: animation and drivers.
    ("action", &[("action", &[])]),
    ("slot", &[("action", &[]), ("slot", &[Handle])]),
    (
        "f-curve in a slot (short form)",
        &[("action", &[]), ("slot", &[]), ("fcurve", &[])],
    ),
    (
        "key",
        &[
            ("action", &[]),
            ("slot", &[]),
            ("fcurve", &[]),
            ("key", &[]),
        ],
    ),
    (
        "key interpolation",
        &[
            ("action", &[]),
            ("slot", &[]),
            ("fcurve", &[]),
            ("key", &[Prop("interpolation")]),
        ],
    ),
    (
        "f-curve modifier",
        &[
            ("action", &[]),
            ("slot", &[]),
            ("fcurve", &[]),
            ("modifier", &[]),
        ],
    ),
    (
        "layer, strip and channels (full form)",
        &[
            ("action", &[]),
            ("layer", &[]),
            ("strip", &[Word("keyframe")]),
            ("channels", &[]),
            ("fcurve", &[Prop("index")]),
        ],
    ),
    (
        "key handles",
        &[
            ("action", &[]),
            ("layer", &[]),
            ("strip", &[]),
            ("channels", &[]),
            ("fcurve", &[]),
            ("key", &[Prop("handle-left"), Prop("handle-right")]),
        ],
    ),
    (
        "f-curve setting",
        &[
            ("action", &[]),
            ("layer", &[]),
            ("strip", &[]),
            ("channels", &[]),
            ("fcurve", &[]),
            ("extrapolation", &[]),
        ],
    ),
    (
        "a datablock playing an action",
        &[("light", &[]), ("action", &[Prop("slot")])],
    ),
    (
        "NLA track",
        &[
            ("object", &[]),
            ("track", &[]),
            ("strip", &[]),
            ("action", &[]),
        ],
    ),
    (
        "driver",
        &[
            ("light", &[]),
            ("energy", &[Prop("driver")]),
            ("var", &[]),
            ("target", &[]),
        ],
    ),
    (
        "driver variable: transforms",
        &[
            ("light", &[]),
            ("energy", &[]),
            ("var", &[Word("transforms")]),
        ],
    ),
    (
        "driver variable: single property",
        &[
            ("light", &[]),
            ("energy", &[]),
            ("var", &[Word("single-prop")]),
            ("target", &[Handle]),
        ],
    ),
    (
        "driver variable: location difference",
        &[
            ("light", &[]),
            ("energy", &[]),
            ("var", &[Word("loc-diff")]),
            ("target", &[]),
        ],
    ),
    (
        "driver variable: rotation difference",
        &[
            ("light", &[]),
            ("energy", &[]),
            ("var", &[Word("rotation-diff")]),
            ("target", &[]),
        ],
    ),
    (
        "driver variable: context property",
        &[
            ("light", &[]),
            ("energy", &[]),
            ("var", &[Word("context-prop")]),
            ("target", &[Word("active-scene")]),
        ],
    ),
    // Section 17: short forms.
    (
        "short form: a handle made from the name",
        &[("object", &[NoHandle])],
    ),
    (
        "short form: a quoted name left out",
        &[("object", &[]), ("modifier", &[NoName])],
    ),
    (
        "short form: settings on the line",
        &[("object", &[]), ("modifier", &[AnyProp])],
    ),
    (
        "short form: a bare boolean",
        &[("object", &[]), ("hide-render", &[Bare])],
    ),
    (
        "short form: units on one line",
        &[("scene", &[]), ("units", &[Prop("scale")])],
    ),
    (
        "units in full",
        &[("scene", &[]), ("units", &[]), ("scale-length", &[])],
    ),
    (
        "short form: slot= left out",
        &[("object", &[]), ("action", &[NoProp("slot")])],
    ),
    (
        "short form: a path's owner left out",
        &[
            ("material", &[]),
            ("nodes", &[]),
            ("node", &[]),
            ("parent", &[WordWith("@")]),
        ],
    ),
];

/// Section 5's syntax, which no node shows: each found in a file's tokens (or, for a keyword
/// attribute, its entries), with how to write a file without it, for the load-bearing test.
#[allow(clippy::type_complexity)]
const SYNTAX: &[(&str, fn(&str) -> bool, fn(&str) -> String)] = &[
    ("a keyword attribute (`to @x`)", has_keyword, |t| {
        t.replace(" to ", " tx ")
            .replace(" at ", " ax ")
            .replace(" len ", " lx ")
    }),
    (
        "a raw string",
        |t| has_token(t, |k, _| k == TokenKind::RawString),
        |t| t.replace("#\"", "\"").replace("\"#", "\""),
    ),
    (
        "a nested block comment",
        |t| {
            has_token(t, |k, s| {
                k == TokenKind::BlockComment && s.matches("/*").count() > 1
            })
        },
        |t| without_tokens(t, TokenKind::BlockComment, " "),
    ),
    (
        "a node commented out with `/-`",
        |t| has_token(t, |k, _| k == TokenKind::Slashdash),
        |t| t.replace("/-", "//"),
    ),
    (
        "a line continued with a backslash",
        |t| has_token(t, |k, _| k == TokenKind::Continuation),
        |t| t.replace("\\\n", " "),
    ),
];

/// Whether some token of `text` is one `pick` takes, given its kind and text.
fn has_token(text: &str, pick: impl Fn(TokenKind, &str) -> bool) -> bool {
    let mut at = 0;
    null_syntax::lex(text).0.iter().any(|t| {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        pick(t.kind, s)
    })
}

/// `text` with every token of `kind` written as `with`.
fn without_tokens(text: &str, kind: TokenKind, with: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    for t in null_syntax::lex(text).0 {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        out.push_str(if t.kind == kind { with } else { s });
    }
    out
}

/// Whether some node of `text` has a keyword attribute (`to @x`), as Scowr's parser reads it.
fn has_keyword(text: &str) -> bool {
    fn walk(n: null_syntax::ast::Node<'_>) -> bool {
        n.entries()
            .any(|e| matches!(e, null_syntax::ast::Entry::Keyword { .. }))
            || n.children().any(walk)
    }
    let parse = null_syntax::parse(text);
    parse.document().nodes().any(walk)
}

/// A file's live top-level nodes, read by Null's parser; the file must parse cleanly.
fn parse(text: &str, path: &Path) -> Vec<Node> {
    let parsed = source::parse(text, path);
    assert!(
        parsed.is_clean(),
        "{}: {:?}",
        path.display(),
        parsed.diagnostics
    );
    parsed.nodes
}

fn has(nodes: &[Node], path: &[Step]) -> bool {
    let Some(((name, wants), rest)) = path.split_first() else {
        return true;
    };
    nodes.iter().any(|n| {
        n.name == *name
            && wants.iter().all(|w| holds(n, *w))
            && (rest.is_empty() || has(n.children.as_deref().unwrap_or_default(), rest))
    })
}

fn missing(files: &[Vec<Node>]) -> Vec<&'static str> {
    CONSTRUCTS
        .iter()
        .filter(|(_, path)| !files.iter().any(|nodes| has(nodes, path)))
        .map(|(name, _)| *name)
        .collect()
}

fn corpus_texts() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    for p in projects() {
        noll_files(&p, &mut files);
    }
    files
        .into_iter()
        .map(|f| {
            let t = fs::read_to_string(&f).unwrap();
            (f, t)
        })
        .collect()
}

#[test]
fn every_v0_construct_appears_in_the_corpus() {
    let parsed: Vec<Vec<Node>> = corpus_texts().iter().map(|(f, t)| parse(t, f)).collect();
    let missing = missing(&parsed);
    assert!(missing.is_empty(), "the corpus never writes: {missing:?}");
}

#[test]
fn section_5_s_syntax_appears_in_the_corpus_and_each_is_load_bearing() {
    let texts = corpus_texts();
    for (name, found, without) in SYNTAX {
        assert!(
            texts.iter().any(|(_, t)| found(t)),
            "the corpus never writes {name}"
        );
        // Written without it, every file still parses, and it is found nowhere.
        for (f, t) in &texts {
            let changed = without(t);
            parse(&changed, f);
            assert!(!found(&changed), "{name}: still found in {}", f.display());
        }
    }
}

#[test]
fn each_construct_is_load_bearing() {
    // Each construct's last step, renamed wherever the corpus writes it, makes the construct
    // missing: the test fails when a construct goes, as the row asks.
    let texts = corpus_texts();
    for (name, path) in CONSTRUCTS {
        let (last, _) = path.last().unwrap();
        let renamed: Vec<Vec<Node>> = texts
            .iter()
            .map(|(f, t)| {
                let mut nodes = parse(t, f);
                rename(&mut nodes, path, 0, last);
                nodes
            })
            .collect();
        assert!(
            missing(&renamed).contains(name),
            "{name}: still found with its `{last}` lines renamed"
        );
    }
}

/// Renames the nodes at the path's last step, wherever the path leads to them.
fn rename(nodes: &mut [Node], path: &[Step], at: usize, last: &str) {
    for n in nodes {
        if at + 1 == path.len() && n.name == last {
            n.name = format!("zz-{last}");
        }
        if let Some(c) = n.children.as_mut() {
            rename(c, path, at + 1, last);
        }
    }
}

#[test]
fn every_project_checks_formats_and_expands_clean() {
    let projects = projects();
    assert!(projects.len() >= 12, "{projects:?}");
    for p in &projects {
        let check = Command::new(env!("CARGO_BIN_EXE_sau"))
            .arg("check")
            .arg(p)
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "{}: {}",
            p.display(),
            String::from_utf8_lossy(&check.stderr)
        );
        let fmt = Command::new(env!("CARGO_BIN_EXE_sau"))
            .args(["fmt", "--check"])
            .arg(p)
            .output()
            .unwrap();
        assert!(
            fmt.status.success(),
            "{}: {}",
            p.display(),
            String::from_utf8_lossy(&fmt.stdout)
        );
        // Components expanded: the flat project checks clean too (Null spec, section 8).
        let mut files = Vec::new();
        noll_files(p, &mut files);
        let files: Vec<PathBuf> = files
            .into_iter()
            .filter(|f| !f.ends_with(".sau/ids.noll"))
            .collect();
        let texts: Vec<String> = files
            .iter()
            .map(|f| fs::read_to_string(f).unwrap())
            .collect();
        let parses: Vec<null_syntax::Parse> = texts.iter().map(|t| null_syntax::parse(t)).collect();
        let ids: Vec<(FileId, &null_syntax::Parse)> = parses
            .iter()
            .enumerate()
            .map(|(i, p)| (FileId(i as u32), p))
            .collect();
        let schema = cosmosis::schema_check::schema();
        let (flat, problems) = null_check::expand(schema, &ids);
        assert!(problems.is_empty(), "{}: {problems:?}", p.display());
        let found = null_check::check_expanded(schema, &flat);
        // Notes stand (a value outside its range, `inf` and `nan` among them); errors and
        // warnings do not.
        let found: Vec<String> = found
            .iter()
            .filter(|d| d.severity != null_syntax::Severity::Note)
            .map(|d| {
                format!(
                    "{}: {}",
                    files[d.primary.file.0 as usize].display(),
                    d.message
                )
            })
            .collect();
        assert!(found.is_empty(), "{}: {found:#?}", p.display());
    }
}
