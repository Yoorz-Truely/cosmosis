//! Reading a Null file with Null's parser (`null_syntax`) into Cosmósis's parser-neutral tree
//! ([`crate::tree`]), with Null's own diagnostics (Sau task 1a-11).
//!
//! The parser recovers from errors and keeps every byte; its diagnostics (`null-syntax`,
//! `null-name`) come through with their file, line, column and fix. The tree is built from
//! the live nodes: nodes commented out with `/-` are left out, a value the parser found
//! missing adds nothing (the parser has reported it), and the keyword attributes `at`, `to`
//! and `len` become the two arguments they are in Null (Null's `AGENTS.md`).

use std::path::Path;

use null_syntax::ast::{self, Entry as AstEntry, Value as AstValue};
use null_syntax::{FileId, Severity as NullSeverity};

use crate::diagnostic::{Diagnostic, Severity};
use crate::tree::{Entry, Node, Property, Span, Value, ValueKind};

/// A file read by Null's parser.
#[derive(Debug)]
pub struct Parsed {
    /// Its live top-level nodes.
    pub nodes: Vec<Node>,
    /// Null's diagnostics for it, in text order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Parsed {
    /// True when Null found no error in the file.
    pub fn is_clean(&self) -> bool {
        !self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Parses `text`, the file at `path` (relative to the project root, for messages).
pub fn parse(text: &str, path: &Path) -> Parsed {
    let parse = null_syntax::parse(text);
    let diagnostics = null_syntax::diagnostics(&parse, FileId(0))
        .into_iter()
        .map(|d| {
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
            .in_file(path)
            .at(text.as_bytes(), span);
            if let Some(fix) = d.fixes.first() {
                out = out.with_help(fix.message.clone());
            } else if let Some(note) = d.notes.first() {
                out = out.with_help(note.clone());
            }
            // A span at the very start of the file is still a position.
            if span.start == 0 && out.position.is_none() {
                out.position = Some(crate::diagnostic::LineCol { line: 1, column: 1 });
            }
            out
        })
        .collect();
    let nodes = parse.document().nodes().map(node).collect();
    Parsed { nodes, diagnostics }
}

fn span(s: null_syntax::Span) -> Span {
    Span::new(s.start, s.end)
}

fn node(n: ast::Node<'_>) -> Node {
    let (name, name_span) = match n.name_token() {
        Some(t) => (n.name().into_owned(), span(t.span)),
        None => (String::new(), span(n.span())),
    };
    let mut entries = Vec::new();
    for e in n.entries() {
        match e {
            AstEntry::Arg(v) => entries.extend(value(v).map(Entry::Argument)),
            AstEntry::Prop { key, value: v } => {
                if let Some(v) = value(v) {
                    entries.push(Entry::Property(Property {
                        key: ast::scalar_text(key).into_owned(),
                        key_span: span(key.span),
                        value: v,
                    }));
                }
            }
            AstEntry::Keyword { key, value: v } => {
                entries.push(Entry::Argument(Value::word(key.text).at(span(key.span))));
                entries.extend(value(v).map(Entry::Argument));
            }
        }
    }
    let children = n.block().map(|_| n.children().map(node).collect());
    Node {
        name,
        name_span,
        entries,
        children,
        span: span(n.span()),
    }
}

fn value(v: AstValue<'_>) -> Option<Value> {
    let kind = match v {
        AstValue::Word(t) => ValueKind::Word(t.text.to_owned()),
        AstValue::Str(_) | AstValue::Raw(_) => ValueKind::Str(v.as_string()?.into_owned()),
        AstValue::List(_) => ValueKind::List(v.items().into_iter().filter_map(value).collect()),
        AstValue::Missing(_) => return None,
    };
    Some(Value {
        kind,
        span: span(v.span()),
    })
}

#[cfg(test)]
mod tests {
    //! Tests of the conversion from Null's tree.
    use super::*;

    fn read(text: &str) -> Parsed {
        parse(text, Path::new("cosmosis.noll"))
    }

    #[test]
    fn a_manifest_becomes_the_tree_cosmosis_reads() {
        let text = "package \"harbor\" kind=project {\n  null-format 1\n  deps [a \"b\" [c]]\n}\n";
        let p = read(text);
        assert!(p.is_clean(), "{:?}", p.diagnostics);
        let expected = Node::new("package")
            .arg(Value::string("harbor"))
            .prop("kind", Value::word("project"))
            .child(Node::new("null-format").arg(Value::word("1")))
            .child(Node::new("deps").arg(Value::list(vec![
                Value::word("a"),
                Value::string("b"),
                Value::list(vec![Value::word("c")]),
            ])));
        assert_eq!(strip(&p.nodes), vec![expected]);
        // Spans point into the text.
        let pkg = &p.nodes[0];
        assert_eq!(&text[pkg.name_span.start..pkg.name_span.end], "package");
        let fmt = &pkg.children.as_ref().unwrap()[0];
        assert_eq!(&text[fmt.name_span.start..fmt.name_span.end], "null-format");
    }

    #[test]
    fn commented_out_nodes_and_keywords() {
        let p = read("/- package \"old\"\npackage \"new\" { constraint to @target }\n");
        assert_eq!(p.nodes.len(), 1);
        let constraint = &p.nodes[0].children.as_ref().unwrap()[0];
        let args: Vec<_> = constraint.arguments().map(Value::describe).collect();
        assert_eq!(args, ["to", "@target"]);
    }

    #[test]
    fn nulls_diagnostics_come_through_with_code_position_and_fix() {
        let p = read("package \"x\" {\n  null_format 1\n");
        let rendered: Vec<String> = p.diagnostics.iter().map(ToString::to_string).collect();
        assert!(!p.is_clean());
        assert!(
            rendered
                .iter()
                .any(|d| d.starts_with("error[null-syntax]: ")),
            "{rendered:?}"
        );
        assert!(
            rendered.iter().any(|d| d
                .starts_with("error[null-name]: `null_format` is not kebab-case")
                && d.contains("--> cosmosis.noll:2:3")
                && d.contains("help: write `null-format`")),
            "{rendered:?}"
        );
    }

    /// The tree with every span cleared, to compare with hand-made trees.
    fn strip(nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .map(|n| Node {
                name: n.name.clone(),
                name_span: Span::default(),
                entries: n
                    .entries
                    .iter()
                    .map(|e| match e {
                        Entry::Argument(v) => Entry::Argument(strip_value(v)),
                        Entry::Property(p) => Entry::Property(Property {
                            key: p.key.clone(),
                            key_span: Span::default(),
                            value: strip_value(&p.value),
                        }),
                    })
                    .collect(),
                children: n.children.as_deref().map(strip),
                span: Span::default(),
            })
            .collect()
    }

    fn strip_value(v: &Value) -> Value {
        let kind = match &v.kind {
            ValueKind::List(items) => ValueKind::List(items.iter().map(strip_value).collect()),
            k => k.clone(),
        };
        Value {
            kind,
            span: Span::default(),
        }
    }
}
