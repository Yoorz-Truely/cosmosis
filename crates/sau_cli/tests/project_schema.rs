//! `sau check` reads the schema of all project data (Sau task C-06): the corpus still checks
//! clean, and a datablock type the slice's schema never knew is checked, its misspellings
//! corrected.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sau(root: &Path) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sau"))
        .args(["check", root.to_str().unwrap()])
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice");
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("cosmosis.noll").is_file())
        .collect();
    v.sort();
    v
}

/// A copy of one-cube with `extra` written as `world.noll` beside its files.
fn with_file(name: &str, extra: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("project-schema-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice/one-cube.sau");
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        fs::create_dir_all(root.join(&rel)).unwrap();
        for entry in fs::read_dir(from.join(&rel)).unwrap() {
            let entry = entry.unwrap();
            let next = rel.join(entry.file_name());
            if entry.path().is_dir() {
                stack.push(next);
            } else {
                fs::copy(entry.path(), root.join(&next)).unwrap();
            }
        }
    }
    fs::write(root.join("world.noll"), extra).unwrap();
    root
}

#[test]
fn the_corpus_checks_clean_on_the_project_schema() {
    let projects = corpus();
    assert!(projects.len() >= 5);
    for p in projects {
        let (code, out, err) = sau(&p);
        assert_eq!(code, Some(0), "{}: {err}", p.display());
        assert!(out.contains("Null files check"), "{out}");
    }
}

#[test]
fn a_datablock_type_the_slice_never_knew_is_checked() {
    // A world: the slice's schema refused the kind itself (`null-kind`); the project's checks
    // its properties and corrects a misspelling.
    let clean = with_file(
        "world-clean",
        "world @sky \"Sky\" {\n  color [0.1 0.1 0.1]\n}\n",
    );
    let (code, _, err) = sau(&clean);
    assert_eq!(code, Some(0), "{err}");
    let misspelt = with_file(
        "world-misspelt",
        "world @sky \"Sky\" {\n  colr [0.1 0.1 0.1]\n}\n",
    );
    let (code, _, err) = sau(&misspelt);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.starts_with("error[null-unknown-property]: a world has no `colr`; did you mean `color`?\n  --> world.noll:2:3"),
        "{err}"
    );
}

#[test]
fn a_misspelt_modifier_property_is_caught_with_its_fix() {
    // A modifier is a sub-object line, `modifier @bevel bevel { … }`, whose block holds the
    // properties of that kind of modifier (Null's spec, section 8; Sau task 1d-07a), so a
    // misspelt one is an error with the right name as its fix.
    let root = with_file(
        "modifier",
        "object @lid \"Lid\" {\n  modifier @bevel bevel {\n    wdth 4mm\n  }\n}\n",
    );
    let (code, _, err) = sau(&root);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains(
            "error[null-unknown-property]: a bevel modifier has no `wdth`; did you mean `width`?"
        ),
        "{err}"
    );
}
