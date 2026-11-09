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
//! separate project. The one exception is `.sau/ids.noll`, the map of handles to persistent IDs,
//! which is source and read with the other Null files (Sau's decisions S61 (2) and S66).
//! `settings.noll`, `view.noll` and `.sau/ids.noll` are read for their syntax only
//! ([`holds_datablocks`]).

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
/// The handle-to-ID map (Sau's decisions S61 (2) and S66): the one source file in the cache
/// folder `.sau/`, kept in Git; its form is Sau's task 1b-01f's.
pub const IDS_FILE: &str = ".sau/ids.noll";
/// The root's names that must be files.
pub const ROOT_FILES: [&str; 5] = [
    MANIFEST_FILE,
    LOCK_FILE,
    PROJECT_FILE,
    VIEW_FILE,
    SETTINGS_FILE,
];
/// Whether the Null file at `path` (from the root) defines datablocks, so Null's checker reads
/// it against Blender's schema. The manifest is Cosmósis's own; `settings.noll`, `view.noll`
/// and `.sau/ids.noll` are Null text whose forms are their own and not yet specified, so only
/// their syntax is checked (Sau's decision S66).
pub fn holds_datablocks(path: &Path) -> bool {
    ![MANIFEST_FILE, SETTINGS_FILE, VIEW_FILE, IDS_FILE]
        .iter()
        .any(|special| path == Path::new(special))
}

/// Folders whose files are named by their contents.
pub const CONTENT_FOLDERS: [&str; 2] = ["media", "data"];
/// Output folders at the root that are not source.
pub const OUTPUT_FOLDERS: [&str; 1] = ["renders"];

/// Whether `name` is an editor's or the system's own file, not the project's (task C-05): hidden
/// or temporary, so a project ignores it even when it ends in `.noll`. Editors save by writing a
/// temporary file and renaming it over the original, and a check that reads the temporary file
/// before the rename would see every datablock in it defined twice. The names (Cosmósis spec,
/// section 2, "Ignored names"):
///
/// - starting with `.` (hidden: `.DS_Store`, macOS's `._x`, Vim's `.x.swp`, Emacs's `.#x`) or
///   `~` (`~$x`, `~x`);
/// - ending in `~` (backups) or `#` (Emacs's `#x#`);
/// - ending in `.tmp`, `.temp`, `.bak`, `.swp`, `.swo`, `.swx`, `.crswap` or `.vsctmp` (VS
///   Code's atomic save);
/// - holding `.sb-` (macOS's safe save, `x.sb-1a2b3c4d-AbCdEf`), `___jb_` (JetBrains' safe
///   write) or `.noll.` (a copy of a Null file under another name, such as `x.noll.saving.noll`);
/// - wholly in parentheses: the folder macOS saves through, `(A Document Being Saved By TextEdit)`
///   in English and localised in other languages, so it is matched by its shape, not its words.
pub fn is_ignored(name: &str) -> bool {
    const ENDINGS: [&str; 8] = [
        ".tmp", ".temp", ".bak", ".swp", ".swo", ".swx", ".crswap", ".vsctmp",
    ];
    const INSIDE: [&str; 3] = [".sb-", "___jb_", ".noll."];
    let lower = name.to_ascii_lowercase();
    name.starts_with('.')
        || name.starts_with('~')
        || name.ends_with('~')
        || name.ends_with('#')
        || ENDINGS.iter().any(|e| lower.ends_with(e))
        || INSIDE.iter().any(|s| lower.contains(s))
        || (name.len() > 2 && name.starts_with('(') && name.ends_with(')'))
}

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
/// `cosmosis.noll` (S37). `start` may be a folder or a file inside the project. The error is
/// boxed, as a diagnostic is large.
pub fn find_root(start: &Path) -> Result<PathBuf, Box<Diagnostic>> {
    let full = start.canonicalize().map_err(|e| {
        Box::new(Diagnostic::error(
            "cosmosis-no-such-path",
            format!("cannot open {}: {e}", start.display()),
        ))
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
            Box::new(
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
                )),
            )
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
    // Each media datablock's `filepath` (Null section 12, S83, task 1d-05f).
    let media = crate::media::check(root, &layout.sources);
    layout.diagnostics.extend(media);
    // A file in `media/` or `data/` that a line names as its data file has the place error
    // (`cosmosis-data-file-place`, task C-10), not also one saying to rename it by its hash.
    let misplaced = crate::data_files::check(&layout.sources).misplaced;
    layout.diagnostics.retain(|d| {
        !(matches!(
            d.code,
            "cosmosis-layout-content-name" | "cosmosis-layout-content-hash"
        ) && d.file.as_ref().is_some_and(|f| misplaced.contains(f)))
    });
    // `.sau/` is a cache and skipped as hidden, but for the ID map, which is source (S66). A
    // `.sau` that is a link to a folder is not followed, as no link to a folder is: the map read
    // through it could be another project's.
    let cache = root.join(".sau");
    if cache
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        layout.diagnostics.push(
            Diagnostic::note(
                "cosmosis-layout-link",
                format!("a link to a folder is not followed, so {IDS_FILE} is not read"),
            )
            .in_file(".sau"),
        );
    } else {
        let ids = root.join(IDS_FILE);
        if ids.symlink_metadata().is_ok() {
            match fs::metadata(&ids) {
                // A link to nothing: as anywhere else, an error; here the more so, since losing
                // the map gives every datablock a new identity on the next load (S61 (2)).
                Err(_) => layout.diagnostics.push(
                    Diagnostic::error("cosmosis-io", "cannot read the file: it is a broken link")
                        .in_file(IDS_FILE),
                ),
                Ok(m) if m.is_dir() => layout.diagnostics.push(
                    Diagnostic::error(
                        "cosmosis-layout-not-a-file",
                        format!("{IDS_FILE} is a folder, not a file"),
                    )
                    .in_file(IDS_FILE)
                    .with_help(format!(
                        "{IDS_FILE} is the project's map of handles to persistent IDs, a Null file; rename the folder"
                    )),
                ),
                Ok(_) => source(&ids, PathBuf::from(IDS_FILE), &mut layout),
            }
        }
    }
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
        // A folder gone since its parent was listed (task C-04).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !rel.as_os_str().is_empty() => {
            Vec::new()
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
        // An editor's or the system's file, even one ending in `.noll` (task C-05).
        if is_ignored(&name) {
            continue;
        }
        let path = entry.path();
        let rel_path = rel.join(&name);
        let at_root = rel.as_os_str().is_empty();
        // Symbolic links are followed to files only, so a link can never make a loop.
        let Ok(meta) = fs::metadata(&path) else {
            // Gone since the listing: an editor saved by renaming a temporary file over the
            // original (task C-04). Only an entry that is still there is a broken link.
            if fs::symlink_metadata(&path).is_ok() {
                layout.diagnostics.push(
                    Diagnostic::error("cosmosis-io", "cannot read the file: it is a broken link")
                        .in_file(&rel_path),
                );
            }
            continue;
        };
        let is_link = entry.file_type().is_ok_and(|t| t.is_symlink());
        if meta.is_dir() {
            if at_root && OUTPUT_FOLDERS.contains(&name.as_str()) {
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
        // Gone since the listing, as a temporary file renamed over another is (task C-04).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
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
        if is_ignored(&name) {
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
        let hash = match sha256(&path) {
            Ok(h) => h,
            // Gone since the listing (task C-04).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                layout.content_files += 1;
                layout.diagnostics.push(
                    Diagnostic::error("cosmosis-io", format!("cannot read the file: {e}"))
                        .in_file(&rel_path),
                );
                continue;
            }
        };
        layout.content_files += 1;
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
