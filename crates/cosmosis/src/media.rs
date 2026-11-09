//! Media files and how a datablock names one (Null, section 12; RFC 0003, Sau's
//! decision S83; Sau task 1d-05f).
//!
//! A media datablock's `filepath` is a path from the project's root (`media/<sha256>.png`), with
//! `/` separators and no `.` or `..` parts, or an absolute path to a source outside the project.
//! Blender's `//` is never written (Sau's decision S83, RFC 0003). [`check`] holds each
//! `filepath` to that, and [`content_path`] names a file in `media/` by its bytes.
//!
//! A tiled image or an image sequence is a folder of `media/` (RFC 0007, Sau's decision S96):
//! `<sha256>.tiles/` or `<sha256>.frames/`, its files named by their numbers and the folder by its
//! listing's hash ([`set_path`], [`listing_hash`]); [`check`] holds a set's `filepath` to its
//! token or frame, and the layout check the folder.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::diagnostic::{Diagnostic, Severity};
use crate::layout::{self, SourceFile};
use crate::tree::Span;

/// The kinds whose datablocks name a file in their `filepath`.
pub const MEDIA_KINDS: [&str; 6] = [
    "image",
    "sound",
    "vector-font",
    "movie-clip",
    "cache-file",
    "volume",
];

/// The SHA-256 of `bytes`, in lower-case hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A file's extension as `media/` keeps it: in lower case, or `bin` when there is none to keep.
pub fn media_ext(ext: &str) -> String {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    if !ext.is_empty() && ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        ext
    } else {
        "bin".to_string()
    }
}

/// The path in `media/` of a file of these bytes and extension: `media/<sha256>.<ext>`, the
/// extension in lower case, or `bin` when there is none to keep.
pub fn content_path(bytes: &[u8], ext: &str) -> String {
    format!(
        "{}/{}.{}",
        layout::CONTENT_FOLDERS[0],
        sha256_hex(bytes),
        media_ext(ext)
    )
}

/// The two kinds of folder `media/` holds besides its files (Null section 12; RFC 0007, Sau's
/// decision S96; task 1d-05m): a tiled image's tiles, and an image sequence's frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    /// `<sha256>.tiles/`, each tile `<number>.<ext>`, 1001 to 2000.
    Tiles,
    /// `<sha256>.frames/`, each frame `<number>.<ext>`, the numbers of one width.
    Frames,
}

impl SetKind {
    /// The folder's suffix, after its hash.
    pub fn suffix(self) -> &'static str {
        match self {
            SetKind::Tiles => "tiles",
            SetKind::Frames => "frames",
        }
    }

    /// The kind a folder of `media/` named `name` is, if it is one: `<64 hex>.tiles` or
    /// `.frames`.
    pub fn of_folder(name: &str) -> Option<(SetKind, &str)> {
        let (stem, suffix) = name.split_once('.')?;
        let hex = stem.len() == 64
            && stem
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        match suffix {
            "tiles" if hex => Some((SetKind::Tiles, stem)),
            "frames" if hex => Some((SetKind::Frames, stem)),
            _ => None,
        }
    }
}

/// The SHA-256 a tile set's or a sequence's folder is named by: of its listing, one line for
/// each file in increasing number, its name, a space, its own SHA-256 and a line feed.
/// `members` are (number, name, SHA-256 of its bytes), in any order.
pub fn listing_hash(members: &[(u64, String, String)]) -> String {
    let mut sorted: Vec<&(u64, String, String)> = members.iter().collect();
    sorted.sort_by_key(|(n, name, _)| (*n, name.clone()));
    let listing: String = sorted
        .iter()
        .map(|(_, name, hash)| format!("{name} {hash}\n"))
        .collect();
    sha256_hex(listing.as_bytes())
}

/// A set's folder in `media/` and its files' names, for files of these numbers and bytes, all of
/// extension `ext`: tiles named by their number, frames by theirs padded to `width` digits.
/// Returns the folder's path from the root and each (name, bytes).
pub fn set_path<'b>(
    kind: SetKind,
    members: &[(u64, &'b [u8])],
    ext: &str,
    width: usize,
) -> (String, Vec<(String, &'b [u8])>) {
    let ext = media_ext(ext);
    let files: Vec<(u64, String, &'b [u8])> = members
        .iter()
        .map(|(n, bytes)| {
            let name = match kind {
                SetKind::Tiles => format!("{n}.{ext}"),
                SetKind::Frames => format!("{n:0width$}.{ext}"),
            };
            (*n, name, *bytes)
        })
        .collect();
    let listed: Vec<(u64, String, String)> = files
        .iter()
        .map(|(n, name, bytes)| (*n, name.clone(), sha256_hex(bytes)))
        .collect();
    let folder = format!(
        "{}/{}.{}",
        layout::CONTENT_FOLDERS[0],
        listing_hash(&listed),
        kind.suffix()
    );
    (
        folder,
        files.into_iter().map(|(_, name, b)| (name, b)).collect(),
    )
}

/// What a `filepath` as written is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaPath {
    /// A path from the project's root, by its parts.
    Root(PathBuf),
    /// An absolute path, outside the project.
    Absolute(PathBuf),
    /// Neither: why.
    Bad(String),
}

/// `path` as written read as a media path, by its text alone (no file is looked at).
pub fn classify(path: &str) -> MediaPath {
    if path.starts_with("//") {
        return MediaPath::Bad(
            "`//` is Blender's path from a .blend file, which a project has none of".into(),
        );
    }
    if path.contains('\\') {
        return MediaPath::Bad("its separators are `/`, never `\\`".into());
    }
    if path.is_empty() {
        return MediaPath::Bad("it is empty".into());
    }
    if path.starts_with('/') {
        return MediaPath::Absolute(PathBuf::from(path));
    }
    for part in path.split('/') {
        match part {
            "" => return MediaPath::Bad("it has an empty part (`//` or a `/` at its end)".into()),
            "." | ".." => {
                return MediaPath::Bad(format!(
                    "it has a `{part}` part; write the path from the project's root"
                ));
            }
            _ => {}
        }
    }
    MediaPath::Root(PathBuf::from(path))
}

/// Each media datablock's `filepath` in `sources` (from the layout check of `root`), checked:
/// `cosmosis-media-path` for a path of neither form, or one that leaves the root through a link;
/// `cosmosis-media-missing` for a path from the root that names no file; and the warning
/// `cosmosis-media-outside` for every absolute path, which is not portable.
pub fn check(root: &Path, sources: &[SourceFile]) -> Vec<Diagnostic> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut out = Vec::new();
    for file in sources.iter().filter(|f| layout::holds_datablocks(&f.path)) {
        let parse = null_syntax::parse(&file.text);
        for node in parse.document().nodes() {
            if !MEDIA_KINDS.contains(&node.name().as_ref()) {
                continue;
            }
            for (path, span) in filepaths(node) {
                let at = |d: Diagnostic| d.in_file(&file.path).at(file.text.as_bytes(), span);
                match classify(&path) {
                    MediaPath::Bad(why) => out.push(at(Diagnostic::error(
                        "cosmosis-media-path",
                        format!("`{path}` is not a media path: {why}"),
                    )
                    .with_help("write a path from the project's root, such as `media/<sha256>.png`, or an absolute path"))),
                    MediaPath::Absolute(p) => out.push(at(Diagnostic {
                        severity: Severity::Warning,
                        ..Diagnostic::error(
                            "cosmosis-media-outside",
                            format!("`{path}` is outside the project, so it is not there on another machine{}", if p.exists() { "" } else { ", and not here either" }),
                        )
                    }
                    .with_help("copy the file into media/, named by its SHA-256, and name it from the root"))),
                    MediaPath::Root(rel) if set_line(&rel).is_some() => {
                        if let Some(d) = set_filepath(root, &rel, &path) {
                            out.push(at(d));
                        }
                    }
                    MediaPath::Root(rel) => {
                        let full = root.join(&rel);
                        match full.canonicalize() {
                            Ok(real) if !real.starts_with(&canonical_root) => out.push(at(Diagnostic::error(
                                "cosmosis-media-path",
                                format!("`{path}` leaves the project through a link, to {}", real.display()),
                            )
                            .with_help("copy the file into the project; a link to outside it is not followed"))),
                            Ok(real) if real.is_file() => {}
                            Ok(_) => out.push(at(Diagnostic::error(
                                "cosmosis-media-missing",
                                format!("`{path}` is a folder, not a file"),
                            ))),
                            Err(_) => out.push(at(Diagnostic::error(
                                "cosmosis-media-missing",
                                format!("no file `{path}` in the project"),
                            )
                            .with_help("put the file there, or write the path of one that is"))),
                        }
                    }
                }
            }
        }
    }
    out
}

/// Whether a path from the root is a tile set's or a sequence's, by its tokens or the folder it
/// is in: the set's kind under `media/`, or `None` for a tiled path elsewhere.
fn set_line(rel: &Path) -> Option<Option<SetKind>> {
    let text = rel.to_string_lossy();
    let folder = rel
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let in_media = rel
        .components()
        .next()
        .is_some_and(|c| c.as_os_str() == layout::CONTENT_FOLDERS[0]);
    if let Some((kind, _)) = SetKind::of_folder(&folder).filter(|_| in_media) {
        return Some(Some(kind));
    }
    (text.contains("<UDIM>") || text.contains("<UVTILE>")).then_some(None)
}

/// A tile set's or a sequence's `filepath` checked (RFC 0007, S96): under `media/`, a tiled path
/// names `<UDIM>.<ext>` in its `.tiles` folder (`cosmosis-media-set-token`), and a sequence's
/// names a frame of its `.frames` folder (`cosmosis-media-set-frame`); a tiled path elsewhere
/// names a folder with at least one tile (`cosmosis-media-missing`).
fn set_filepath(root: &Path, rel: &Path, written: &str) -> Option<Diagnostic> {
    let name = rel.file_name()?.to_string_lossy().into_owned();
    let dir = rel.parent().unwrap_or(Path::new(""));
    let listed: Vec<String> = std::fs::read_dir(root.join(dir))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    match set_line(rel)? {
        Some(SetKind::Tiles) => {
            let ext = name.strip_prefix("<UDIM>.");
            if written.contains("<UVTILE>") || ext.is_none() {
                return Some(
                    Diagnostic::error(
                        "cosmosis-media-set-token",
                        format!("`{written}` is a tile set's: its file is `<UDIM>.<ext>`, the tiles numbered as UDIM's"),
                    )
                    .with_help("write the set's path as `media/<sha256>.tiles/<UDIM>.<ext>`"),
                );
            }
            let ext = ext.unwrap_or_default();
            if !listed.iter().any(|f| f.ends_with(&format!(".{ext}"))) {
                return Some(Diagnostic::error(
                    "cosmosis-media-missing",
                    format!("no tile of `{written}` in the project"),
                ));
            }
            None
        }
        Some(SetKind::Frames) => {
            if listed.contains(&name) {
                None
            } else {
                Some(
                    Diagnostic::error(
                        "cosmosis-media-set-frame",
                        format!("`{written}` names no frame of its sequence's folder"),
                    )
                    .with_help("a sequence's path names one of its frames, such as its first"),
                )
            }
        }
        None => {
            let pattern = name.replace("<UDIM>", "*").replace("<UVTILE>", "*");
            let (head, tail) = pattern.split_once('*').unwrap_or((&pattern, ""));
            if listed.iter().any(|f| {
                f.starts_with(head) && f.ends_with(tail) && f.len() > head.len() + tail.len()
            }) {
                None
            } else {
                Some(Diagnostic::error(
                    "cosmosis-media-missing",
                    format!("no tile of `{written}` in the project"),
                ))
            }
        }
    }
}

/// The `filepath` values a media datablock's line and block write, with their spans.
fn filepaths(node: null_syntax::ast::Node<'_>) -> Vec<(String, Span)> {
    use null_syntax::ast::{Entry, scalar_text};
    let mut out = Vec::new();
    let mut take = |v: null_syntax::ast::Value<'_>| {
        if let Some(s) = v.as_string() {
            let sp = v.span();
            out.push((s.into_owned(), Span::new(sp.start, sp.end)));
        }
    };
    for e in node.entries() {
        if let Entry::Prop { key, value } = e
            && scalar_text(key) == "filepath"
        {
            take(value);
        }
    }
    for c in node.children().filter(|c| c.name() == "filepath") {
        if let Some(v) = c.args().next() {
            take(v);
        }
    }
    out
}
