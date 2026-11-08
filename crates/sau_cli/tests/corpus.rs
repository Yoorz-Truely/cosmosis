//! The slice corpus in `corpus/slice/` (Sau tasks C-03 and 1a-15): at least five projects, each
//! passing `sau check` as a whole (manifest, layout, Null's parser and Null's checker against
//! Blender's schema), and together writing every construct of phase 1a's slice, found by walking
//! the nodes Null's parser reads, from the datablock down.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cosmosis::source;
use cosmosis::tree::{Node, ValueKind};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice")
}

fn projects() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(corpus())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("cosmosis.noll").is_file())
        .collect();
    v.sort();
    v
}

/// Every `.noll` file under `dir`.
fn noll_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            noll_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "noll") {
            out.push(p);
        }
    }
}

/// One step down Null's tree: a node's name, and a word that must be among its arguments.
type Step = (&'static str, Option<&'static str>);

/// The slice's constructs (Sau's board, phase 1a): a name, and the path of nodes from the top
/// level of a file that writes it. A path starts at a datablock, so a scene's `camera @cam`
/// line or an object's `material @clay` slot is not a camera or a material.
const CONSTRUCTS: [(&str, &[Step]); 19] = [
    ("scene", &[("scene", None)]),
    ("object", &[("object", None)]),
    ("object location", &[("object", None), ("location", None)]),
    (
        "object rotation",
        &[("object", None), ("rotation-euler", None)],
    ),
    ("object scale", &[("object", None), ("scale", None)]),
    ("object parent", &[("object", None), ("parent", None)]),
    (
        "object data: mesh",
        &[("object", None), ("data", Some("mesh"))],
    ),
    (
        "object data: camera",
        &[("object", None), ("data", Some("camera"))],
    ),
    (
        "object data: light",
        &[("object", None), ("data", Some("light"))],
    ),
    ("mesh", &[("mesh", None)]),
    (
        "mesh as a primitive",
        &[("mesh", None), ("primitive", None)],
    ),
    (
        "mesh with inline vertices",
        &[("mesh", None), ("vertices", None)],
    ),
    (
        "mesh with inline polygons",
        &[("mesh", None), ("polygons", None)],
    ),
    ("material", &[("material", None)]),
    (
        "Principled BSDF",
        &[
            ("material", None),
            ("nodes", None),
            ("node", Some("bsdf-principled")),
        ],
    ),
    (
        "base colour",
        &[
            ("material", None),
            ("nodes", None),
            ("node", Some("bsdf-principled")),
            ("base-color", None),
        ],
    ),
    (
        "roughness",
        &[
            ("material", None),
            ("nodes", None),
            ("node", Some("bsdf-principled")),
            ("roughness", None),
        ],
    ),
    ("camera", &[("camera", None)]),
    ("light", &[("light", None)]),
];

/// The live top-level nodes of a file, read by Null's parser; the file must parse cleanly.
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

/// True when some node in `nodes` matches the path's first step and, below it, the rest.
fn has(nodes: &[Node], path: &[Step]) -> bool {
    let Some(((name, word), rest)) = path.split_first() else {
        return true;
    };
    nodes.iter().any(|n| {
        n.name == *name
            && word.is_none_or(|w| {
                n.arguments()
                    .any(|a| matches!(&a.kind, ValueKind::Word(x) if x == w))
            })
            && (rest.is_empty() || has(n.children.as_deref().unwrap_or_default(), rest))
    })
}

/// The constructs that no file in `files` writes.
fn missing(files: &[Vec<Node>]) -> Vec<&'static str> {
    CONSTRUCTS
        .iter()
        .filter(|(_, path)| !files.iter().any(|nodes| has(nodes, path)))
        .map(|(name, _)| *name)
        .collect()
}

#[test]
fn there_are_at_least_five_projects_and_each_passes_sau_check() {
    let projects = projects();
    assert!(projects.len() >= 5, "{projects:?}");
    for p in projects {
        let out = Command::new(env!("CARGO_BIN_EXE_sau"))
            .arg("check")
            .arg(&p)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}:\n{}",
            p.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn every_slice_construct_appears_in_the_corpus() {
    let mut files = Vec::new();
    for p in projects() {
        noll_files(&p, &mut files);
    }
    let parsed: Vec<Vec<Node>> = files
        .iter()
        .map(|f| parse(&fs::read_to_string(f).unwrap(), f))
        .collect();
    let missing = missing(&parsed);
    assert!(missing.is_empty(), "the corpus never writes: {missing:?}");
}

/// A project of every construct, in two files, for the self-tests below to break.
const WHOLE: [&str; 2] = [
    "scene @main \"Main\" {\n  camera @cam\n  object @cube\n  object @cam\n  object @sun\n}\n",
    "object @cube {\n  location [0m 0m 1m]\n  rotation-euler [0deg 0deg 45deg]\n  scale [1 1 1]\n  \
     parent @cam\n  data mesh @box\n  material @clay\n}\n\
     object @cam {\n  data camera @lens\n}\n\
     object @sun {\n  data light @sunlight\n}\n\
     mesh @box {\n  primitive cube size=2m\n}\n\
     mesh @tri {\n  vertices [[0m 0m 0m] [1m 0m 0m] [0m 1m 0m]]\n  polygons [[0 1 2]]\n}\n\
     material @clay {\n  nodes {\n    node @bsdf bsdf-principled {\n      base-color [0.8 0.3 0.2 1]\n      \
     roughness 0.7\n    }\n  }\n}\n\
     camera @lens {\n  lens 50mm\n}\n\
     light @sunlight {\n  type sun\n}\n",
];

fn whole_with(change: impl Fn(&str) -> String) -> Vec<Vec<Node>> {
    WHOLE
        .iter()
        .map(|text| parse(&change(text), Path::new("test.noll")))
        .collect()
}

#[test]
fn the_coverage_test_finds_every_construct_in_a_whole_project() {
    assert_eq!(missing(&whole_with(str::to_owned)), Vec::<&str>::new());
}

#[test]
fn a_camera_line_in_a_scene_is_not_a_camera_datablock() {
    // The scene still says `camera @cam` and the object `data camera @lens`; only the datablock goes.
    let files = whole_with(|t| t.replace("camera @lens {\n  lens 50mm\n}\n", ""));
    assert_eq!(missing(&files), ["camera"]);
}

#[test]
fn a_material_slot_on_an_object_is_not_a_material_datablock() {
    let files = whole_with(|t| {
        let start = t.find("material @clay {").unwrap_or(t.len());
        let end = t.find("camera @lens {").unwrap_or(t.len());
        format!("{}{}", &t[..start], &t[end..])
    });
    assert_eq!(
        missing(&files),
        ["material", "Principled BSDF", "base colour", "roughness"]
    );
}

#[test]
fn a_construct_commented_out_does_not_count() {
    let files =
        whole_with(|t| t.replace("  parent @cam\n", "  /- parent @cam\n  // parent @cam\n"));
    assert_eq!(missing(&files), ["object parent"]);
}
