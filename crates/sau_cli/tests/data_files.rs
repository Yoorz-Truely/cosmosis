//! Level 0 data files in the layout (Sau task C-10; Null spec, section 9): `sau check` finds each
//! `file=` from the folder of the file that names it and reports each mistake once, at its line;
//! `sau fmt` writes a data file one element per line.

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

/// The corpus's rock, whose arrays are in data files.
fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/project/rock-data.sau")
}

/// A copy of the corpus's rock in a fresh folder, with `files` written over it and `gone`
/// removed.
fn rock(files: &[(&str, &str)], gone: &[&str]) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "data-files-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    copy(&corpus(), &root);
    for path in gone {
        fs::remove_file(root.join(path)).unwrap();
    }
    for (path, text) in files {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    root
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy(&e.path(), &target);
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}

const MESH: &str = "meshes/rock.noll";

fn mesh() -> String {
    fs::read_to_string(corpus().join(MESH)).unwrap()
}

/// `sau check`'s errors: (code, where) for each, and all it printed.
fn errors(root: &Path) -> (Vec<(String, String)>, String) {
    let out = sau(&["check", root.to_str().unwrap()]);
    let err = String::from_utf8(out.stderr).unwrap();
    let mut found = Vec::new();
    let mut lines = err.lines().peekable();
    while let Some(l) = lines.next() {
        if let Some(rest) = l.strip_prefix("error[") {
            let code = rest.split(']').next().unwrap().to_string();
            let at = lines
                .peek()
                .and_then(|n| n.trim().strip_prefix("--> "))
                .unwrap_or("")
                .to_string();
            found.push((code, at));
        }
    }
    assert_eq!(out.status.success(), found.is_empty(), "{err}");
    (found, err)
}

#[test]
fn the_corpus_rock_checks_clean_with_its_data_files_counted() {
    let out = sau(&["check", corpus().to_str().unwrap()]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("10 Null files check, 3 of them data files"),
        "{stdout}"
    );
}

#[test]
fn a_drawings_arrays_in_data_files_are_named_by_its_handle() {
    // RFC 0006 (S91, task 1d-06f): in a Grease Pencil drawing's block, `points file=…` names a
    // data file whose header is the drawing's handle and the line's name (`@first.points`), and
    // an attribute's is the drawing's handle, `/` and the attribute's (`@first/@radius`), as
    // Null's checker matches them. The corpus's sketch holds both.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/project/geometry.sau");
    let (found, err) = errors(&root);
    assert!(found.is_empty(), "{err}");
    // A header naming the array as a datablock's own is the error at the line.
    let points = "grease-pencils/sketch.first.points.noll";
    let geometry = |files: &[(&str, &str)]| {
        static N: AtomicUsize = AtomicUsize::new(0);
        let to = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "data-files-geometry-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&to);
        copy(&root, &to);
        for (p, t) in files {
            fs::write(to.join(p), t).unwrap();
        }
        to
    };
    let data = fs::read_to_string(root.join(points)).unwrap();
    let wrong = geometry(&[(points, &data.replace("@first.points [", "points ["))]);
    let (found, err) = errors(&wrong);
    assert!(
        found.iter().any(|(c, _)| c == "cosmosis-data-file-header"),
        "{err}"
    );
    // An attribute's header naming it as the datablock's own attribute: the same.
    let radius = "grease-pencils/sketch.first.radius.noll";
    let data = fs::read_to_string(root.join(radius)).unwrap();
    let wrong = geometry(&[(radius, &data.replace("@first/@radius [", "@radius ["))]);
    let (found, err) = errors(&wrong);
    assert!(
        found.iter().any(|(c, _)| c == "cosmosis-data-file-header"),
        "{err}"
    );
}

#[test]
fn a_data_file_may_be_in_another_folder() {
    let root = rock(
        &[
            (
                MESH,
                &mesh().replace("\"rock.heat.noll\"", "\"../data-files/rock.heat.noll\""),
            ),
            (
                "data-files/rock.heat.noll",
                &fs::read_to_string(corpus().join("meshes/rock.heat.noll")).unwrap(),
            ),
        ],
        &["meshes/rock.heat.noll"],
    );
    assert_eq!(errors(&root).0, []);
}

#[test]
fn a_missing_data_file_is_one_error_at_its_line() {
    // Nowhere in the project: Cosmósis's error, not Null's as well.
    let root = rock(&[], &["meshes/rock.heat.noll"]);
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-missing".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(
        err.contains("no file meshes/rock.heat.noll: `file=\"rock.heat.noll\"` names it for `@heat` of mesh @rock"),
        "{err}"
    );
    assert!(
        err.contains("a data file there holds `array mesh @rock @heat [ … ]`"),
        "{err}"
    );
    // Elsewhere: the help names it, and it is not also a file no line names.
    let root = rock(
        &[(MESH, &mesh().replace("\"rock.heat.noll\"", "\"heat.noll\""))],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-missing".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(
        err.contains("help: meshes/rock.heat.noll holds it: write `file=\"rock.heat.noll\"`"),
        "{err}"
    );
}

#[test]
fn a_data_file_holding_another_array_is_an_error_at_the_line() {
    let root = rock(
        &[(
            "meshes/rock.heat.noll",
            "array mesh @rock @hot [\n  0.25\n  4\n  -0\n  1\n]\n",
        )],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-header".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(
        err.contains(
            "meshes/rock.heat.noll holds `array mesh @rock @hot`, not `@heat` of mesh @rock"
        ),
        "{err}"
    );
    // Another owner.
    let root = rock(
        &[(
            "meshes/rock.polygons.noll",
            "array mesh @stone polygons [\n  [0 1 2]\n  [1 3 2]\n]\n",
        )],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-header".into(),
            "meshes/rock.noll:3:17".into()
        )],
        "{err}"
    );
}

#[test]
fn a_data_file_named_by_two_lines_is_an_error_at_the_second() {
    let root = rock(
        &[(
            MESH,
            &mesh().replace("\"rock.heat.noll\"", "\"rock.vertices.noll\""),
        )],
        &["meshes/rock.heat.noll"],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-twice".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(
        err.contains("meshes/rock.noll:2:17: first named here"),
        "{err}"
    );
}

#[test]
fn a_data_file_no_line_names_is_an_error() {
    let root = rock(
        &[(
            "meshes/rock.old-heat.noll",
            "array mesh @rock @old-heat [\n  1\n]\n",
        )],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-unnamed".into(),
            "meshes/rock.old-heat.noll:1:1".into()
        )],
        "{err}"
    );
}

#[test]
fn a_path_out_of_the_project_or_to_a_datablock_file_is_an_error() {
    let root = rock(
        &[(
            MESH,
            &mesh().replace("\"rock.heat.noll\"", "\"../../heat.noll\""),
        )],
        &["meshes/rock.heat.noll"],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-outside".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    let root = rock(
        &[(
            MESH,
            &mesh().replace("\"rock.heat.noll\"", "\"../objects/rock.noll\""),
        )],
        &["meshes/rock.heat.noll"],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-not-data".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
}

#[test]
fn fmt_writes_a_data_file_one_element_per_line() {
    let root = rock(
        &[
            (
                "meshes/rock.heat.noll",
                "array mesh @rock @heat [0.25 4 -0 1]\n",
            ),
            (
                "meshes/rock.vertices.noll",
                "array mesh @rock vertices [\n  [0m 0m 0m] [1m  0m 0.5m]\n  [0m 2m 0m]\n  [1m 2m 0.25m]\n]\n",
            ),
        ],
        &[],
    );
    let out = sau(&["fmt", root.to_str().unwrap()]);
    assert!(out.status.success());
    for f in [
        "meshes/rock.heat.noll",
        "meshes/rock.vertices.noll",
        "meshes/rock.polygons.noll",
    ] {
        assert_eq!(
            fs::read_to_string(root.join(f)).unwrap(),
            fs::read_to_string(corpus().join(f)).unwrap(),
            "{f}"
        );
    }
    // The corpus is already so.
    let out = sau(&["fmt", "--check", corpus().to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_mesh_whose_handle_comes_from_its_name_names_its_data_files() {
    // `mesh "Rock"` is `@rock` (S18), as Null's checker reads it.
    let root = rock(
        &[(
            MESH,
            &mesh().replace("mesh @rock \"Rock\"", "mesh \"Rock\""),
        )],
        &[],
    );
    assert_eq!(errors(&root).0, []);
}

#[test]
fn a_wrong_path_to_another_array_s_file_is_one_error_naming_the_right_one() {
    // The heat line names an old file holding another array; the heat file is the one.
    let root = rock(
        &[
            (
                MESH,
                &mesh().replace("\"rock.heat.noll\"", "\"rock.old.noll\""),
            ),
            ("meshes/rock.old.noll", "array mesh @rock @old [\n  1\n]\n"),
        ],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-header".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(
        err.contains("help: meshes/rock.heat.noll holds it: write `file=\"rock.heat.noll\"`"),
        "{err}"
    );
    // Named by another line too: one error, naming the right one.
    let root = rock(
        &[(
            MESH,
            &mesh().replace("\"rock.heat.noll\"", "\"rock.polygons.noll\""),
        )],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-twice".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(err.contains("meshes/rock.heat.noll holds it"), "{err}");
}

#[test]
fn a_data_file_in_a_content_folder_is_in_the_wrong_place() {
    // The file is there too: the layout does not also ask to rename it by its hash.
    let root = rock(
        &[
            (
                MESH,
                &mesh().replace("\"rock.heat.noll\"", "\"../data/rock.heat.noll\""),
            ),
            ("data/rock.heat.noll", "array mesh @rock @heat [\n  1\n]\n"),
        ],
        &[],
    );
    let (found, err) = errors(&root);
    assert_eq!(
        found,
        [(
            "cosmosis-data-file-place".into(),
            "meshes/rock.noll:4:43".into()
        )],
        "{err}"
    );
    assert!(err.contains("meshes/rock.heat.noll holds it"), "{err}");
}

#[test]
fn a_shape_keys_arrays_in_data_files_are_named_by_its_handle() {
    // RFC 0005 (S88, task 1d-07r): in a shape key's block, `positions file=…` names a data file
    // whose header is the key's handle (`array mesh @pebble @lift`), as Null's checker matches
    // it; another of its arrays is the handle and the line's name.
    let pebble = "meshes/pebble.noll";
    let text = fs::read_to_string(corpus().join(pebble)).unwrap().replace(
        "    positions [[0m 0m 0m] [1m 0m 0.5m] [0m 1m 0.25m] [1m 1m 1m]]\n",
        "    positions file=\"pebble.lift.noll\" count=4 bounds=[[0m 0m 0m] [1m 1m 1m]]\n",
    );
    let data = "array mesh @pebble @lift [\n  [0m 0m 0m]\n  [1m 0m 0.5m]\n  [0m 1m 0.25m]\n  [1m 1m 1m]\n]\n";
    let root = rock(&[(pebble, &text), ("meshes/pebble.lift.noll", data)], &[]);
    let (found, err) = errors(&root);
    assert!(found.is_empty(), "{err}");
    // The header naming `positions` instead is the error at the line.
    let root = rock(
        &[
            (pebble, &text),
            (
                "meshes/pebble.lift.noll",
                &data.replace("@lift [", "positions ["),
            ),
        ],
        &[],
    );
    let (found, err) = errors(&root);
    assert!(
        found.iter().any(|(c, _)| c == "cosmosis-data-file-header"),
        "{err}"
    );
}
