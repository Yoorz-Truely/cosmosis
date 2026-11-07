//! Tests of the `sau` command as a program: exit status, help, and `sau check` on valid and
//! broken project folders (Sau task C-01).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn sau(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sau"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// A fresh project folder with a manifest and `files`.
fn project(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "cli-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("cosmosis.noll"),
        "package \"p\" {\n  null-format 1\n}\n",
    )
    .unwrap();
    for (path, contents) in files {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }
    root
}

#[test]
fn help_lists_every_command() {
    let out = sau(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let help = text(&out.stdout);
    for command in [
        "new", "open", "dev", "watch", "check", "fmt", "convert", "render", "test", "lsp",
    ] {
        assert!(
            help.contains(&format!("\n  {command} ")),
            "{command} missing from:\n{help}"
        );
    }
    assert!(
        help.contains("  check    Check a project's layout and files, with diagnostics\n"),
        "{help}"
    );
    assert!(
        help.contains("  open     Open a project in Sau (not built yet)\n"),
        "{help}"
    );
    // `sau` alone prints the same help, as an error.
    let bare = sau(&[]);
    assert_eq!(bare.status.code(), Some(2));
    assert_eq!(text(&bare.stderr), help);
}

#[test]
fn version() {
    let out = sau(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        text(&out.stdout),
        format!("sau {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn wrong_usage_exits_2() {
    let unknown = sau(&["chek"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(
        text(&unknown.stderr),
        "sau: no command `chek`; did you mean `check`?\nRun `sau --help` for the commands.\n"
    );
    let unbuilt = sau(&["render"]);
    assert_eq!(unbuilt.status.code(), Some(2));
    assert_eq!(
        text(&unbuilt.stderr),
        "sau: `render` is not built yet (Cosmósis spec, section 7)\n"
    );
    assert_eq!(sau(&["check", "--strict"]).status.code(), Some(2));
    assert_eq!(sau(&["check", "a", "b"]).status.code(), Some(2));
    let help = sau(&["check", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(text(&help.stdout).starts_with("Usage: sau check [path]\n"));
}

#[test]
fn a_valid_project_passes() {
    let root = project(
        "valid",
        &[
            ("project.noll", b"project \"P\" {}\n"),
            ("objects/a.noll", b"object @a {}\n"),
        ],
    );
    let out = sau(&["check", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let shown = root.canonicalize().unwrap();
    assert_eq!(
        text(&out.stdout),
        format!(
            "sau check: {}: the layout is valid (3 Null files, 0 content files)\n",
            shown.display()
        )
    );
    assert!(
        text(&out.stderr).starts_with("note[cosmosis-not-parsed]: "),
        "{}",
        text(&out.stderr)
    );
    // From a folder inside the project, the same root is found.
    let inner = sau(&["check", root.join("objects").to_str().unwrap()]);
    assert_eq!(inner.status.code(), Some(0));
    assert_eq!(inner.stdout, out.stdout);
}

/// `sau check` on a project with `files`: exits 1, and its first diagnostic has `code` and
/// names `file`.
fn fails_with(name: &str, files: &[(&str, &[u8])], code: &str, file: &str) {
    let root = project(name, files);
    let out = sau(&["check", root.to_str().unwrap()]);
    let err = text(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.starts_with(&format!("error[{code}]: ")), "{err}");
    assert!(err.contains(&format!("\n  --> {file}")), "{err}");
    assert!(
        err.ends_with(&format!(
            "sau check: 1 error in {}\n",
            root.canonicalize().unwrap().display()
        )),
        "{err}"
    );
}

#[test]
fn each_layout_error_fails_with_its_code_and_file() {
    fails_with(
        "utf8",
        &[("objects/bad.noll", b"\xff\n")],
        "cosmosis-not-utf8",
        "objects/bad.noll:1:1",
    );
    fails_with(
        "not-a-file",
        &[("project.noll/x", b"")],
        "cosmosis-layout-not-a-file",
        "project.noll",
    );
    fails_with(
        "content-name",
        &[("media/photo.png", b"")],
        "cosmosis-layout-content-name",
        "media/photo.png",
    );
    let wrong = "media/0000000000000000000000000000000000000000000000000000000000000000.png";
    fails_with(
        "content-hash",
        &[(wrong, b"")],
        "cosmosis-layout-content-hash",
        wrong,
    );
    fails_with(
        "content-folder",
        &[("data/sub/x", b"")],
        "cosmosis-layout-content-folder",
        "data/sub",
    );
}

#[test]
fn a_folder_outside_any_project_fails() {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{}-none", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let out = sau(&["check", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).starts_with("error[cosmosis-no-project]: no cosmosis.noll in "));
}
