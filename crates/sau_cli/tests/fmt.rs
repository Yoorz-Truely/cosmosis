//! `sau fmt` (Sau task C-07): a project's files in Null's canonical form. Shuffled copies of
//! every corpus file come back as the corpus, byte for byte; formatting twice changes nothing;
//! comments survive; `sau fmt --check` fails on a shuffled file, writing nothing, and passes on
//! the corpus.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sau(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sau"))
        .args(args)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn corpus_projects() -> Vec<PathBuf> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let mut out = Vec::new();
    for group in ["slice", "project"] {
        for e in fs::read_dir(corpus.join(group)).unwrap() {
            let p = e.unwrap().path();
            if p.join("cosmosis.noll").is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let p = e.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_tree(&p, &dest);
        } else {
            fs::copy(&p, &dest).unwrap();
        }
    }
}

fn noll_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            noll_files(root, &p, out);
        } else if p.extension().is_some_and(|x| x == "noll") {
            out.push(p.strip_prefix(root).unwrap().to_path_buf());
        }
    }
    out.sort();
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("fmt-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// `text` with each top-level block whose lines are all single lines (no nested block, no
/// comment) in another order: the lines of each name together, as canonical order keeps them,
/// the names in reverse order of their first appearance.
fn shuffled(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        out.push(line.to_string());
        i += 1;
        if !(line.ends_with('{') && !line.starts_with(' ')) {
            continue;
        }
        let end = (i..lines.len()).find(|&j| lines[j] == "}").unwrap();
        let children = &lines[i..end];
        let simple = children.iter().all(|c| {
            c.starts_with("  ")
                && !c.starts_with("   ")
                && !c.contains('{')
                && !c.contains('}')
                && !c.contains("//")
        });
        if simple && children.len() > 1 {
            let mut names: Vec<&str> = Vec::new();
            for c in children {
                let name = c.split_whitespace().next().unwrap();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            names.reverse();
            for name in names {
                out.extend(
                    children
                        .iter()
                        .filter(|c| c.split_whitespace().next() == Some(name))
                        .map(|c| c.to_string()),
                );
            }
        } else {
            out.extend(children.iter().map(|c| c.to_string()));
        }
        i = end;
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

#[test]
fn shuffled_corpus_files_come_back_as_the_corpus_byte_for_byte() {
    let mut moved = 0;
    for project in corpus_projects() {
        let name = project.file_name().unwrap().to_string_lossy().to_string();
        let copy = scratch(&name);
        copy_tree(&project, &copy);
        let mut files = Vec::new();
        noll_files(&copy, &copy, &mut files);
        for f in &files {
            let original = fs::read_to_string(copy.join(f)).unwrap();
            let mixed = shuffled(&original);
            if mixed != original {
                moved += 1;
            }
            fs::write(copy.join(f), mixed).unwrap();
        }
        // --check finds the shuffled files and writes nothing.
        let before: Vec<String> = files
            .iter()
            .map(|f| fs::read_to_string(copy.join(f)).unwrap())
            .collect();
        let check = sau(&["fmt", "--check", copy.to_str().unwrap()]);
        let after: Vec<String> = files
            .iter()
            .map(|f| fs::read_to_string(copy.join(f)).unwrap())
            .collect();
        assert_eq!(before, after, "{name}: --check wrote a file");
        let shuffled_any = files
            .iter()
            .zip(&before)
            .any(|(f, t)| fs::read_to_string(project.join(f)).unwrap() != *t);
        if shuffled_any {
            assert_eq!(
                check.status.code(),
                Some(1),
                "{name}: {}",
                text(&check.stdout)
            );
            assert!(
                text(&check.stderr).contains("not in canonical form"),
                "{}",
                text(&check.stderr)
            );
        }
        // sau fmt gives the corpus back, byte for byte.
        let out = sau(&["fmt", copy.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(0), "{name}: {}", text(&out.stderr));
        for f in &files {
            assert_eq!(
                fs::read_to_string(copy.join(f)).unwrap(),
                fs::read_to_string(project.join(f)).unwrap(),
                "{name}/{} is not the corpus's",
                f.display()
            );
        }
        // Twice changes nothing.
        let again = sau(&["fmt", copy.to_str().unwrap()]);
        assert!(
            text(&again.stdout).contains(&format!("sau fmt: 0 of {} Null files", files.len())),
            "{}",
            text(&again.stdout)
        );
        fs::remove_dir_all(&copy).unwrap();
    }
    assert!(moved >= 10, "the shuffle moved only {moved} files");
}

#[test]
fn the_corpus_passes_sau_fmt_check() {
    for project in corpus_projects() {
        let out = sau(&["fmt", "--check", project.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}: {}{}",
            project.display(),
            text(&out.stdout),
            text(&out.stderr)
        );
    }
}

#[test]
fn comments_survive_and_stay_with_their_lines() {
    let root = scratch("comments");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice/one-cube.sau"),
        &root,
    );
    let cube = root.join("objects/cube.noll");
    fs::write(
        &cube,
        "object @cube \"Cube\" { // the hero\n  // Where it stands.\n  location [0m 0m 1m]\n  scale [1 1 1] // as made\n  data mesh @cube-mesh\n}\n\nmesh @cube-mesh \"Cube\" {\n  primitive cube size=2m\n}\n",
    )
    .unwrap();
    let out = sau(&["fmt", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(
        fs::read_to_string(&cube).unwrap(),
        "object @cube \"Cube\" { // the hero\n  data mesh @cube-mesh\n  // Where it stands.\n  location [0m 0m 1m]\n  scale [1 1 1] // as made\n}\n\nmesh @cube-mesh \"Cube\" {\n  primitive cube size=2m\n}\n"
    );
    let check = sau(&["check", root.to_str().unwrap()]);
    assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_default_goes_unless_a_comment_says_why_it_is_written() {
    // Sau task C-09 (Null spec, section 15): a line of its own holding its property's default is
    // left out; one with a comment on it or just above it stays, as the corpus keeps its
    // examples of a form.
    let root = scratch("defaults");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice/one-cube.sau"),
        &root,
    );
    let cube = root.join("objects/cube.noll");
    fs::write(
        &cube,
        "object @cube \"Cube\" {\n  data mesh @cube-mesh\n  location [0m 0m 1m]\n  rotation-mode xyz\n  pass-index 0 // for the compositor\n  pass-index 4\n}\n\nmesh @cube-mesh \"Cube\" {\n  primitive cube size=2m\n}\n",
    )
    .unwrap();
    let check = sau(&["fmt", "--check", root.to_str().unwrap()]);
    assert_eq!(check.status.code(), Some(1), "{}", text(&check.stdout));
    let out = sau(&["fmt", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(
        fs::read_to_string(&cube).unwrap(),
        "object @cube \"Cube\" {\n  data mesh @cube-mesh\n  location [0m 0m 1m]\n  pass-index 0 // for the compositor\n  pass-index 4\n}\n\nmesh @cube-mesh \"Cube\" {\n  primitive cube size=2m\n}\n"
    );
    // And once more changes nothing.
    let again = sau(&["fmt", "--check", root.to_str().unwrap()]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again.stdout));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn sau_fmt_refuses_wrong_usage_and_a_folder_outside_a_project() {
    assert_eq!(sau(&["fmt", "--quick"]).status.code(), Some(2));
    assert_eq!(sau(&["fmt", "a", "b"]).status.code(), Some(2));
    let outside = scratch("outside");
    fs::create_dir_all(&outside).unwrap();
    let out = sau(&["fmt", outside.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(sau(&["fmt", "--help"]).status.success());
}
