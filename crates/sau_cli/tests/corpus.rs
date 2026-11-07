//! The slice corpus in `corpus/slice/` (Sau tasks C-03 and 1a-15): at least five projects, each
//! passing `sau check` as a whole (manifest, layout, Null's parser and Null's checker against
//! Blender's schema), and together writing every construct of phase 1a's slice, found by name
//! in the files' text with comments removed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Each line's words, with `//` comments removed (the corpus has no `//` inside strings).
fn lines(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(|l| {
            l.split("//")
                .next()
                .unwrap()
                .split_whitespace()
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

/// The slice's constructs (Sau's board, phase 1a): a name, the words that write it, and
/// whether they start the line (a node) or may appear anywhere on it.
const CONSTRUCTS: [(&str, &[&str], bool); 19] = [
    ("scene", &["scene"], true),
    ("object", &["object"], true),
    ("object location", &["location"], true),
    ("object rotation", &["rotation-euler"], true),
    ("object scale", &["scale"], true),
    ("object parent", &["parent"], true),
    ("object data: mesh", &["data", "mesh"], true),
    ("object data: camera", &["data", "camera"], true),
    ("object data: light", &["data", "light"], true),
    ("mesh", &["mesh"], true),
    ("mesh as a primitive", &["primitive"], true),
    ("mesh with inline vertices", &["vertices"], true),
    ("mesh with inline polygons", &["polygons"], true),
    ("material", &["material"], true),
    ("Principled BSDF", &["principled-bsdf"], false),
    ("base colour", &["base-color"], true),
    ("roughness", &["roughness"], true),
    ("camera", &["camera"], true),
    ("light", &["light"], true),
];

fn has(words: &[String], needle: &[&str], at_start: bool) -> bool {
    if at_start {
        words.len() >= needle.len() && words.iter().zip(needle).all(|(w, n)| w == n)
    } else {
        words
            .windows(needle.len())
            .any(|win| win.iter().zip(needle).all(|(w, n)| w == n))
    }
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
    let mut all: Vec<Vec<String>> = Vec::new();
    let mut files = Vec::new();
    for p in projects() {
        noll_files(&p, &mut files);
    }
    for f in &files {
        all.extend(lines(&fs::read_to_string(f).unwrap()));
    }
    let missing: Vec<&str> = CONSTRUCTS
        .iter()
        .filter(|(_, needle, at_start)| !all.iter().any(|l| has(l, needle, *at_start)))
        .map(|(name, _, _)| *name)
        .collect();
    assert!(missing.is_empty(), "the corpus never writes: {missing:?}");
}

#[test]
fn the_coverage_test_notices_a_missing_construct() {
    let without_parent = lines("object @a {\n  location [0m 0m 0m] // parent @b\n}\n");
    assert!(!without_parent.iter().any(|l| has(l, &["parent"], true)));
    let with_parent = lines("object @a {\n  parent @b\n}\n");
    assert!(with_parent.iter().any(|l| has(l, &["parent"], true)));
}
