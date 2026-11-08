//! Level 0 data files in the layout (Null spec, section 9, "Larger arrays in a Null text file of
//! their own"; Sau's decision S74 (2); task C-10).
//!
//! A mesh's array too large to write inline is a Null file of its own beside the mesh's file,
//! one element per line, holding one `array` node: `array mesh @rock vertices [ … ]`. The line
//! that names it gives its path from the naming file's folder (`vertices file="rock.vertices.noll"
//! count=12480 bounds=…`). Null's checker holds the file to the line's `count=` and `bounds=`,
//! matching the two by the `array` node's owner and array; whether the path names that file is
//! Cosmósis's, which knows the files' paths. Here:
//!
//! - every `file=` is resolved from the folder of the file that names it, and a path that leaves
//!   the project, names no file, or names a file that is no data file is an error at the line;
//! - the data file's `array` node must name the line's owner and array;
//! - a data file is named by one line, and a data file no line names is an error;
//! - a data file is never read as a datablock file, so a file of one `array` node is never
//!   "a datablock file Sau does not know" (it is the line's), and `sau fmt` writes it one element
//!   per line ([`layout_of`]).
//!
//! These replace Null's own matching errors (`null-handle` at the line's name and at the data
//! file's node): Null matches by the header, so where the path and the header disagree Null's
//! message would name the wrong cause.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use null_syntax::ast::{Entry, Node, Value};
use null_syntax::names::to_kebab_case;

use crate::diagnostic::Diagnostic;
use crate::layout::{CONTENT_FOLDERS, SourceFile, holds_datablocks};
use crate::tree::Span;

/// The node a data file holds.
pub const ARRAY: &str = "array";

/// The lines whose array is the line's own name; the others (`attribute`, `uv-map`) are named by
/// their handle.
const NAMED_BY_LINE: [&str; 3] = ["vertices", "edges", "polygons"];

/// Whether `text` is a data file: its one live node is `array`.
pub fn is_data_file(text: &str) -> bool {
    let parse = null_syntax::parse(text);
    let mut nodes = parse.document().nodes().filter(|n| !n.is_commented_out());
    matches!((nodes.next(), nodes.next()), (Some(n), None) if n.name() == ARRAY)
}

/// A data file's header: the owner's kind and handle, and the array.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Header {
    kind: String,
    handle: String,
    array: String,
}

impl Header {
    fn show(&self) -> String {
        format!("array {} {} {}", self.kind, self.handle, self.array)
    }
}

/// A line that names a data file.
struct Line {
    /// The file it is in, and the span of its `file=` value there.
    from: PathBuf,
    value: Span,
    /// The span of its name, where Null reports its own matching errors.
    name: Span,
    /// The path as written, and resolved from the root (`None` when it leaves the project).
    written: String,
    target: Option<PathBuf>,
    header: Header,
}

/// What the data files of a project come to.
#[derive(Debug, Default)]
pub struct Checked {
    /// The problems, in the order of the files and lines they concern.
    pub diagnostics: Vec<Diagnostic>,
    /// Where Null's own matching errors (`null-handle`) are replaced by these: a naming line's
    /// name and a data file's `array` node, each with its file.
    pub replaced: Vec<(PathBuf, Span)>,
}

/// `path`, written in the file at `from`, as a path from the root; `None` when it leaves it.
fn resolve(from: &Path, path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in from
        .parent()
        .unwrap_or(Path::new(""))
        .join(path)
        .components()
    {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

/// `to` written as a path from the folder of `from`, as a `file=` in `from` would name it.
fn relative(from: &Path, to: &Path) -> String {
    let base: Vec<_> = from
        .parent()
        .unwrap_or(Path::new(""))
        .components()
        .collect();
    let target: Vec<_> = to.components().collect();
    let common = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut out: Vec<String> = vec!["..".to_string(); base.len() - common];
    out.extend(
        target[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    out.join("/")
}

fn span(s: null_syntax::Span) -> Span {
    Span::new(s.start, s.end)
}

/// The lines naming data files in one datablock file.
fn lines_in(file: &SourceFile, out: &mut Vec<Line>) {
    let parse = null_syntax::parse(&file.text);
    for top in parse.document().nodes() {
        let Some(handle) = handle_of(top) else {
            continue;
        };
        let owner = (top.name().into_owned(), handle);
        walk(file, top, &owner, out);
    }
}

/// The handle a datablock's line defines, as Null's checker reads it: written, or `@` and the
/// kebab-case form of its display name (Sau's decision S18; `mesh "Rock"` is `@rock`).
fn handle_of(node: Node<'_>) -> Option<String> {
    if let Some(h) = node.handle() {
        return Some(h.text.to_string());
    }
    let name = node.args().find_map(|v| match v {
        Value::Str(_) | Value::Raw(_) => v.as_string(),
        _ => None,
    })?;
    Some(format!("@{}", to_kebab_case(&name)?))
}

fn walk(file: &SourceFile, node: Node<'_>, owner: &(String, String), out: &mut Vec<Line>) {
    for c in node.children() {
        let named = c.entries().find_map(|e| match e {
            Entry::Prop { key, value } if key.text == "file" => Some(value),
            _ => None,
        });
        match named {
            Some(v @ (Value::Str(_) | Value::Raw(_))) => {
                let written = v.as_string().unwrap_or_default().into_owned();
                let line = c.name().into_owned();
                let array = if NAMED_BY_LINE.contains(&line.as_str()) {
                    line
                } else {
                    c.handle().map_or(line, |h| h.text.to_string())
                };
                out.push(Line {
                    from: file.path.clone(),
                    value: span(v.span()),
                    name: c.name_token().map_or(span(c.span()), |t| span(t.span)),
                    target: resolve(&file.path, &written),
                    written,
                    header: Header {
                        kind: owner.0.clone(),
                        handle: owner.1.clone(),
                        array,
                    },
                });
            }
            // Not a string: Null's checker says so.
            Some(_) => {}
            None => walk(file, c, owner, out),
        }
    }
}

/// A data file's header and the span of its `array` node, if it reads.
fn header_of(text: &str) -> Option<(Header, Span)> {
    let parse = null_syntax::parse(text);
    let node = parse.document().nodes().find(|n| n.name() == ARRAY)?;
    let words: Vec<String> = node
        .args()
        .filter_map(|v| v.as_word().map(str::to_string))
        .collect();
    match words.as_slice() {
        [kind, handle, array, ..] => Some((
            Header {
                kind: kind.clone(),
                handle: handle.clone(),
                array: array.clone(),
            },
            span(node.code_span()),
        )),
        _ => None,
    }
}

/// Checks the project's data files against the lines that name them (module docs).
pub fn check(sources: &[SourceFile]) -> Checked {
    let mut checked = Checked::default();
    let by_path: HashMap<&Path, &SourceFile> =
        sources.iter().map(|f| (f.path.as_path(), f)).collect();
    let data: Vec<&SourceFile> = sources
        .iter()
        .filter(|f| holds_datablocks(&f.path) && is_data_file(&f.text))
        .collect();
    let mut lines = Vec::new();
    for f in sources
        .iter()
        .filter(|f| holds_datablocks(&f.path) && !is_data_file(&f.text))
    {
        lines_in(f, &mut lines);
    }
    let headers: HashMap<&Path, (Header, Span)> = data
        .iter()
        .filter_map(|f| header_of(&f.text).map(|h| (f.path.as_path(), h)))
        .collect();
    for f in &data {
        if let Some((_, s)) = headers.get(f.path.as_path()) {
            checked.replaced.push((f.path.clone(), *s));
        }
    }
    // Which line first named each data file, and the data files a missing one's help points
    // to, which are that mistake's, not one of their own.
    let mut named: HashMap<PathBuf, usize> = HashMap::new();
    let mut pointed: Vec<PathBuf> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        checked.replaced.push((line.from.clone(), line.name));
        let source = by_path[line.from.as_path()].text.as_bytes();
        let at = |code, message: String| {
            Diagnostic::error(code, message)
                .in_file(&line.from)
                .at(source, line.value)
        };
        let Some(target) = &line.target else {
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-outside",
                    format!("`file=\"{}\"` leaves the project's folder", line.written),
                )
                .with_help(
                    "a data file is in the project, its path written from the folder of the file that names it",
                ),
            );
            continue;
        };
        // Where the line names the wrong file, the file that holds its array, if any, is named
        // in the help, and is not reported again as a file no line names.
        let elsewhere = data
            .iter()
            .find(|d| {
                d.path != *target
                    && headers
                        .get(d.path.as_path())
                        .is_some_and(|(h, _)| *h == line.header)
            })
            .map(|d| d.path.clone());
        let instead = |help: String| match &elsewhere {
            Some(d) => format!(
                "{} holds it: write `file=\"{}\"`",
                d.display(),
                relative(&line.from, d)
            ),
            None => help,
        };
        if target
            .components()
            .next()
            .is_some_and(|c| CONTENT_FOLDERS.iter().any(|f| c.as_os_str() == *f))
        {
            let folder = target
                .components()
                .next()
                .unwrap()
                .as_os_str()
                .to_string_lossy();
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-place",
                    format!(
                        "`file=\"{}\"` names a file in {folder}/, which holds files named by their contents; a data file is a Null file named after its mesh and array",
                        line.written
                    ),
                )
                .with_help(instead(format!(
                    "move it beside {}, as {}.{}.noll, and name that",
                    line.from.display(),
                    line.header.handle.trim_start_matches('@'),
                    line.header.array.trim_start_matches('@')
                ))),
            );
            if let Some(d) = &elsewhere {
                pointed.push(d.clone());
            }
            continue;
        }
        let Some(file) = by_path.get(target.as_path()) else {
            if let Some(d) = &elsewhere {
                pointed.push(d.clone());
            }
            let help = instead(format!(
                "the path is from {}/; a data file there holds `{} [ … ]`, one element per line",
                line.from.parent().unwrap_or(Path::new("")).display(),
                line.header.show()
            ));
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-missing",
                    format!(
                        "no file {}: `file=\"{}\"` names it for `{}` of {} {}",
                        target.display(),
                        line.written,
                        line.header.array,
                        line.header.kind,
                        line.header.handle
                    ),
                )
                .with_help(help),
            );
            continue;
        };
        let is_data = data.iter().any(|d| d.path == *target);
        let Some((header, header_span)) = headers.get(target.as_path()) else {
            if is_data {
                // An `array` node of the wrong shape: Null's checker says how.
                named.entry(target.clone()).or_insert(i);
                continue;
            }
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-not-data",
                    format!(
                        "{} is not a data file: a data file holds one `array` node",
                        target.display()
                    ),
                )
                .with_help(format!(
                    "write `{} [ … ]` in a file of its own, one element per line, and name that",
                    line.header.show()
                )),
            );
            continue;
        };
        if let Some(&first) = named.get(target) {
            if let Some(d) = &elsewhere {
                pointed.push(d.clone());
            }
            let other = &lines[first];
            let other_source = by_path[other.from.as_path()].text.as_bytes();
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-twice",
                    format!(
                        "{} is named by two lines; a data file holds one array",
                        target.display()
                    ),
                )
                .with_related(&other.from, other_source, other.value, "first named here")
                .with_help(instead(format!(
                    "give `{}` of {} {} a data file of its own",
                    line.header.array, line.header.kind, line.header.handle
                ))),
            );
            continue;
        }
        named.insert(target.clone(), i);
        if *header != line.header {
            if let Some(d) = &elsewhere {
                pointed.push(d.clone());
            }
            checked.diagnostics.push(
                at(
                    "cosmosis-data-file-header",
                    format!(
                        "{} holds `{}`, not `{}` of {} {}",
                        target.display(),
                        header.show(),
                        line.header.array,
                        line.header.kind,
                        line.header.handle
                    ),
                )
                .with_related(
                    &file.path,
                    file.text.as_bytes(),
                    *header_span,
                    "its `array` node",
                )
                .with_help(instead(format!(
                    "the data file's first line names its owner and array: `{} [`",
                    line.header.show()
                ))),
            );
        }
    }
    // A data file no line names is read by nothing.
    for f in &data {
        if named.contains_key(&f.path) || pointed.contains(&f.path) {
            continue;
        }
        let mut d = Diagnostic::error(
            "cosmosis-data-file-unnamed",
            format!(
                "no line names {}: a data file is read only through the `file=` of its line",
                f.path.display()
            ),
        )
        .in_file(&f.path);
        if let Some((h, s)) = headers.get(f.path.as_path()) {
            d = d.at(f.text.as_bytes(), *s).with_help(format!(
                "name it from the `{}` line of {} {}: `file=\"{}\"` from that line's folder, or delete the file",
                h.array,
                h.kind,
                h.handle,
                f.path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        checked.diagnostics.push(d);
    }
    checked
}

/// A data file's text as `sau fmt` writes it: its `array` line, then one element per line,
/// indented two spaces, each written with single spaces, then `]`. `None` when the file is not
/// one `array` node with its list last, or holds a comment, which the layout would move.
pub fn layout_of(text: &str) -> Option<String> {
    let parse = null_syntax::parse(text);
    let comments = null_syntax::lex(text).0.iter().any(|t| t.kind.is_comment());
    if !parse.errors().is_empty() || comments {
        return None;
    }
    let mut nodes = parse.document().nodes();
    let node = nodes.next().filter(|n| n.name() == ARRAY)?;
    if nodes.next().is_some() || node.block().is_some() {
        return None;
    }
    let entries: Vec<Entry<'_>> = node.entries().collect();
    let (Entry::Arg(list @ Value::List(_)), head) = entries.split_last()? else {
        return None;
    };
    let mut out = String::from(ARRAY);
    for e in head {
        let Entry::Arg(Value::Word(w)) = e else {
            return None;
        };
        out.push(' ');
        out.push_str(w.text);
    }
    out.push_str(" [\n");
    for item in list.items() {
        out.push_str("  ");
        out.push_str(&element(&item));
        out.push('\n');
    }
    out.push_str("]\n");
    Some(out)
}

/// An element with single spaces between its items.
fn element(v: &Value<'_>) -> String {
    match v {
        Value::List(_) => {
            let items: Vec<String> = v.items().iter().map(element).collect();
            format!("[{}]", items.join(" "))
        }
        other => other.text().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_resolve_from_the_naming_file_s_folder() {
        let from = Path::new("meshes/rock.noll");
        assert_eq!(
            resolve(from, "rock.vertices.noll"),
            Some(PathBuf::from("meshes/rock.vertices.noll"))
        );
        assert_eq!(
            resolve(from, "../data/rock.noll"),
            Some(PathBuf::from("data/rock.noll"))
        );
        assert_eq!(
            resolve(from, "./a/../b.noll"),
            Some(PathBuf::from("meshes/b.noll"))
        );
        assert_eq!(resolve(from, "../../x.noll"), None);
        assert_eq!(resolve(from, "/etc/x.noll"), None);
        assert_eq!(
            relative(from, Path::new("data/rock.vertices.noll")),
            "../data/rock.vertices.noll"
        );
        assert_eq!(relative(from, Path::new("meshes/v.noll")), "v.noll");
    }

    #[test]
    fn a_data_file_is_one_array_node() {
        assert!(is_data_file(
            "array mesh @rock vertices [\n  [0m 0m 0m]\n]\n"
        ));
        assert!(is_data_file(
            "// the rock's\narray mesh @rock @heat [1 2]\n"
        ));
        assert!(!is_data_file("mesh @rock \"Rock\"\n"));
        assert!(!is_data_file("array mesh @a v [1]\narray mesh @b v [1]\n"));
    }

    #[test]
    fn the_layout_is_one_element_per_line() {
        assert_eq!(
            layout_of("array mesh @rock @heat [0.25 4 1]\n").as_deref(),
            Some("array mesh @rock @heat [\n  0.25\n  4\n  1\n]\n")
        );
        assert_eq!(
            layout_of("array mesh @rock vertices [\n  [0m  0m 0m] [1m 0m 0.5m]\n]\n").as_deref(),
            Some("array mesh @rock vertices [\n  [0m 0m 0m]\n  [1m 0m 0.5m]\n]\n")
        );
        assert_eq!(layout_of("array mesh @rock v [1] // x\n"), None);
        // `//` in a string is no comment.
        assert!(layout_of("array mesh @rock @names [\"a//b\" \"c\"]\n").is_some());
        assert_eq!(layout_of("mesh @rock \"Rock\"\n"), None);
    }
}
