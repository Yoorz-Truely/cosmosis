//! The corpus's projects beyond the slice, in `corpus/project/` (Sau task 1b-03g): each checks
//! clean against the project schema, and every file prints back unchanged in canonical form
//! (`null_print::canonical`). `cycles-render.sau` holds Cycles' render, object, light and world
//! settings as blocks of Cycles' own structs (S70); applying them is Sau task 1b-03i, so Sau's
//! apply tests, which read `corpus/slice/`, do not open these.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn projects() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/project");
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("cosmosis.noll").is_file())
        .collect();
    v.sort();
    v
}

fn noll_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            noll_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "noll") && !p.ends_with("cosmosis.noll") {
            out.push(p);
        }
    }
}

#[test]
fn each_project_checks_clean() {
    let projects = projects();
    assert!(!projects.is_empty());
    for p in projects {
        let out = Command::new(env!("CARGO_BIN_EXE_sau"))
            .args(["check", p.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}: {}",
            p.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn every_file_prints_back_unchanged() {
    let schema = cosmosis::schema_check::schema();
    for p in projects() {
        let mut files = Vec::new();
        noll_files(&p, &mut files);
        assert!(files.len() >= 3, "{}", p.display());
        for f in files {
            let text = fs::read_to_string(&f).unwrap();
            let printed = null_print::canonical(&text, schema);
            assert_eq!(printed, text, "{} is not in canonical form", f.display());
        }
    }
}

#[test]
fn cycles_render_holds_cycles_blocks() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/project/cycles-render.sau");
    let mut files = Vec::new();
    noll_files(&p, &mut files);
    let blocks: usize = files
        .iter()
        .map(|f| {
            let file = null_syntax::parse(&fs::read_to_string(f).unwrap());
            file.document()
                .nodes()
                .flat_map(|n| n.children().collect::<Vec<_>>())
                .filter(|c| c.name() == "cycles" && c.block().is_some())
                .count()
        })
        .sum();
    assert_eq!(blocks, 4, "a scene's, an object's, a light's and a world's");
}
