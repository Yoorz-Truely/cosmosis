//! A project of an older Null format lifted to the format this Sau reads (Sau's decision S45,
//! task 1d-04d; Cosmósis spec, section 4).
//!
//! The manifest's `null-format` names the format a project's files are written in. When it is
//! older than [`NEWEST_NULL_FORMAT`], [`project`] runs Null's steps
//! ([`null_check::migrate`]) from that format up to the newest over every Null file the
//! project's sources hold, and writes the newest format into the manifest; [`write`] puts the
//! files that changed in place. A newer format is never guessed at: the manifest refuses it.
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
use crate::layout::{IDS_FILE, VIEW_FILE};
use crate::manifest::{MANIFEST_FILE, NEWEST_NULL_FORMAT};

/// A project's files in the newest format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Migrated {
    /// The format they were written in.
    pub from: u32,
    /// The steps run, oldest first: what each changes.
    pub steps: Vec<&'static str>,
    /// The manifest's text, its `null-format` the newest.
    pub manifest: String,
    /// Each file a step changed, with its new text, in path order.
    pub files: Vec<SourceFile>,
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

/// The project of manifest text `manifest` and `sources` (its readable `.noll` files, as the
/// layout check lists them) in the newest format, through `migrations` (Null's own,
/// [`null_check::migrate::MIGRATIONS`], outside tests): `Ok(None)` when it is in the newest
/// format already, or names no format, or a newer one, which the manifest reports. A file a step
/// cannot read, or a format no step reads, is an error, and nothing is migrated.
pub fn project(
    manifest: &str,
    sources: &[SourceFile],
    migrations: &[Migration],
) -> Result<Option<Migrated>, Vec<Diagnostic>> {
    let Some((from, span)) = format_of(manifest) else {
        return Ok(None);
    };
    if from >= NEWEST_NULL_FORMAT {
        return Ok(None);
    }
    let steps = null_check::migrate::steps(from, NEWEST_NULL_FORMAT, migrations).map_err(|e| {
        vec![
            Diagnostic::error(
                "cosmosis-migrate-no-step",
                format!("the project is written in Null format {from}: {e}"),
            )
            .in_file(MANIFEST_FILE)
            .at(manifest.as_bytes(), crate::tree::Span::new(span.start, span.end))
            .with_help(format!(
                "a project of format {from} needs a Sau that migrates it; this one reads format {NEWEST_NULL_FORMAT}"
            )),
        ]
    })?;
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for file in sources.iter().filter(|f| migrates(&f.path)) {
        match null_check::migrate::migrate(&file.text, from, NEWEST_NULL_FORMAT, migrations) {
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
    manifest_text.replace_range(span.start..span.end, &NEWEST_NULL_FORMAT.to_string());
    Ok(Some(Migrated {
        from,
        steps: steps.iter().map(|m| m.what).collect(),
        manifest: manifest_text,
        files,
    }))
}

/// `migrated` written into the project at `root`: every changed file, then the manifest last,
/// so a stop part-way leaves the old format named and a second migration, which leaves the files
/// already rewritten as they are, finishes it. Each file is written to a temporary file beside its
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
