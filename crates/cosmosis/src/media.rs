//! Media files and how a datablock names one (Null, section 12; RFC 0003, Sau's
//! decision S83; Sau task 1d-05f).
//!
//! A media datablock's `filepath` is a path from the project's root (`media/<sha256>.png`), with
//! `/` separators and no `.` or `..` parts, or an absolute path to a source outside the project.
//! Blender's `//` is never written (Sau's decision S83, RFC 0003). [`check`] holds each
//! `filepath` to that, and [`content_path`] names a file in `media/` by its bytes.

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

/// The path in `media/` of a file of these bytes and extension: `media/<sha256>.<ext>`, the
/// extension in lower case, or `bin` when there is none to keep.
pub fn content_path(bytes: &[u8], ext: &str) -> String {
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let ext = if !ext.is_empty() && ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        ext
    } else {
        "bin".to_string()
    };
    format!("{}/{hash}.{ext}", layout::CONTENT_FOLDERS[0])
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
