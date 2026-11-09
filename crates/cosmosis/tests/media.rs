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

#[test]
fn a_tile_set_and_a_sequence_are_folders_named_by_their_listing() {
    // Null section 12, RFC 0007 (Sau's decision S96, task 1d-05m): a tiled image's tiles in
    // `media/<sha256>.tiles/` and a sequence's frames in `media/<sha256>.frames/`, each folder
    // named by the SHA-256 of its listing; their paths check clean.
    use cosmosis::media::{SetKind, listing_hash, set_path, sha256_hex};
    let (tiles, tile_files) = set_path(SetKind::Tiles, &[(1002, b"b"), (1001, b"a")], "PNG", 4);
    let names: Vec<&str> = tile_files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["1002.png", "1001.png"]);
    let listing = format!(
        "1001.png {}\n1002.png {}\n",
        sha256_hex(b"a"),
        sha256_hex(b"b")
    );
    assert_eq!(
        tiles,
        format!("media/{}.tiles", sha256_hex(listing.as_bytes()))
    );
    assert_eq!(
        listing_hash(&[
            (1002, "1002.png".into(), sha256_hex(b"b")),
            (1001, "1001.png".into(), sha256_hex(b"a")),
        ]),
        sha256_hex(listing.as_bytes())
    );
    let (frames, frame_files) = set_path(SetKind::Frames, &[(1, b"x"), (2, b"y")], "png", 4);
    assert_eq!(frame_files[0].0, "0001.png");
    let images = format!(
        "image @wood \"wood\" {{\n  filepath \"{tiles}/<UDIM>.png\"\n  source tiled\n}}\nimage @shot \"shot\" {{\n  filepath \"{frames}/0001.png\"\n  source sequence\n}}\n"
    );
    let root = project("sets", &images);
    for (folder, files) in [(&tiles, &tile_files), (&frames, &frame_files)] {
        fs::create_dir_all(root.join(folder)).unwrap();
        for (name, bytes) in files {
            fs::write(root.join(folder).join(name), bytes).unwrap();
        }
    }
    assert_eq!(found(&root), vec![]);
    let all = layout::check(&root, None);
    assert!(
        all.diagnostics
            .iter()
            .all(|d| !d.code.starts_with("cosmosis-layout")),
        "{:#?}",
        all.diagnostics
    );
}

#[test]
fn each_mistake_of_a_set_has_its_own_code() {
    // RFC 0007 with Sau's decision S97: a code for each mistake. The folder's hash, a file not
    // `<number>.<ext>`, two extensions, a tile out of UDIM's range; at the `filepath` line, a
    // tiled path without `<UDIM>` or with `<UVTILE>`, and a sequence's path naming no frame.
    use cosmosis::media::{SetKind, set_path};
    let (tiles, tile_files) = set_path(SetKind::Tiles, &[(1001, b"a")], "png", 4);
    let (frames, frame_files) = set_path(SetKind::Frames, &[(1, b"x")], "png", 4);
    let images = format!(
        "image @a \"a\" {{\n  filepath \"{tiles}/1001.png\"\n}}\nimage @b \"b\" {{\n  filepath \"{tiles}/<UVTILE>.png\"\n}}\nimage @c \"c\" {{\n  filepath \"{frames}/0009.png\"\n}}\n"
    );
    let root = project("set-errors", &images);
    for (folder, files) in [(&tiles, &tile_files), (&frames, &frame_files)] {
        fs::create_dir_all(root.join(folder)).unwrap();
        for (name, bytes) in files {
            fs::write(root.join(folder).join(name), bytes).unwrap();
        }
    }
    let wrong = root.join(format!("media/{}.tiles", "0".repeat(64)));
    fs::create_dir_all(&wrong).unwrap();
    fs::write(wrong.join("1001.png"), "a").unwrap();
    fs::write(wrong.join("999.png"), "z").unwrap();
    fs::write(wrong.join("1002.jpg"), "j").unwrap();
    fs::write(wrong.join("notes.txt"), "n").unwrap();
    // A tile named with a leading zero, which Blender's `<UDIM>` never reads (1d-05m's review).
    let (zeroed, _) = set_path(SetKind::Tiles, &[(1001, b"a")], "png", 4);
    let zeroed = root
        .join(&zeroed)
        .with_file_name(format!("{}.tiles", "1".repeat(64)));
    fs::create_dir_all(&zeroed).unwrap();
    fs::write(zeroed.join("01001.png"), "a").unwrap();
    let codes: Vec<String> = layout::check(&root, None)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .filter(|c| c.starts_with("cosmosis-media-set"))
        .collect();
    for code in [
        "cosmosis-media-set-token",
        "cosmosis-media-set-frame",
        "cosmosis-media-set-name",
        "cosmosis-media-set-tile",
        "cosmosis-media-set-mixed",
    ] {
        assert!(codes.iter().any(|c| c == code), "{code}: {codes:?}");
    }
    assert_eq!(
        codes
            .iter()
            .filter(|c| *c == "cosmosis-media-set-token")
            .count(),
        2,
        "{codes:?}"
    );
    let named_in = |folder: &str| {
        layout::check(&root, None)
            .diagnostics
            .iter()
            .any(|d| d.code == "cosmosis-media-set-name" && d.to_string().contains(folder))
    };
    assert!(
        named_in(&format!("{}.tiles/01001.png", "1".repeat(64))),
        "{codes:?}"
    );
    // A folder otherwise well made but named by another listing.
    let renamed = root.join(format!("media/{}.frames", "a".repeat(64)));
    fs::create_dir_all(&renamed).unwrap();
    fs::write(renamed.join("0001.png"), "x").unwrap();
    let codes: Vec<String> = layout::check(&root, None)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "cosmosis-media-set-hash"),
        "{codes:?}"
    );
}
