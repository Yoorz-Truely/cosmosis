//! A project of an older Null format lifted to the newest (task 1d-04d, Sau's S45): a test
//! migration from a made-up format 0, written into the project and checked clean after; and
//! what is refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cosmosis::layout::{self, SourceFile};
use cosmosis::manifest::{MANIFEST_FILE, Manifest};
use cosmosis::{migrate, schema_check};
use null_check::migrate::{MIGRATIONS, Migration, Rename, rename_properties};

/// A made-up format 0, before v0's format 1: a camera's `lens` was `focal-length`.
fn zero_to_one(text: &str) -> String {
    rename_properties(
        text,
        &[Rename {
            kind: "camera",
            path: &[],
            old: "focal-length",
            new: "lens",
        }],
    )
}

const TEST: [Migration; 1] = [Migration {
    from: 0,
    what: "a camera's focal-length is lens",
    run: zero_to_one,
}];

/// A project of format 0 in a fresh folder: one scene, a camera with its `focal-length`, and the
/// handle map, which has its own form and is never migrated.
fn format_zero(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "migrate-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    for (path, text) in [
        ("cosmosis.noll", "package \"old\" {\n  null-format 0\n}\n"),
        (
            "project.noll",
            "project \"Old\" {\n  units metric scale=1 length=m rotation=deg\n  scene @main\n}\n",
        ),
        (
            "scenes/main.noll",
            "scene @main \"Main\" {\n  camera @eye\n  object @eye\n}\n",
        ),
        (
            "objects/eye.noll",
            "object @eye \"Eye\" {\n  data camera @eye-lens\n}\n\ncamera @eye-lens \"Eye\" {\n  focal-length 35mm\n}\n",
        ),
    ] {
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    root
}

fn sources(root: &Path) -> Vec<SourceFile> {
    layout::check(root, None).sources
}

fn manifest(root: &Path) -> String {
    fs::read_to_string(root.join(MANIFEST_FILE)).unwrap()
}

#[test]
fn a_test_migration_lifts_a_format_0_project_to_format_1() {
    let root = format_zero("lift");
    // As written, the manifest names a format this Sau does not read.
    assert!(Manifest::parse(&manifest(&root)).is_err());
    let m = migrate::project(&manifest(&root), &sources(&root), &TEST)
        .unwrap()
        .expect("format 0 is older");
    assert_eq!(m.from, 0);
    assert_eq!(m.steps, ["a camera's focal-length is lens"]);
    assert_eq!(m.manifest, "package \"old\" {\n  null-format 1\n}\n");
    let changed: Vec<&Path> = m.files.iter().map(|f| f.path.as_path()).collect();
    assert_eq!(changed, [Path::new("objects/eye.noll")]);
    let written = migrate::write(&root, &m).unwrap();
    assert_eq!(
        written,
        [
            PathBuf::from("objects/eye.noll"),
            PathBuf::from(MANIFEST_FILE)
        ]
    );
    // Now format 1: the manifest reads, every file checks against the schema, and the camera's
    // line is the only one that changed.
    let read = Manifest::parse(&manifest(&root)).unwrap();
    assert_eq!(read.null_format, 1);
    let now = sources(&root);
    let found = schema_check::check(&now);
    assert!(found.iter().all(|d| !d.is_error()), "{found:?}");
    assert_eq!(
        fs::read_to_string(root.join("objects/eye.noll")).unwrap(),
        "object @eye \"Eye\" {\n  data camera @eye-lens\n}\n\ncamera @eye-lens \"Eye\" {\n  lens 35mm\n}\n"
    );
    assert!(
        fs::read_dir(root.join("objects")).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".sau-migrate")),
        "no temporary file is left"
    );
    // And in the newest format there is nothing to do.
    assert_eq!(migrate::project(&manifest(&root), &now, &TEST), Ok(None));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_migration_stopped_before_the_manifest_finishes_the_second_time() {
    // The files are replaced first and the manifest last: stopped in between, the project still
    // names format 0, and migrating again changes no file but the manifest.
    let root = format_zero("stopped");
    let m = migrate::project(&manifest(&root), &sources(&root), &TEST)
        .unwrap()
        .unwrap();
    for f in &m.files {
        fs::write(root.join(&f.path), &f.text).unwrap();
    }
    let again = migrate::project(&manifest(&root), &sources(&root), &TEST)
        .unwrap()
        .unwrap();
    assert!(again.files.is_empty(), "{:?}", again.files);
    assert_eq!(again.manifest, m.manifest);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_format_no_migration_reads_is_refused_and_nothing_changes() {
    // This Sau ships no migration: format 1 is the first, so format 0 has no step.
    let root = format_zero("no-step");
    let errs = migrate::project(&manifest(&root), &sources(&root), MIGRATIONS).unwrap_err();
    assert_eq!(errs.len(), 1);
    assert_eq!(
        errs[0].to_string(),
        "error[cosmosis-migrate-no-step]: the project is written in Null format 0: no migration reads Null format 0: this Null reads format 1 and migrates no older format\n  \
         --> cosmosis.noll:2:15\n  \
         help: a project of format 0 needs a Sau that migrates it; this one reads format 1"
    );
    assert_eq!(manifest(&root), "package \"old\" {\n  null-format 0\n}\n");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_file_with_syntax_errors_stops_the_migration() {
    let root = format_zero("unparsed");
    fs::write(root.join("objects/broken.noll"), "object @broken {\n").unwrap();
    let errs = migrate::project(&manifest(&root), &sources(&root), &TEST).unwrap_err();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "cosmosis-migrate-unparsed");
    assert_eq!(
        errs[0].file.as_deref(),
        Some(Path::new("objects/broken.noll"))
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_format_is_read_where_the_manifest_writes_it() {
    assert_eq!(
        migrate::format_of("package \"p\" null-format=0").map(|(n, _)| n),
        Some(0)
    );
    assert_eq!(
        migrate::format_of("package \"p\" {\n  null-format 1\n}\n").map(|(n, _)| n),
        Some(1)
    );
    for bad in ["01", "x", "\"0\"", "-1"] {
        assert_eq!(
            migrate::format_of(&format!("package \"p\" {{ null-format {bad} }}")),
            None,
            "{bad}"
        );
    }
    // A format newer than this Sau reads is the manifest's to refuse, not a migration's.
    assert_eq!(
        migrate::project("package \"p\" { null-format 7 }", &[], &TEST),
        Ok(None)
    );
}
