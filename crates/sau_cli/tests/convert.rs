//! `sau convert <scene.blend> <Scene.sau>` (task 1d-05d) as a program, without Sau: its usage, and
//! what it refuses before running Sau. A stand-in for Sau, a script that writes a project as
//! Sau's `--convert` does, shows the folder is then checked. The conversion itself is Sau's
//! (`sau_core::convert`, model tests), and every corpus `.blend` converted by the built Sau is the
//! `convert` gate's, in the batch build.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn sau(args: &[&str], app: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_sau"));
    c.args(args).env_remove("SAU_APP");
    if let Some(app) = app {
        c.env("SAU_APP", app);
    }
    c.output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn folder(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "convert-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stand-in for Sau in `dir`: `-b <blend> --convert <folder>` writes a small project there,
/// with `object` as the object file's text.
fn stand_in(dir: &Path, object: &str) -> PathBuf {
    let app = dir.join("fake-sau.sh");
    fs::write(
        &app,
        format!(
            "#!/bin/sh\n[ \"$1\" = -b ] && [ \"$3\" = --convert ] || exit 9\nout=$4\nmkdir -p \"$out/scenes\" \"$out/objects\"\n\
             printf 'package \"scene\" {{\\n  null-format 1\\n}}\\n' >\"$out/cosmosis.noll\"\n\
             printf 'project \"Scene\" {{\\n  scene @scene\\n}}\\n' >\"$out/project.noll\"\n\
             printf 'scene @scene \"Scene\" {{\\n  object @cube\\n}}\\n' >\"$out/scenes/scene.noll\"\n\
             printf '{object}' >\"$out/objects/cube.noll\"\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&app, fs::Permissions::from_mode(0o755)).unwrap();
    }
    app
}

#[test]
fn convert_has_help_and_takes_a_blend_and_a_folder() {
    let out = sau(&["convert", "--help"], None);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).starts_with("Usage: sau convert <scene.blend> <Scene.sau>\n"));
    let out = sau(&["convert", "scene.blend"], None);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("takes a .blend file and a project folder"));
    let out = sau(&["convert", "scene.txt", "Out.sau"], None);
    assert_eq!(out.status.code(), Some(2));
    let help = text(&sau(&["--help"], None).stdout);
    assert!(
        help.contains("  convert  Convert a .blend to a project and back\n"),
        "{help}"
    );
}

#[test]
fn convert_refuses_a_folder_in_use_and_needs_the_app() {
    let dir = folder("refuse");
    let blend = dir.join("scene.blend");
    fs::write(&blend, b"BLENDER").unwrap();
    let used = dir.join("Used.sau");
    fs::create_dir_all(&used).unwrap();
    fs::write(used.join("keep.txt"), "mine").unwrap();
    let app = stand_in(&dir, "object @cube \"Cube\"\\n");
    let out = sau(
        &["convert", blend.to_str().unwrap(), used.to_str().unwrap()],
        Some(&app),
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("is not an empty folder"));
    assert_eq!(fs::read_dir(&used).unwrap().count(), 1);
    let fresh = dir.join("Fresh.sau");
    let out = sau(
        &["convert", blend.to_str().unwrap(), fresh.to_str().unwrap()],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("set SAU_APP"));
    assert!(!fresh.exists());
    let out = sau(
        &[
            "convert",
            dir.join("none.blend").to_str().unwrap(),
            fresh.to_str().unwrap(),
        ],
        Some(&app),
    );
    assert_eq!(out.status.code(), Some(1));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_folder_sau_writes_is_checked() {
    let dir = folder("checked");
    let blend = dir.join("scene.blend");
    fs::write(&blend, b"BLENDER").unwrap();
    let good = stand_in(&dir, "object @cube \"Cube\"\\n");
    let out_dir = dir.join("Scene.sau");
    let out = sau(
        &[
            "convert",
            blend.to_str().unwrap(),
            out_dir.to_str().unwrap(),
        ],
        Some(&good),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("package \"scene\""),
        "{}",
        text(&out.stdout)
    );
    // A folder that does not check fails the command.
    let bad_dir = dir.join("bad");
    fs::create_dir_all(&bad_dir).unwrap();
    let bad = stand_in(&bad_dir, "object @cube \"Cube\" {\\n  locaton 1\\n}\\n");
    let out_bad = dir.join("Bad.sau");
    let out = sau(
        &[
            "convert",
            blend.to_str().unwrap(),
            out_bad.to_str().unwrap(),
        ],
        Some(&bad),
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("null-unknown-property"),
        "{}",
        text(&out.stderr)
    );
    let _ = fs::remove_dir_all(&dir);
}
