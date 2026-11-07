//! Tests of finding a project's root and checking its layout, one per layout error, on
//! project folders made in Cargo's temporary folder (Sau task C-01).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cosmosis::layout::{self, find_root};
use cosmosis::manifest::Kind;

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// A fresh, empty folder for one test.
fn folder(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "layout-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `files` (path, contents) under `root`, making folders as needed.
fn write(root: &Path, files: &[(&str, &[u8])]) {
    for (path, contents) in files {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }
}

/// A valid project: manifest, project file, a scene and an object.
fn project(name: &str) -> PathBuf {
    let root = folder(name);
    write(
        &root,
        &[
            ("cosmosis.noll", b"package \"p\" {\n  null-format 1\n}\n"),
            ("project.noll", b"project \"P\" {\n  scene @main\n}\n"),
            (
                "scenes/main.noll",
                b"scene @main \"Main\" {\n  object @cube\n}\n",
            ),
            ("objects/cube.noll", b"object @cube \"Cube\" {}\n"),
        ],
    );
    root
}

/// The diagnostics of checking `root`, rendered.
fn found(root: &Path, kind: Option<Kind>) -> Vec<String> {
    layout::check(root, kind)
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_valid_project_has_no_diagnostics() {
    let root = project("valid");
    let l = layout::check(&root, Some(Kind::Project));
    assert!(l.diagnostics.is_empty(), "{:?}", l.diagnostics);
    let paths: Vec<_> = l
        .sources
        .iter()
        .map(|s| s.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        paths,
        [
            "cosmosis.noll",
            "objects/cube.noll",
            "project.noll",
            "scenes/main.noll"
        ]
    );
    assert_eq!(l.sources[0].text, "package \"p\" {\n  null-format 1\n}\n");
}

#[test]
fn the_root_is_found_from_any_folder_or_file_inside() {
    let root = project("root").canonicalize().unwrap();
    assert_eq!(find_root(&root).unwrap(), root);
    assert_eq!(find_root(&root.join("objects")).unwrap(), root);
    assert_eq!(find_root(&root.join("objects/cube.noll")).unwrap(), root);
}

#[test]
fn a_folder_with_no_manifest_above_it_is_not_a_project() {
    // Cargo's temporary folder sits inside this repository, which has no cosmosis.noll above it.
    let dir = folder("none");
    let d = find_root(&dir).unwrap_err();
    assert_eq!(d.code, "cosmosis-no-project");
    assert!(
        d.message.starts_with("no cosmosis.noll in "),
        "{}",
        d.message
    );
    assert_eq!(
        d.help.as_deref(),
        Some(
            "a project is a folder with cosmosis.noll at its root, such as `package \"my-project\" { null-format 1 }`"
        )
    );
    assert_eq!(
        find_root(&dir.join("missing")).unwrap_err().code,
        "cosmosis-no-such-path"
    );
}

#[test]
fn root_names_that_must_be_files() {
    let root = project("not-a-file");
    fs::create_dir_all(root.join("view.noll")).unwrap();
    fs::write(root.join("view.noll/inside.noll"), "x\n").unwrap();
    assert_eq!(
        found(&root, None),
        [
            "error[cosmosis-layout-not-a-file]: view.noll is a folder, not a file\n  --> view.noll\n  \
          help: view.noll at a project's root is a Null file; rename the folder"
        ]
    );
}

#[test]
fn a_project_package_needs_project_noll() {
    let root = project("no-project-file");
    fs::remove_file(root.join("project.noll")).unwrap();
    assert_eq!(
        found(&root, Some(Kind::Project)),
        [
            "error[cosmosis-layout-no-project-file]: the project has no project.noll\n  \
          help: a package of kind project keeps its settings and scenes in project.noll at its root"
        ]
    );
    // Other kinds need none, and an unread manifest (no kind yet) asks for nothing.
    assert!(found(&root, Some(Kind::Material)).is_empty());
    assert!(found(&root, None).is_empty());
}

#[test]
fn null_files_are_utf8() {
    let root = project("utf8");
    write(
        &root,
        &[("objects/bad.noll", b"object @a {\n  name \"r\xe9\"\n}\n")],
    );
    assert_eq!(
        found(&root, None),
        [
            "error[cosmosis-not-utf8]: the file is not UTF-8 text from here on\n  --> objects/bad.noll:2:10\n  \
          help: Null files are plain UTF-8 text: save it as UTF-8"
        ]
    );
}

#[test]
fn content_files_are_named_by_their_sha256() {
    let root = project("content");
    let good = format!("media/{ABC_SHA256}.png");
    let empty = format!("data/{EMPTY_SHA256}.mesh");
    write(
        &root,
        &[(&good, b"abc"), (&empty, b""), ("media/.DS_Store", b"x")],
    );
    let l = layout::check(&root, None);
    assert!(l.diagnostics.is_empty(), "{:?}", l.diagnostics);
    assert_eq!(l.content_files, 2);
}

#[test]
fn a_content_file_with_another_name_is_an_error() {
    let root = project("content-name");
    write(
        &root,
        &[("media/oak_diffuse.png", b"abc"), ("media/noext", b"")],
    );
    assert_eq!(
        found(&root, None),
        [
            format!(
                "error[cosmosis-layout-content-name]: files in media are named by the SHA-256 of their \
                 contents, as <sha256>.<ext>\n  --> media/noext\n  help: rename it to media/{EMPTY_SHA256}.<ext>, \
                 with the file's own extension"
            ),
            format!(
                "error[cosmosis-layout-content-name]: files in media are named by the SHA-256 of their \
                 contents, as <sha256>.<ext>\n  --> media/oak_diffuse.png\n  help: rename it to \
                 media/{ABC_SHA256}.png"
            ),
        ]
    );
}

#[test]
fn a_content_file_whose_contents_changed_is_an_error() {
    let root = project("content-hash");
    let name = format!("media/{EMPTY_SHA256}.png");
    write(&root, &[(&name, b"abc")]);
    assert_eq!(
        found(&root, None),
        [format!(
            "error[cosmosis-layout-content-hash]: the file's contents have the SHA-256 {ABC_SHA256}, not the \
             one in its name\n  --> {name}\n  help: rename it to media/{ABC_SHA256}.png"
        )]
    );
}

#[test]
fn content_folders_hold_no_folders() {
    let root = project("content-folder");
    write(&root, &[("data/meshes/x.bin", b"")]);
    assert_eq!(
        found(&root, None),
        [
            "error[cosmosis-layout-content-folder]: data holds files named by their contents, not folders\n  \
          --> data/meshes\n  help: move the folder's files into it, each named by its SHA-256"
        ]
    );
}

#[test]
fn caches_outputs_and_nested_projects_are_skipped() {
    let root = project("skipped");
    write(
        &root,
        &[
            (".sau/cache.noll", b"\xff"),
            ("renders/notes.noll", b"\xff"),
            ("packages/trees/cosmosis.noll", b"\xff"),
            ("scripts/scatter.py", b"\xff"),
            ("objects/renders/fine.noll", b"object @x {}\n"),
        ],
    );
    let l = layout::check(&root, None);
    let rendered: Vec<String> = l.diagnostics.iter().map(ToString::to_string).collect();
    assert_eq!(
        rendered,
        [
            "note[cosmosis-layout-nested-project]: skipped: this folder is a project of its own\n  --> packages/trees"
        ]
    );
    assert_eq!(l.errors(), 0);
    // `renders/` is skipped only at the root; deeper, it is an ordinary folder.
    assert!(
        l.sources
            .iter()
            .any(|s| s.path == Path::new("objects/renders/fine.noll"))
    );
}

/// Editors save by writing a temporary file and renaming it over the original (task C-04).
/// While another thread does that as fast as it can, 200 checks of the folder must never report
/// the temporary file, which can vanish between the folder's listing and its reading, and every
/// check must still read the object's file.
#[test]
fn a_file_renamed_over_another_while_walking_is_not_a_broken_link() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let root = project("save-by-rename");
    let stop = Arc::new(AtomicBool::new(false));
    let saver = {
        let (root, stop) = (root.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(Ordering::Relaxed) {
                // As an editor saves: a temporary file beside the original (one ending in .noll
                // too, as some editors name them), then renamed over it.
                for temp in ["objects/.cube.noll.tmp", "objects/cube.noll.saving.noll"] {
                    let path = root.join(temp);
                    fs::write(&path, format!("object @cube \"Cube\" {{}} // save {n}\n")).unwrap();
                    fs::rename(&path, root.join("objects/cube.noll")).unwrap();
                    n += 1;
                }
            }
            n
        })
    };
    let mut wrong = Vec::new();
    for _ in 0..200 {
        let l = layout::check(&root, Some(Kind::Project));
        wrong.extend(l.diagnostics.iter().map(ToString::to_string));
        assert!(
            l.sources
                .iter()
                .any(|s| s.path == Path::new("objects/cube.noll")),
            "the object's file was not read"
        );
    }
    stop.store(true, Ordering::Relaxed);
    let saves = saver.join().unwrap();
    assert!(saves > 0);
    assert!(
        wrong.is_empty(),
        "{} reports over 200 checks: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(3)]
    );
}

/// A link to nothing is still an error, as it was before C-04.
#[test]
#[cfg(unix)]
fn a_real_broken_link_is_still_reported() {
    let root = project("broken-link");
    std::os::unix::fs::symlink(
        root.join("objects/missing.noll"),
        root.join("objects/link.noll"),
    )
    .unwrap();
    let found = found(&root, Some(Kind::Project));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("broken link") && found[0].contains("objects/link.noll"),
        "{found:?}"
    );
}

/// Editors' and the system's files are not the project's, even ending in `.noll` (task C-05).
#[test]
fn editors_and_system_files_are_ignored_even_ending_in_noll() {
    let root = project("ignored");
    let again = b"object @cube \"Cube\" {}\n";
    let ignored = [
        "objects/.cube.noll",
        "objects/._cube.noll",
        "objects/.#cube.noll",
        "objects/~cube.noll",
        "objects/~$cube.noll",
        "objects/cube.noll~",
        "objects/#cube.noll#",
        "objects/cube.noll.tmp",
        "objects/cube.noll.swp",
        "objects/cube.noll.vsctmp",
        "objects/cube.noll.saving.noll",
        "objects/cube.noll.sb-1a2b3c4d-AbCdEf",
        "objects/cube.sb-1a2b3c4d-AbCdEf.noll",
        "objects/cube.noll___jb_tmp___",
        "objects/(A Document Being Saved By TextEdit)/cube.noll",
        "objects/.hidden/cube.noll",
    ];
    for name in ignored {
        write(&root, &[(name, again)]);
    }
    let l = layout::check(&root, Some(Kind::Project));
    assert!(l.diagnostics.is_empty(), "{:?}", l.diagnostics);
    let mut paths: Vec<_> = l
        .sources
        .iter()
        .map(|s| s.path.to_string_lossy().into_owned())
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "cosmosis.noll",
            "objects/cube.noll",
            "project.noll",
            "scenes/main.noll"
        ]
        .map(String::from)
    );
}

#[test]
fn ordinary_names_are_not_ignored() {
    for name in [
        "cube.noll",
        "pier-2.noll",
        "my.cube.noll",
        "tmp.noll",
        "swap.noll",
        "bak",
        "café.noll",
    ] {
        assert!(!layout::is_ignored(name), "{name}");
    }
}

/// The C-04 review's stress, as C-05's row asks: 300 checks of a corpus project, manifest to
/// Null's checker, while another thread saves its object by renaming over it from three
/// temporary names, one ending in `.noll`. No check may report anything.
#[test]
fn checks_during_saves_by_rename_report_nothing() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/slice/one-cube.sau");
    let root = folder("one-cube-saving");
    copy_tree(&corpus, &root);
    let cube = root.join("objects/cube.noll");
    let text = fs::read_to_string(&cube).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let saver = {
        let (root, stop, text) = (root.clone(), Arc::clone(&stop), text.clone());
        std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(Ordering::Relaxed) {
                for temp in ["cube.noll.saving.noll", ".cube.noll.tmp", "cube.noll~"] {
                    let path = root.join("objects").join(temp);
                    fs::write(&path, &text).unwrap();
                    fs::rename(&path, root.join("objects/cube.noll")).unwrap();
                    n += 1;
                }
            }
            n
        })
    };
    let mut wrong = Vec::new();
    for _ in 0..300 {
        let l = layout::check(&root, Some(Kind::Project));
        wrong.extend(l.diagnostics.iter().map(ToString::to_string));
        wrong.extend(
            cosmosis::schema_check::check(&l.sources)
                .iter()
                .map(ToString::to_string),
        );
    }
    stop.store(true, Ordering::Relaxed);
    let saves = saver.join().unwrap();
    assert!(saves > 0);
    assert!(
        wrong.is_empty(),
        "{} reports over 300 checks ({saves} saves): {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(3)]
    );
}

fn copy_tree(from: &Path, to: &Path) {
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&e.path(), &target);
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}
