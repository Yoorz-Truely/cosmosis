//! `sau new empty <path>` (Sau task 1d-04e): a project with the `.gitignore` and
//! `.gitattributes` Cosmósis ships, which Git then reads as the templates say.

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
    String::from_utf8(b.to_vec()).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("new-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) -> String {
    text(
        &Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
            .stdout,
    )
}

#[test]
fn a_new_project_has_the_shipped_git_files_and_checks_clean() {
    let root = scratch("harbor").join("Harbor at Dusk.sau");
    let out = sau(&["new", "empty", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("made package \"harbor-at-dusk\""));
    assert_eq!(
        fs::read_to_string(root.join(".gitignore")).unwrap(),
        cosmosis::templates::GITIGNORE
    );
    assert_eq!(
        fs::read_to_string(root.join(".gitattributes")).unwrap(),
        cosmosis::templates::GITATTRIBUTES
    );
    assert!(
        fs::read_to_string(root.join("cosmosis.noll"))
            .unwrap()
            .starts_with("package \"harbor-at-dusk\" {")
    );
    // The project checks clean as made.
    let check = sau(&["check", root.to_str().unwrap()]);
    assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
    // And Git reads its files as Cosmósis means: the cache but the ID map ignored, with
    // view.noll and renders/; media/ and data/ in LFS.
    git(&root, &["init", "-q"]);
    let ignored = git(
        &root,
        &[
            "check-ignore",
            "--no-index",
            ".sau/snapshot.blend",
            ".sau/ids.noll",
            "view.noll",
            "renders/a.png",
            "settings.noll",
            "project.noll",
            "objects/pier.noll",
        ],
    );
    assert_eq!(ignored, ".sau/snapshot.blend\nview.noll\nrenders/a.png\n");
    let hash = "0".repeat(64);
    let attrs = git(
        &root,
        &[
            "check-attr",
            "filter",
            "--",
            &format!("media/{hash}.png"),
            &format!("data/{hash}.mesh"),
        ],
    );
    assert_eq!(
        attrs,
        format!("media/{hash}.png: filter: lfs\ndata/{hash}.mesh: filter: lfs\n")
    );
}

#[test]
fn sau_new_refuses_a_folder_in_use_an_unknown_template_and_a_nameless_folder() {
    let busy = scratch("busy");
    fs::write(busy.join("x.txt"), "x").unwrap();
    let out = sau(&["new", "empty", busy.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("exists and is not an empty folder"),
        "{}",
        text(&out.stderr)
    );
    assert!(!busy.join("cosmosis.noll").exists());
    let empty = scratch("empty-folder");
    assert_eq!(
        sau(&["new", "empty", empty.to_str().unwrap()])
            .status
            .code(),
        Some(0)
    );
    let other = scratch("other").join("p");
    let out = sau(&["new", "product-shot", other.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("no template `product-shot` yet"),
        "{}",
        text(&out.stderr)
    );
    let nameless = scratch("nameless").join("!!!.sau");
    let out = sau(&["new", "empty", nameless.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
    assert_eq!(sau(&["new"]).status.code(), Some(2));
}

/// The project's name in `project.noll`, read back as Null reads it.
fn project_name(root: &Path) -> String {
    let text = fs::read_to_string(root.join("project.noll")).unwrap();
    let (tokens, errors) = null_syntax::lex(&text);
    assert!(errors.is_empty(), "{errors:?} in {text}");
    let mut at = 0;
    for t in tokens {
        let end = at + t.len as usize;
        if t.kind == null_syntax::TokenKind::String {
            return null_syntax::unescape(&text[at..end]);
        }
        at = end;
    }
    panic!("no name in {text}")
}

#[test]
fn a_folder_name_with_quotes_or_backslashes_is_written_escaped_and_reads_back() {
    for folder in ["Back\\slash", "Quote \"x\""] {
        let root = scratch("escaped").join(format!("{folder}.sau"));
        let out = sau(&["new", "empty", root.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
        assert_eq!(project_name(&root), folder);
        let check = sau(&["check", root.to_str().unwrap()]);
        assert_eq!(
            check.status.code(),
            Some(0),
            "{folder}: {}{}",
            text(&check.stdout),
            text(&check.stderr)
        );
    }
}

#[test]
fn only_one_sau_suffix_comes_off_the_name() {
    let root = scratch("double").join("x.sau.sau");
    let out = sau(&["new", "empty", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(project_name(&root), "x.sau");
}
