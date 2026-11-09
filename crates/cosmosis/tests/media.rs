//! A media datablock's `filepath` (Null, section 12; RFC 0003, Sau's decision S83; task 1d-05f):
//! a path from the project's root or an absolute one, checked by the layout check.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cosmosis::layout;
use cosmosis::media::{MediaPath, classify, content_path};

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn project(name: &str, images: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "media-{}-{name}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("media")).unwrap();
    fs::create_dir_all(root.join("materials")).unwrap();
    fs::write(
        root.join("cosmosis.noll"),
        "package \"p\" {\n  null-format 1\n}\n",
    )
    .unwrap();
    fs::write(root.join("project.noll"), "project \"P\" {}\n").unwrap();
    fs::write(root.join(format!("media/{ABC_SHA256}.png")), "abc").unwrap();
    fs::write(root.join("materials/images.noll"), images).unwrap();
    root
}

/// The media diagnostics of the layout check, as (code, severity word, line).
fn found(root: &Path) -> Vec<(String, String, usize)> {
    layout::check(root, None)
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("cosmosis-media-"))
        .map(|d| {
            let s = d.to_string();
            let severity = s.split('[').next().unwrap_or("").to_string();
            (
                d.code.to_string(),
                severity,
                d.position.map_or(0, |p| p.line),
            )
        })
        .collect()
}

#[test]
fn a_file_in_media_is_named_by_its_bytes() {
    assert_eq!(
        content_path(b"abc", "PNG"),
        format!("media/{ABC_SHA256}.png")
    );
    assert_eq!(
        content_path(b"abc", ".wav"),
        format!("media/{ABC_SHA256}.wav")
    );
    assert_eq!(content_path(b"abc", ""), format!("media/{ABC_SHA256}.bin"));
    assert_eq!(
        content_path(b"abc", "t.gz"),
        format!("media/{ABC_SHA256}.bin")
    );
}

#[test]
fn a_path_reads_as_from_the_root_absolute_or_neither() {
    assert_eq!(
        classify("media/x.png"),
        MediaPath::Root(PathBuf::from("media/x.png"))
    );
    assert_eq!(
        classify("/Volumes/hdri/dusk.exr"),
        MediaPath::Absolute(PathBuf::from("/Volumes/hdri/dusk.exr"))
    );
    for bad in [
        "//x.png",
        "media\\x.png",
        "./media/x.png",
        "media/../x.png",
        "../x.png",
        "media/",
        "",
    ] {
        assert!(matches!(classify(bad), MediaPath::Bad(_)), "{bad}");
    }
}

#[test]
fn every_media_path_is_checked_where_it_is_written() {
    let images = format!(
        "image @ok \"ok\" {{\n  filepath \"media/{ABC_SHA256}.png\"\n}}\n\
         image @gone \"gone\" {{\n  filepath \"media/none.png\"\n}}\n\
         image @blend \"blend\" {{\n  filepath \"//textures/oak.png\"\n}}\n\
         image @up \"up\" {{\n  filepath \"../oak.png\"\n}}\n\
         sound @far \"far\" filepath=\"/no/such/surf.wav\"\n\
         image @there \"there\" {{\n  filepath \"{}\"\n}}\n",
        std::env::temp_dir().display()
    );
    let root = project("checked", &images);
    let got = found(&root);
    assert_eq!(
        got,
        [
            ("cosmosis-media-missing".into(), "error".into(), 5),
            ("cosmosis-media-path".into(), "error".into(), 8),
            ("cosmosis-media-path".into(), "error".into(), 11),
            ("cosmosis-media-outside".into(), "warning".into(), 13),
            ("cosmosis-media-outside".into(), "warning".into(), 15),
        ],
        "{got:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn a_path_that_leaves_the_root_through_a_link_is_an_error() {
    let root = project(
        "link",
        "image @linked \"linked\" {\n  filepath \"media/outside.png\"\n}\n",
    );
    let outside = root.with_extension("outside.png");
    fs::write(&outside, "elsewhere").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("media/outside.png")).unwrap();
    let got = found(&root);
    assert_eq!(
        got,
        [("cosmosis-media-path".into(), "error".into(), 2)],
        "{got:?}"
    );
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&outside);
}
