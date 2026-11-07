//! The project folder (Cosmósis spec, section 2; Sau's decisions S36 and S37): finding a
//! project's root and checking what its layout fixes before any file is parsed.
//!
//! The layout is free (S36): folders such as `objects/` and `materials/` are conventions, not
//! rules. What is fixed, and checked here:
//!
//! - the root is the nearest folder, from the path given upwards, holding `cosmosis.noll`;
//! - `cosmosis.noll`, `cosmosis.lock`, `project.noll`, `view.noll` and `settings.noll` at the
//!   root are files, not folders, and a `project` package has `project.noll`;
//! - every `.noll` file is readable UTF-8 text (Null spec, section 2);
//! - `media/` and `data/` hold only files named by the SHA-256 of their contents, as
//!   `<sha256>.<ext>` (Null spec, sections 9 and 12).
//!
//! Folders whose names start with `.` (`.sau/`, the cache; `.git/`) and `renders/` are not
//! source and are skipped, as is a folder holding a `cosmosis.noll` of its own, which is a
//! separate project.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::diagnostic::Diagnostic;
use crate::manifest::{Kind, MANIFEST_FILE};
use crate::tree::Span;

/// The lockfile.
pub const LOCK_FILE: &str = "cosmosis.lock";
/// Project settings, needed by a `project` package.
pub const PROJECT_FILE: &str = "project.noll";
/// Where the project was left; not in Git.
pub const VIEW_FILE: &str = "view.noll";
/// The project's own settings.
pub const SETTINGS_FILE: &str = "settings.noll";
/// The root's names that must be files.
pub const ROOT_FILES: [&str; 5] = [
    MANIFEST_FILE,
    LOCK_FILE,
    PROJECT_FILE,
    VIEW_FILE,
    SETTINGS_FILE,
];
/// Folders whose files are named by their contents.
pub const CONTENT_FOLDERS: [&str; 2] = ["media", "data"];
/// Output folders at the root that are not source.
pub const OUTPUT_FOLDERS: [&str; 1] = ["renders"];

/// A `.noll` file that is readable UTF-8 text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// Its path from the root, such as `objects/pier.noll`.
    pub path: PathBuf,
    /// Its text.
    pub text: String,
}

/// What checking a project's layout found.
#[derive(Debug)]
pub struct Layout {
    /// The project's root.
    pub root: PathBuf,
    /// Every readable `.noll` file, in path order.
    pub sources: Vec<SourceFile>,
    /// How many files `media/` and `data/` hold.
    pub content_files: usize,
    /// Errors and notes, in path order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Layout {
    /// How many errors were found.
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }
}

/// The project holding `start`: the nearest folder, from `start` upwards, that holds
/// `cosmosis.noll` (S37). `start` may be a folder or a file inside the project.
pub fn find_root(start: &Path) -> Result<PathBuf, Diagnostic> {
    let full = start.canonicalize().map_err(|e| {
        Diagnostic::error(
            "cosmosis-no-such-path",
            format!("cannot open {}: {e}", start.display()),
        )
    })?;
    let dir = if full.is_dir() {
        full.as_path()
    } else {
        full.parent().unwrap_or(&full)
    };
    dir.ancestors()
        .find(|d| d.join(MANIFEST_FILE).symlink_metadata().is_ok())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            Diagnostic::error(
                "cosmosis-no-project",
                format!(
                    "no {MANIFEST_FILE} in {} or any folder above it",
                    dir.display()
                ),
            )
            .with_help(format!(
                "a project is a folder with {MANIFEST_FILE} at its root, such as \
                 `package \"my-project\" {{ null-format 1 }}`"
            ))
        })
}

/// Checks the layout of the project at `root`. `kind` is the manifest's kind once it has been
/// read; until then (`None`) nothing that depends on it is checked.
pub fn check(root: &Path, kind: Option<Kind>) -> Layout {
    let mut layout = Layout {
        root: root.to_path_buf(),
        sources: Vec::new(),
        content_files: 0,
        diagnostics: Vec::new(),
    };
    for name in ROOT_FILES {
        if root.join(name).is_dir() {
            layout.diagnostics.push(
                Diagnostic::error(
                    "cosmosis-layout-not-a-file",
                    format!("{name} is a folder, not a file"),
                )
                .in_file(name)
                .with_help(format!(
                    "{name} at a project's root is a Null file; rename the folder"
                )),
            );
        }
    }
    if kind == Some(Kind::Project) && !root.join(PROJECT_FILE).exists() {
        layout.diagnostics.push(
            Diagnostic::error("cosmosis-layout-no-project-file", format!("the project has no {PROJECT_FILE}"))
                .with_help(format!(
                    "a package of kind project keeps its settings and scenes in {PROJECT_FILE} at its root"
                )),
        );
    }
    walk(root, Path::new(""), &mut layout);
    layout
}

/// The entries of `dir`, sorted by name; unreadable folders are reported.
fn entries(dir: &Path, rel: &Path, layout: &mut Layout) -> Vec<fs::DirEntry> {
    match fs::read_dir(dir) {
        Ok(rd) => {
            let mut v: Vec<fs::DirEntry> = rd.filter_map(Result::ok).collect();
            v.sort_by_key(fs::DirEntry::file_name);
            v
        }
        Err(e) => {
            layout.diagnostics.push(
                Diagnostic::error("cosmosis-io", format!("cannot read the folder: {e}"))
                    .in_file(rel),
            );
            Vec::new()
        }
    }
}

fn walk(dir: &Path, rel: &Path, layout: &mut Layout) {
    for entry in entries(dir, rel, layout) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let rel_path = rel.join(&name);
        let at_root = rel.as_os_str().is_empty();
        // Symbolic links are followed to files only, so a link can never make a loop.
        let Ok(meta) = fs::metadata(&path) else {
            layout.diagnostics.push(
                Diagnostic::error("cosmosis-io", "cannot read the file: it is a broken link")
                    .in_file(&rel_path),
            );
            continue;
        };
        let is_link = entry.file_type().is_ok_and(|t| t.is_symlink());
        if meta.is_dir() {
            if name.starts_with('.') || (at_root && OUTPUT_FOLDERS.contains(&name.as_str())) {
                continue;
            }
            if at_root && ROOT_FILES.contains(&name.as_str()) {
                continue; // Reported by `check`.
            }
            if is_link {
                layout.diagnostics.push(
                    Diagnostic::note("cosmosis-layout-link", "a link to a folder is not followed")
                        .in_file(&rel_path),
                );
            } else if path.join(MANIFEST_FILE).exists() {
                layout.diagnostics.push(
                    Diagnostic::note(
                        "cosmosis-layout-nested-project",
                        "skipped: this folder is a project of its own",
                    )
                    .in_file(&rel_path),
                );
            } else if at_root && CONTENT_FOLDERS.contains(&name.as_str()) {
                content(&path, &rel_path, layout);
            } else {
                walk(&path, &rel_path, layout);
            }
        } else if path.extension().is_some_and(|e| e == "noll") {
            source(&path, rel_path, layout);
        }
    }
}

/// Reads a `.noll` file, which must be UTF-8 text.
fn source(path: &Path, rel: PathBuf, layout: &mut Layout) {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            layout.diagnostics.push(
                Diagnostic::error("cosmosis-io", format!("cannot read the file: {e}")).in_file(rel),
            );
            return;
        }
    };
    match String::from_utf8(bytes) {
        Ok(text) => layout.sources.push(SourceFile { path: rel, text }),
        Err(e) => {
            let at = e.utf8_error().valid_up_to();
            let bytes = e.into_bytes();
            layout.diagnostics.push(
                Diagnostic::error(
                    "cosmosis-not-utf8",
                    "the file is not UTF-8 text from here on",
                )
                .in_file(rel)
                .at(&bytes, Span::new(at, at + 1))
                .with_help("Null files are plain UTF-8 text: save it as UTF-8"),
            );
        }
    }
}

/// Checks a content-addressed folder: every file named `<sha256 of its contents>.<ext>`.
fn content(dir: &Path, rel: &Path, layout: &mut Layout) {
    for entry in entries(dir, rel, layout) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let rel_path = rel.join(&name);
        let path = entry.path();
        if path.is_dir() {
            layout.diagnostics.push(
                Diagnostic::error(
                    "cosmosis-layout-content-folder",
                    format!(
                        "{} holds files named by their contents, not folders",
                        rel.display()
                    ),
                )
                .in_file(&rel_path)
                .with_help("move the folder's files into it, each named by its SHA-256"),
            );
            continue;
        }
        layout.content_files += 1;
        let hash = match sha256(&path) {
            Ok(h) => h,
            Err(e) => {
                layout.diagnostics.push(
                    Diagnostic::error("cosmosis-io", format!("cannot read the file: {e}"))
                        .in_file(&rel_path),
                );
                continue;
            }
        };
        let (stem, ext) = name.split_once('.').unwrap_or((&name, ""));
        let named = stem.len() == 64
            && stem
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        let renamed = |ext: &str| {
            if ext.is_empty() {
                format!(
                    "rename it to {}/{hash}.<ext>, with the file's own extension",
                    rel.display()
                )
            } else {
                format!("rename it to {}/{hash}.{ext}", rel.display())
            }
        };
        if !named || ext.is_empty() {
            layout.diagnostics.push(
                Diagnostic::error(
                    "cosmosis-layout-content-name",
                    format!(
                        "files in {} are named by the SHA-256 of their contents, as <sha256>.<ext>",
                        rel.display()
                    ),
                )
                .in_file(&rel_path)
                .with_help(renamed(ext)),
            );
        } else if stem != hash {
            layout.diagnostics.push(
                Diagnostic::error(
                    "cosmosis-layout-content-hash",
                    format!("the file's contents have the SHA-256 {hash}, not the one in its name"),
                )
                .in_file(&rel_path)
                .with_help(renamed(ext)),
            );
        }
    }
}

/// The SHA-256 of a file's contents, in lower-case hexadecimal.
fn sha256(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
