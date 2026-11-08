//! A project of an older Null format lifted to the format this Sau reads (Sau's decision S45,
//! task 1d-04d; Cosmósis spec, section 4).
//!
//! The manifest's `null-format` names the format a project's files are written in. When it is
//! older than [`crate::manifest::NEWEST_NULL_FORMAT`], [`run`] lifts it one format at a time: [`step`] runs Null's
//! step from that format ([`null_check::migrate`]) over every Null file the project holds, and
//! [`write`] puts the files it changed in place and then the manifest, raised by one. A stop
//! part-way leaves the format that step reads named, so the next run repeats that step alone,
//! which leaves a file it already rewrote as it is, and never an earlier step. A newer format is
//! never guessed at: the manifest refuses it.
//!
//! The manifest, the handle map `.sau/ids.noll` and `view.noll` are not given the steps: their
//! forms are Cosmósis's and Sau's own (S66), not Null's datablocks.

use std::fs;
use std::path::{Path, PathBuf};

use null_check::migrate::{MigrateError, Migration};
use null_syntax::Span;
use null_syntax::ast::{Entry, Value};

use crate::diagnostic::Diagnostic;
use crate::layout::SourceFile;
use crate::layout::{self, IDS_FILE, VIEW_FILE};
use crate::manifest::MANIFEST_FILE;

/// A project's files one format on: what one step makes of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Migrated {
    /// The format they were written in.
    pub from: u32,
    /// What the step changes.
    pub what: &'static str,
    /// The manifest's text, its `null-format` raised by one.
    pub manifest: String,
    /// Each file the step changed, with its new text, in path order.
    pub files: Vec<SourceFile>,
}

/// One step written by [`run`]: the format it read, what it changes, and the paths it wrote, the
/// manifest last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// The format the step read; the project is in the next one now.
    pub from: u32,
    /// What it changes.
    pub what: &'static str,
    /// The paths written, the manifest last.
    pub written: Vec<PathBuf>,
}

/// Why [`run`] stopped.
#[derive(Debug)]
pub enum RunError {
    /// Refused before anything was written: a format no step reads, a file a step cannot read,
    /// or a layout that cannot be read whole.
    Refused(Vec<Diagnostic>),
    /// A file could not be written; the steps before it are done, and this one may be in part,
    /// which the next run finishes.
    Io {
        /// The steps written in full.
        done: Vec<Done>,
        /// What failed.
        error: std::io::Error,
    },
}

/// The format `manifest` (the text of `cosmosis.noll`) names, as a whole number, with where its
/// value is written; `None` when it names none that reads as one, which the manifest reports.
pub fn format_of(manifest: &str) -> Option<(u32, Span)> {
    let parse = null_syntax::parse(manifest);
    let package = parse.document().nodes().find(|n| n.name() == "package")?;
    let value = package
        .entries()
        .find_map(|e| match e {
            Entry::Prop { key, value } if null_syntax::ast::scalar_text(key) == "null-format" => {
                Some(value)
            }
            _ => None,
        })
        .or_else(|| {
            package
                .children()
                .find(|c| c.name() == "null-format")
                .and_then(|c| c.args().next())
        })?;
    let Value::Word(t) = value else {
        return None;
    };
    let plain =
        t.text.bytes().all(|b| b.is_ascii_digit()) && (t.text == "0" || !t.text.starts_with('0'));
    if !plain {
        return None;
    }
    Some((t.text.parse().ok()?, t.span))
}

/// Whether the file at `path` (from the root) is one of Null's datablock files, which a
/// migration rewrites.
fn migrates(path: &Path) -> bool {
    ![MANIFEST_FILE, IDS_FILE, VIEW_FILE]
        .iter()
        .any(|f| path == Path::new(f))
}

/// The error for a project of format `from` that `migrations` cannot lift to `newest`, at the
/// manifest's value; `None` when they can.
fn no_chain(
    manifest: &str,
    (from, span): (u32, Span),
    migrations: &[Migration],
    newest: u32,
) -> Option<Diagnostic> {
    let e = null_check::migrate::steps(from, newest, migrations).err()?;
    Some(
        Diagnostic::error(
            "cosmosis-migrate-no-step",
            format!("the project is written in Null format {from}: {e}"),
        )
        .in_file(MANIFEST_FILE)
        .at(manifest.as_bytes(), crate::tree::Span::new(span.start, span.end))
        .with_help(format!(
            "a project of format {from} needs a Sau that migrates it; this one reads format {newest}"
        )),
    )
}

/// The project of manifest text `manifest` and `sources` (its readable `.noll` files, as the
/// layout check lists them) one format on, by the step of `migrations` (Null's own,
/// [`null_check::migrate::MIGRATIONS`], outside tests) that reads its format: `Ok(None)` when it is
/// in format `newest` already, or names no format, or a newer one, which the manifest reports.
/// No step from its format to `newest`, or a file the step cannot read, is an error.
pub fn step(
    manifest: &str,
    sources: &[SourceFile],
    migrations: &[Migration],
    newest: u32,
) -> Result<Option<Migrated>, Vec<Diagnostic>> {
    let Some((from, span)) = format_of(manifest) else {
        return Ok(None);
    };
    if from >= newest {
        return Ok(None);
    }
    if let Some(d) = no_chain(manifest, (from, span), migrations, newest) {
        return Err(vec![d]);
    }
    let one = null_check::migrate::steps(from, from + 1, migrations).map_err(|_| Vec::new())?;
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for file in sources.iter().filter(|f| migrates(&f.path)) {
        match null_check::migrate::migrate(&file.text, from, from + 1, migrations) {
            Ok(text) if text != file.text => files.push(SourceFile {
                path: file.path.clone(),
                text,
            }),
            Ok(_) => {}
            Err(e @ MigrateError::Unparsed) => errors.push(
                Diagnostic::error("cosmosis-migrate-unparsed", e.to_string())
                    .in_file(&file.path)
                    .with_help("`sau check` names its syntax errors"),
            ),
            Err(e) => errors.push(
                Diagnostic::error("cosmosis-migrate-no-step", e.to_string()).in_file(&file.path),
            ),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut manifest_text = manifest.to_string();
    manifest_text.replace_range(span.start..span.end, &(from + 1).to_string());
    Ok(Some(Migrated {
        from,
        what: one[0].what,
        manifest: manifest_text,
        files,
    }))
}

/// The project at `root` lifted to format `newest` by `migrations`, one format at a time, each
/// step's files written and then the manifest (see the module's notes): the steps written, none
/// when it is in that format already. The whole chain is checked before the first write, so a
/// format no step reads changes nothing.
pub fn run(root: &Path, migrations: &[Migration], newest: u32) -> Result<Vec<Done>, RunError> {
    let mut done = Vec::new();
    loop {
        let found = layout::check(root, None);
        let unreadable: Vec<Diagnostic> = found
            .diagnostics
            .iter()
            .filter(|d| d.code == "cosmosis-io" || d.code == "cosmosis-not-utf8")
            .cloned()
            .collect();
        if !unreadable.is_empty() {
            return Err(RunError::Refused(unreadable));
        }
        let manifest =
            fs::read_to_string(root.join(MANIFEST_FILE)).map_err(|error| RunError::Io {
                done: done.clone(),
                error,
            })?;
        let migrated = match step(&manifest, &found.sources, migrations, newest) {
            Ok(Some(m)) => m,
            Ok(None) => return Ok(done),
            Err(d) => return Err(RunError::Refused(d)),
        };
        match write(root, &migrated) {
            Ok(written) => done.push(Done {
                from: migrated.from,
                what: migrated.what,
                written,
            }),
            Err(error) => return Err(RunError::Io { done, error }),
        }
    }
}

/// One step's `migrated` written into the project at `root`: every changed file, then the
/// manifest last, so a stop part-way leaves the format the step reads named, and the step run
/// again, which leaves the files it already rewrote as they are, finishes it. Each file is written to a temporary file beside its
/// target first (through a link, as write-back writes), and if any cannot be, nothing is
/// replaced. The paths written, the manifest last.
pub fn write(root: &Path, migrated: &Migrated) -> std::io::Result<Vec<PathBuf>> {
    let mut all: Vec<(PathBuf, &str)> = migrated
        .files
        .iter()
        .map(|f| (f.path.clone(), f.text.as_str()))
        .collect();
    all.push((PathBuf::from(MANIFEST_FILE), migrated.manifest.as_str()));
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (rel, text) in &all {
        // Through a link to its target, as write-back and `sau rename` write (1a-14).
        let staged_one = fs::canonicalize(root.join(rel)).and_then(|target| {
            let mut name = target.file_name().unwrap_or_default().to_os_string();
            name.push(".sau-migrate");
            let temporary = target.with_file_name(name);
            let written =
                fs::write(&temporary, text).and_then(|()| fs::File::open(&temporary)?.sync_all());
            if let Err(e) = written {
                let _ = fs::remove_file(&temporary);
                return Err(e);
            }
            Ok((temporary, target))
        });
        match staged_one {
            Ok(one) => staged.push(one),
            Err(e) => {
                for (t, _) in &staged {
                    let _ = fs::remove_file(t);
                }
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("cannot write {}: {e}", rel.display()),
                ));
            }
        }
    }
    let mut done = Vec::new();
    for (i, ((temporary, target), (rel, _))) in staged.iter().zip(&all).enumerate() {
        if let Err(e) = fs::rename(temporary, target) {
            for (t, _) in &staged[i..] {
                let _ = fs::remove_file(t);
            }
            return Err(std::io::Error::new(
                e.kind(),
                format!("cannot replace {}: {e}", rel.display()),
            ));
        }
        done.push(rel.clone());
    }
    Ok(done)
}
