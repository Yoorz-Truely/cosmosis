//! The folder layout complete (Sau task 1d-04b, decisions S36 and S66): one test per rule of
//! Cosmósis spec section 2 and per error, each error with its file and line, through `sau check`.
//! The layout's own errors (root files, UTF-8, content-addressed folders) and the ignored names
//! are tested in `check.rs` and in the `cosmosis` crate's `tests/layout.rs`.

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

/// A fresh project of kind `project` with `files` beside its manifest and `project.noll`.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "layout-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("cosmosis.noll"),
        "package \"p\" {\n  null-format 1\n  kind project\n}\n",
    )
    .unwrap();
    fs::write(root.join("project.noll"), "project \"P\" {}\n").unwrap();
    for (path, contents) in files {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }
    root
}

/// `sau check` on `root`: (exit code, stderr).
fn check(root: &Path) -> (Option<i32>, String) {
    let out = sau(&["check", root.to_str().unwrap()]);
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

fn passes(root: &Path) {
    let (code, err) = check(root);
    assert_eq!(code, Some(0), "{err}");
    assert!(err.is_empty(), "{err}");
}

/// Fails with exactly the errors given, each `(code, "file:line:column")`, in order.
fn fails_with(root: &Path, errors: &[(&str, &str)]) -> String {
    let (code, err) = check(root);
    assert_eq!(code, Some(1), "{err}");
    let found: Vec<(String, String)> = err
        .split("\n\n")
        .filter_map(|block| {
            let code = block.strip_prefix("error[")?.split(']').next()?.to_string();
            let at = block.split("\n  --> ").nth(1)?.lines().next()?.to_string();
            Some((code, at))
        })
        .collect();
    let want: Vec<(String, String)> = errors
        .iter()
        .map(|(c, a)| (c.to_string(), a.to_string()))
        .collect();
    assert_eq!(found, want, "{err}");
    err
}

const CAMERA: &str = "camera @lens \"Lens\" {\n  lens 50mm\n}\n";

#[test]
fn the_layout_is_free_any_folder_and_any_file_name() {
    // A datablock may live in any folder under any name; references need no imports (S36).
    let root = project(
        "free",
        &[
            ("anything/deep/inside/here.noll", CAMERA),
            (
                "shots/opening.noll",
                "object @cam \"Cam\" {\n  data camera @lens\n}\n",
            ),
            ("project.noll", "project \"P\" {}\n"),
        ],
    );
    passes(&root);
}

#[test]
fn a_handle_defined_twice_names_both_files_and_lines() {
    let root = project(
        "twice",
        &[
            ("objects/a.noll", "object @pier \"Pier\"\n"),
            (
                "objects/b.noll",
                "// The same pier again.\nobject @pier \"Pier 2\"\n",
            ),
        ],
    );
    let err = fails_with(&root, &[("null-handle", "objects/b.noll:2:8")]);
    assert!(
        err.contains("\n  ::: objects/a.noll:1:8: first defined here"),
        "{err}"
    );
}

#[test]
fn a_reference_is_found_in_any_file_and_a_missing_one_is_an_error_at_its_line() {
    let found = project(
        "found",
        &[
            ("cameras/lens.noll", CAMERA),
            (
                "objects/cam.noll",
                "object @cam \"Cam\" {\n  data camera @lens\n}\n",
            ),
        ],
    );
    passes(&found);
    let missing = project(
        "missing",
        &[(
            "objects/cam.noll",
            "object @cam \"Cam\" {\n  data camera @lense\n}\n",
        )],
    );
    fails_with(&missing, &[("null-handle", "objects/cam.noll:2:15")]);
}

#[test]
fn the_cache_is_skipped_but_for_the_id_map() {
    // Anything in `.sau/` is cache, even Null files with errors in them; `.sau/ids.noll` is the
    // one source file there (S66), clean here.
    let root = project(
        "cache",
        &[
            (".sau/snapshot.blend", "\u{0}\u{1}not text"),
            (".sau/thumbnails/x.noll", "object @x {\n"),
            (".sau/ids.noll", "ids {\n}\n"),
        ],
    );
    passes(&root);
}

#[test]
fn the_id_map_is_read_for_its_syntax_and_must_be_a_file() {
    let broken = project("ids-syntax", &[(".sau/ids.noll", "ids {\n  map [\n}\n")]);
    fails_with(&broken, &[("null-syntax", ".sau/ids.noll:2:7")]);
    let bytes = project("ids-utf8", &[]);
    fs::create_dir_all(bytes.join(".sau")).unwrap();
    fs::write(bytes.join(".sau/ids.noll"), b"ids {\n  \xff\n}\n").unwrap();
    fails_with(&bytes, &[("cosmosis-not-utf8", ".sau/ids.noll:2:3")]);
    let folder = project("ids-folder", &[(".sau/ids.noll/x", "")]);
    fails_with(&folder, &[("cosmosis-layout-not-a-file", ".sau/ids.noll")]);
    // A link to nothing is an error, as a broken link anywhere else is.
    let broken_link = project("ids-broken-link", &[]);
    fs::create_dir_all(broken_link.join(".sau")).unwrap();
    std::os::unix::fs::symlink("gone.noll", broken_link.join(".sau/ids.noll")).unwrap();
    fails_with(&broken_link, &[("cosmosis-io", ".sau/ids.noll")]);
}

#[test]
fn a_linked_cache_folder_is_not_followed_for_the_id_map() {
    // `.sau` as a link to another folder, holding a broken map: not followed, so not read, and
    // said in a note (no folder link is followed).
    let elsewhere = project(
        "cache-elsewhere",
        &[(".sau/ids.noll", "ids {\n  map [\n}\n")],
    );
    let root = project("cache-link", &[]);
    std::os::unix::fs::symlink(elsewhere.join(".sau"), root.join(".sau")).unwrap();
    let (code, err) = check(&root);
    assert_eq!(code, Some(0), "{err}");
    assert!(err.contains("note[cosmosis-layout-link]: a link to a folder is not followed, so .sau/ids.noll is not read\n  --> .sau"), "{err}");
}

#[test]
fn settings_and_view_are_null_text_whose_forms_are_not_checked_yet() {
    // Not datablocks: anything that parses passes (S66).
    let root = project(
        "settings",
        &[
            ("settings.noll", "theme \"dark\"\nautosave off\n"),
            (
                "view.noll",
                "viewport { camera @cam; shading solid }\nopen \"objects/pier.noll\"\n",
            ),
        ],
    );
    passes(&root);
    // Their syntax is checked, at its line.
    let broken = project(
        "settings-syntax",
        &[("settings.noll", "theme \"dark\nautosave off\n")],
    );
    let (code, err) = check(&broken);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("\n  --> settings.noll:1:"), "{err}");
    let view = project(
        "view-syntax",
        &[("view.noll", "viewport {\n  camera [\n}\n")],
    );
    fails_with(&view, &[("null-syntax", "view.noll:2:10")]);
}

/// Runs git in `dir`, its stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_shipped_gitignore_keeps_source_and_the_id_map() {
    // The template, as Git itself reads it in a fresh repository.
    let root = project("gitignore", &[]);
    fs::write(root.join(".gitignore"), cosmosis::templates::GITIGNORE).unwrap();
    git(&root, &["init", "-q"]);
    let paths = [
        ".sau/snapshot.blend",
        ".sau/thumbnails/a.png",
        ".sau/ids.noll",
        "view.noll",
        "renders/shot-1.png",
        "cosmosis.noll",
        "project.noll",
        "settings.noll",
        "objects/pier.noll",
        "media/0000000000000000000000000000000000000000000000000000000000000000.png",
    ];
    let mut args = vec!["check-ignore", "--no-index"];
    args.extend(paths);
    let ignored = git(&root, &args);
    let ignored: Vec<&str> = ignored.lines().collect();
    assert_eq!(
        ignored,
        [
            ".sau/snapshot.blend",
            ".sau/thumbnails/a.png",
            "view.noll",
            "renders/shot-1.png"
        ]
    );
}

#[test]
fn the_shipped_gitattributes_puts_content_in_lfs() {
    let root = project("gitattributes", &[]);
    fs::write(
        root.join(".gitattributes"),
        cosmosis::templates::GITATTRIBUTES,
    )
    .unwrap();
    git(&root, &["init", "-q"]);
    let hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let media = format!("media/{hash}.png");
    let data = format!("data/{hash}.mesh");
    let out = git(
        &root,
        &[
            "check-attr",
            "filter",
            "--",
            &media,
            &data,
            "objects/pier.noll",
        ],
    );
    assert_eq!(
        out,
        format!(
            "{media}: filter: lfs\n{data}: filter: lfs\nobjects/pier.noll: filter: unspecified\n"
        )
    );
}
