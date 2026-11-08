//! Checking a project's Null files against Blender's schema with Null's checker (Sau task 1a-15).
//!
//! The schema is Null's `schema/null-schema-project.json`, the schema of all project data that
//! Sau's build generates from RNA (Sau tasks 1b-03b and 1b-03e; the slice's `null-schema.json`
//! until Sau task C-06), compiled in from Null's repository by path. Every file is checked together, so a
//! handle defined in one file resolves in another (Null spec, section 7). `cosmosis.noll` is
//! Cosmósis's manifest, read by [`crate::manifest`], and is left out, as are the files whose
//! forms are their own ([`holds_datablocks`]: `settings.noll`, `view.noll`, `.sau/ids.noll`).

use std::sync::OnceLock;

use null_check::Schema;
use null_syntax::{FileId, Severity as NullSeverity};

use crate::diagnostic::{Diagnostic, Severity};
use crate::layout::{SourceFile, holds_datablocks};
use crate::tree::Span;

/// The schema's text, as Null holds it.
const SCHEMA: &str = include_str!("../../../../Null/schema/null-schema-project.json");

/// The schema, read once.
pub fn schema() -> &'static Schema {
    static S: OnceLock<Schema> = OnceLock::new();
    S.get_or_init(|| {
        Schema::from_json(SCHEMA).expect("Null's schema/null-schema-project.json reads")
    })
}

/// Null's checker over `sources`, the project's Null files but the manifest, as Cosmósis's
/// diagnostics. Files with syntax errors are still checked, as far as their tree goes.
pub fn check(sources: &[SourceFile]) -> Vec<Diagnostic> {
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
    null_check::check(schema(), &ids)
        .into_iter()
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
        .collect()
}
