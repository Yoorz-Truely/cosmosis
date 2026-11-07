//! Diagnostics: what `sau check` reports. Each names its file, line and column where it has
//! one, says what was expected, offers a fix where it can, and carries a stable code that
//! tools and AI can act on (Null spec, section 18; Cosmósis's codes start `cosmosis-`).

use std::fmt;
use std::path::PathBuf;

use crate::tree::Span;

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The project is wrong; `sau check` fails.
    Error,
    /// Information only; `sau check` still passes.
    Note,
}

/// A line and column, both counted from 1; the column counts characters, not bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    /// The line.
    pub line: usize,
    /// The column.
    pub column: usize,
}

impl LineCol {
    /// The line and column of byte `offset` in `source`, which need not be valid UTF-8.
    pub fn of(source: &[u8], offset: usize) -> Self {
        let before = &source[..offset.min(source.len())];
        let line_start = before
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        Self {
            line: before.iter().filter(|&&b| b == b'\n').count() + 1,
            // Every byte but UTF-8 continuation bytes starts a character.
            column: before[line_start..]
                .iter()
                .filter(|&&b| b & 0xC0 != 0x80)
                .count()
                + 1,
        }
    }
}

/// One finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Error or note.
    pub severity: Severity,
    /// The stable code, such as `cosmosis-manifest-unknown-field`.
    pub code: &'static str,
    /// What is wrong, in one sentence.
    pub message: String,
    /// The file, relative to the project root when inside it.
    pub file: Option<PathBuf>,
    /// Where in the file, when known.
    pub position: Option<LineCol>,
    /// How to fix it.
    pub help: Option<String>,
}

impl Diagnostic {
    /// An error with no file.
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            file: None,
            position: None,
            help: None,
        }
    }

    /// A note with no file.
    pub fn note(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Note,
            ..Self::error(code, message)
        }
    }

    /// The same diagnostic about `file`.
    pub fn in_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// The same diagnostic at `span` in `source`; hand-made spans (`0..0`) give no position.
    pub fn at(mut self, source: &[u8], span: Span) -> Self {
        if span != Span::default() {
            self.position = Some(LineCol::of(source, span.start));
        }
        self
    }

    /// The same diagnostic with a fix.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// True for errors.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    /// `error[code]: message`, then `  --> file:line:column` and `  help: …` when known.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Note => "note",
        };
        write!(f, "{level}[{}]: {}", self.code, self.message)?;
        if let Some(file) = &self.file {
            write!(f, "\n  --> {}", file.display())?;
            if let Some(p) = self.position {
                write!(f, ":{}:{}", p.line, p.column)?;
            }
        }
        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }
        Ok(())
    }
}

/// The closest of `candidates` to `word`, if it is close enough to be a likely misspelling:
/// at most one edit for short words, and a third of the word's length for longer ones.
pub fn nearest<'a>(word: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let limit = (word.chars().count() / 3).max(1);
    candidates
        .iter()
        .map(|c| (edit_distance(word, c), *c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Optimal string alignment distance: insertions, deletions, substitutions and swaps of two
/// neighbouring characters each cost one.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    //! Tests of positions, rendering and suggestions.
    use super::*;

    #[test]
    fn line_and_column_count_characters() {
        let src = "ab\nçd é\n".as_bytes();
        assert_eq!(LineCol::of(src, 0), LineCol { line: 1, column: 1 });
        assert_eq!(LineCol::of(src, 3), LineCol { line: 2, column: 1 });
        // "çd " is four bytes but three characters.
        assert_eq!(LineCol::of(src, 7), LineCol { line: 2, column: 4 });
    }

    #[test]
    fn renders_code_file_position_and_help() {
        let d = Diagnostic::error("cosmosis-x", "something is wrong")
            .in_file("cosmosis.noll")
            .at(b"a\nbc", Span::new(3, 4))
            .with_help("fix it");
        assert_eq!(
            d.to_string(),
            "error[cosmosis-x]: something is wrong\n  --> cosmosis.noll:2:2\n  help: fix it"
        );
    }

    #[test]
    fn suggests_only_near_names() {
        let fields = ["name", "kind", "version", "null-format", "license"];
        assert_eq!(nearest("null_format", &fields), Some("null-format"));
        assert_eq!(nearest("nullformat", &fields), Some("null-format"));
        assert_eq!(nearest("kidn", &fields), Some("kind"));
        assert_eq!(nearest("colour", &fields), None);
    }
}
