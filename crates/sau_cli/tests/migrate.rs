//! `sau migrate` (task 1d-04d, Sau's S45) as a program. This Sau ships no migration, since Null's
//! format 1 is the first: a project of format 1 has nothing to migrate, one of a later format is
//! refused naming the format this Sau reads, and one of format 0 is refused as no migration reads
//! it, with nothing written. The migration itself is Cosmósis's (`cosmosis/tests/migrate.rs`).

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

/// A project of Null format `format` with one object.
fn project(name: &str, format: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "migrate-cli-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("objects")).unwrap();
    fs::write(
        root.join("cosmosis.noll"),
        format!("package \"harbor\" {{\n  null-format {format}\n}}\n"),
    )
    .unwrap();
    fs::write(root.join("project.noll"), "project \"Harbor\" {}\n").unwrap();
    fs::write(root.join("objects/pier.noll"), "object @pier \"Pier\" {}\n").unwrap();
    root
}

#[test]
fn a_project_of_this_format_has_nothing_to_migrate() {
    let root = project("same", "1");
    let out = sau(&["migrate", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(
        text(&out.stdout),
        "sau migrate: package \"harbor\" is in Null format 1, the one this Sau reads; nothing to migrate\n"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_later_format_is_refused_naming_the_format_this_sau_reads() {
    let root = project("newer", "2");
    let out = sau(&["migrate", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        text(&out.stderr),
        "error[cosmosis-manifest-newer-format]: the project is written in Null format 2, newer than this Sau reads (format 1)\n  \
         --> cosmosis.noll:2:15\n  \
         help: open it with a newer Sau: older formats are migrated, newer ones cannot be\n\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("cosmosis.noll")).unwrap(),
        "package \"harbor\" {\n  null-format 2\n}\n"
    );
    // `sau check` refuses it the same way.
    let out = sau(&["check", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("newer than this Sau reads (format 1)"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_format_no_migration_reads_is_refused_and_nothing_is_written() {
    let root = project("zero", "0");
    let out = sau(&["migrate", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        text(&out.stderr),
        "error[cosmosis-migrate-no-step]: the project is written in Null format 0: no migration reads Null format 0: this Null reads format 1 and migrates no older format\n  \
         --> cosmosis.noll:2:15\n  \
         help: a project of format 0 needs a Sau that migrates it; this one reads format 1\n\n\
         sau migrate: nothing was changed\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("cosmosis.noll")).unwrap(),
        "package \"harbor\" {\n  null-format 0\n}\n"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn migrate_has_help_and_takes_one_path() {
    let out = sau(&["migrate", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).starts_with("Usage: sau migrate [path]\n"));
    let out = sau(&["migrate", "a", "b"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("`sau migrate` takes one path; `b` is a second"));
    let help = text(&sau(&["--help"]).stdout);
    assert!(
        help.contains("  migrate  Rewrite a project of an older Null format in this one\n"),
        "{help}"
    );
}
