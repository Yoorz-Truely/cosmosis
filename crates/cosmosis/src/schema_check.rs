//! Checking a project's Null files against Blender's schema with Null's checker (Sau task 1a-15).
//!
//! The schema is Null's `schema/null-schema-project.json`, the schema of all project data that
//! Sau's build generates from RNA (Sau tasks 1b-03b and 1b-03e; the slice's `null-schema.json`
//! until Sau task C-06), compiled in from Null's repository by path. Every file is checked together, so a
//! handle defined in one file resolves in another (Null spec, section 7). `cosmosis.noll` is
//! Cosmósis's manifest, read by [`crate::manifest`], and is left out, as are the files whose
//! forms are their own ([`holds_datablocks`]: `settings.noll`, `view.noll`, `.sau/ids.noll`).
//!
//! With it goes Null's raw table, `schema/null-raw.json` (task C-11), so a datablock's `raw` block
//! of DNA fields is checked against DNA: an unknown field, a value its type cannot hold, a list
//! of another length (Null spec, section 19). While Null holds only the table's placeholder, the
//! blocks are not checked and a note says so.

use std::sync::OnceLock;

use null_check::Schema;
use null_check::raw::RawTable;
use null_syntax::{FileId, Severity as NullSeverity};

use crate::diagnostic::{Diagnostic, Severity};
use crate::layout::{SourceFile, holds_datablocks};
use crate::tree::Span;

/// The schema's text, as Null holds it.
const SCHEMA: &str = include_str!("../../../../Null/schema/null-schema-project.json");

/// The raw table's text, as Null holds it: the DNA fields each struct's `raw` block may write
/// (Null spec, section 19; Sau's task 1d-05b).
const RAW: &str = include_str!("../../../../Null/schema/null-raw.json");

/// The raw table (task C-11), or `None` while Null holds its placeholder, which lists no struct
/// until a build of Sau writes the table (`just schema-sync`). Read as a table, the placeholder
/// would refuse every raw field, so until then `raw` blocks are not checked against DNA and
/// `sau check` says so ([`raw_unchecked`]).
pub fn raw_table() -> Option<&'static RawTable> {
    static T: OnceLock<Option<RawTable>> = OnceLock::new();
    T.get_or_init(|| {
        let table = RawTable::from_json(RAW).expect("Null's schema/null-raw.json reads");
        (!table.structs.is_empty()).then_some(table)
    })
    .as_ref()
}

/// The schema, read once, with the raw table where Null holds one ([`raw_table`]).
pub fn schema() -> &'static Schema {
    static S: OnceLock<Schema> = OnceLock::new();
    S.get_or_init(|| {
        let schema =
            Schema::from_json(SCHEMA).expect("Null's schema/null-schema-project.json reads");
        match raw_table() {
            Some(raw) => schema.with_raw(raw.clone()),
            None => schema,
        }
    })
}

/// A note, at the first `raw` block of the project, when the schema has no raw table to check
/// such blocks against (task C-11); `None` when there is a table or no `raw` block.
pub fn raw_unchecked(schema: &Schema, sources: &[SourceFile]) -> Option<Diagnostic> {
    if schema.raw.is_some() {
        return None;
    }
    for f in sources.iter().filter(|f| holds_datablocks(&f.path)) {
        let parse = null_syntax::parse(&f.text);
        let raw = parse
            .document()
            .nodes()
            .flat_map(|n| n.children())
            .find(|c| c.name() == "raw");
        if let Some(r) = raw {
            let at = r.name_token().map_or(r.span(), |t| t.span);
            return Some(
                Diagnostic::note(
                    "cosmosis-raw-unchecked",
                    "`raw` blocks are not checked against DNA: Null's raw table \
                     (`schema/null-raw.json`) is a placeholder until a build of Sau writes it",
                )
                .in_file(&f.path)
                .at(f.text.as_bytes(), Span::new(at.start, at.end))
                .with_help("`just schema-sync` in Sau, after a build, writes the table"),
            );
        }
    }
    None
}

/// Null's checker over `sources`, the project's Null files but the manifest, as Cosmósis's
/// diagnostics. Files with syntax errors are still checked, as far as their tree goes.
pub fn check(sources: &[SourceFile]) -> Vec<Diagnostic> {
    check_with(schema(), sources)
}

/// [`check`] against `schema`, such as one with a raw table of a test's own.
pub fn check_with(schema: &Schema, sources: &[SourceFile]) -> Vec<Diagnostic> {
    let files: Vec<&SourceFile> = sources
        .iter()
        .filter(|f| holds_datablocks(&f.path))
        .collect();
    let parses: Vec<_> = files.iter().map(|f| null_syntax::parse(&f.text)).collect();
    let ids: Vec<(FileId, &null_syntax::Parse)> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (FileId(i as u32), p))
        .collect();
    // Data files against the lines that name them, by path (task C-10), in place of Null's
    // matching by header.
    let data = crate::data_files::check(sources);
    let replaced = |d: &null_syntax::Diagnostic| {
        d.code == null_check::codes::HANDLE
            && data.replaced.iter().any(|(path, span)| {
                *path == files[d.primary.file.0 as usize].path
                    && span.start == d.primary.span.start
                    && span.end == d.primary.span.end
            })
    };
    let mut out: Vec<Diagnostic> = null_check::check(schema, &ids)
        .into_iter()
        .filter(|d| !replaced(d))
        .map(|d| {
            let file = files[d.primary.file.0 as usize];
            let severity = match d.severity {
                NullSeverity::Error => Severity::Error,
                NullSeverity::Warning => Severity::Warning,
                NullSeverity::Note => Severity::Note,
            };
            let span = Span::new(d.primary.span.start, d.primary.span.end);
            let mut out = Diagnostic {
                severity,
                ..Diagnostic::error(d.code, d.message)
            }
            .in_file(&file.path)
            .at(file.text.as_bytes(), span);
            if out.position.is_none() {
                out.position = Some(crate::diagnostic::LineCol::of(
                    file.text.as_bytes(),
                    span.start,
                ));
            }
            // The other places it names, such as where a handle was first defined, so a handle
            // defined twice names both files (Cosmósis spec, section 2).
            for label in &d.secondary {
                if let Some(other) = files.get(label.location.file.0 as usize) {
                    let span = Span::new(label.location.span.start, label.location.span.end);
                    out = out.with_related(
                        &other.path,
                        other.text.as_bytes(),
                        span,
                        label.message.clone(),
                    );
                }
            }
            if let Some(fix) = d.fixes.first() {
                out = out.with_help(fix.message.clone());
            } else if let Some(note) = d.notes.first() {
                out = out.with_help(note.clone());
            }
            out
        })
        .collect();
    out.extend(data.diagnostics);
    out.extend(raw_unchecked(schema, sources));
    out
}
