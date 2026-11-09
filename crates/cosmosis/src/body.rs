//! A text datablock's body file (Null, section 12; RFC 0011, Sau's decision S103; Sau task
//! 1d-09e).
//!
//! A `text` datablock's content is a file of its own, named by its `body` line with a path from
//! the project's root, by a media file's path rules. The default layout puts it in `scripts/`,
//! named from the text's name ([`file_name`]). [`check`] holds each `body` line to its file, and
//! [`as_blender_reads`] gives a file's bytes as Blender keeps a text.

use std::path::Path;

use crate::diagnostic::{Diagnostic, Severity};
use crate::layout::{self, SourceFile};
use crate::media::{MediaPath, classify};
use crate::tree::Span;

/// The folder the default layout puts texts in, their `.noll` files and their bodies (spec,
/// section 2: "Python generators and text datablocks").
pub const FOLDER: &str = "scripts";

/// The file name of a text's body, from the text's `name`, where `taken` says whether a name is
/// already used (by another body or a file in the folder):
/// - the name as it is, when it is a safe single file name (no `/`, `\`, control character,
///   leading `.` or `~`, nor `.` or `..` alone);
/// - else its stem with each separator and control character made a `-` and no leading `.`,
///   `~` or `-`, `text` when nothing is left;
/// - `.txt` added to a name with no extension, or one ending in `.noll`, so it is never read as
///   Null;
/// - `-2`, `-3`, … before the extension where the name is taken, compared without case, as the
///   folders of macOS and Windows compare names.
pub fn file_name(name: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let safe = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && !name.starts_with('~')
        && !name
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control());
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e))
            if !s.is_empty()
                && !e.is_empty()
                && e.len() <= 8
                && e.bytes().all(|b| b.is_ascii_alphanumeric()) =>
        {
            (s, Some(e))
        }
        _ => (name, None),
    };
    let (stem, ext) = if safe {
        (stem.to_string(), ext.map(str::to_string))
    } else {
        // Each separator or control character a `-`, and no leading `.`, `~` or `-`.
        let cleaned: String = stem
            .chars()
            .map(|c| {
                if c == '/' || c == '\\' || c.is_control() {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        let kebab = cleaned
            .trim_start_matches(['.', '~', '-'])
            .trim_end_matches('-')
            .to_string();
        let kebab = if kebab.is_empty() {
            "text".to_string()
        } else {
            kebab
        };
        (kebab, ext.map(str::to_ascii_lowercase))
    };
    let ext = match ext {
        Some(e) if !e.eq_ignore_ascii_case("noll") => e,
        Some(e) => format!("{e}.txt"),
        None => "txt".to_string(),
    };
    let mut out = format!("{stem}.{ext}");
    let mut n = 2;
    while taken(&out) {
        out = format!("{stem}-{n}.{ext}");
        n += 1;
    }
    out
}

/// `bytes` as Blender keeps a text read from them (`text_from_buf` and `cleanup_textline` in
/// Blender's `text.cc`): every control character but the tab and the line feed taken out, and a
/// byte that is not part of UTF-8 read as Latin-1, its character written in UTF-8.
pub fn as_blender_reads(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(s) => {
                push_clean(s, &mut out);
                break;
            }
            Err(e) => {
                let (good, bad) = rest.split_at(e.valid_up_to());
                // `good` is UTF-8, by `valid_up_to`.
                push_clean(std::str::from_utf8(good).unwrap_or_default(), &mut out);
                let n = e.error_len().unwrap_or(bad.len()).max(1);
                for &b in &bad[..n] {
                    push_clean(char::from(b).encode_utf8(&mut [0; 4]), &mut out);
                }
                rest = &bad[n..];
            }
        }
    }
    out
}

fn push_clean(s: &str, out: &mut Vec<u8>) {
    for c in s.chars() {
        if (c as u32) < 0x20 && c != '\t' && c != '\n' {
            continue;
        }
        out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
    }
}

/// Each text's `body` line in `sources` (from the layout check of `root`), checked:
/// `cosmosis-body-path` for a path that is not one from the root (an absolute path too: a body is
/// the project's own), or one that leaves the root through a link; `cosmosis-body-missing` for
/// one that names no file; and the warning `cosmosis-body-bytes` for a file holding bytes Blender
/// would change on reading it.
pub fn check(root: &Path, sources: &[SourceFile]) -> Vec<Diagnostic> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut out = Vec::new();
    for file in sources.iter().filter(|f| layout::holds_datablocks(&f.path)) {
        let parse = null_syntax::parse(&file.text);
        for node in parse.document().nodes().filter(|n| n.name() == "text") {
            for c in node.children().filter(|c| c.name() == "body") {
                let Some(v) = c.args().next() else { continue };
                let Some(path) = v.as_string() else { continue };
                let sp = v.span();
                let span = Span::new(sp.start, sp.end);
                let at = |d: Diagnostic| d.in_file(&file.path).at(file.text.as_bytes(), span);
                let rel = match classify(&path) {
                    MediaPath::Root(rel) => rel,
                    MediaPath::Absolute(_) => {
                        out.push(at(Diagnostic::error(
                            "cosmosis-body-path",
                            format!("`{path}` is outside the project; a text's body is the project's own"),
                        )
                        .with_help("copy the file into scripts/ and write its path from the project's root")));
                        continue;
                    }
                    MediaPath::Bad(why) => {
                        out.push(at(Diagnostic::error(
                            "cosmosis-body-path",
                            format!("`{path}` is not a path from the project's root: {why}"),
                        )
                        .with_help(
                            "write the path from the project's root, such as `scripts/shader.py`",
                        )));
                        continue;
                    }
                };
                let full = root.join(&rel);
                match full.canonicalize() {
                    Ok(real) if !real.starts_with(&canonical_root) => {
                        out.push(at(Diagnostic::error(
                            "cosmosis-body-path",
                            format!(
                                "`{path}` leaves the project through a link, to {}",
                                real.display()
                            ),
                        )
                        .with_help(
                            "copy the file into the project; a link to outside it is not followed",
                        )))
                    }
                    Ok(real) if real.is_file() => {
                        let Ok(bytes) = std::fs::read(&real) else {
                            continue;
                        };
                        if as_blender_reads(&bytes) != bytes {
                            out.push(at(Diagnostic {
                                severity: Severity::Warning,
                                ..Diagnostic::error(
                                    "cosmosis-body-bytes",
                                    format!("`{path}` holds bytes Blender changes on reading it (a carriage return, another control character, or bytes that are not UTF-8), so the text will not hold the file's bytes"),
                                )
                            }
                            .with_help("save the file as UTF-8 with line feeds alone")));
                        }
                    }
                    Ok(_) => out.push(at(Diagnostic::error(
                        "cosmosis-body-missing",
                        format!("`{path}` is a folder, not a file"),
                    ))),
                    Err(_) => out.push(at(Diagnostic::error(
                        "cosmosis-body-missing",
                        format!("no file `{path}` in the project"),
                    )
                    .with_help("put the text's file there, or write the path of one that is"))),
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_is_named_from_its_texts_name() {
        let none = |_: &str| false;
        assert_eq!(file_name("shader.py", &none), "shader.py");
        assert_eq!(file_name("Text", &none), "Text.txt");
        assert_eq!(file_name("notes.noll", &none), "notes.noll.txt");
        assert_eq!(file_name("rig ui.py", &none), "rig ui.py");
        assert_eq!(file_name("../evil.py", &none), "evil.py");
        assert_eq!(file_name("a/b.osl", &none), "a-b.osl");
        assert_eq!(file_name(".hidden", &none), "hidden.txt");
        assert_eq!(file_name("..", &none), "text.txt");
        assert_eq!(file_name("", &none), "text.txt");
        // Taken, case ignored: `-2` before the extension.
        let taken = |n: &str| {
            ["shader.py", "shader-2.py"]
                .iter()
                .any(|t| t.eq_ignore_ascii_case(n))
        };
        assert_eq!(file_name("Shader.py", &taken), "Shader-3.py");
    }

    #[test]
    fn bytes_are_kept_as_blender_reads_them() {
        assert_eq!(
            as_blender_reads(b"import bpy\n\tx = 1\n"),
            b"import bpy\n\tx = 1\n"
        );
        assert_eq!(as_blender_reads(b"a\r\nb\x07"), b"a\nb");
        assert_eq!(as_blender_reads("é".as_bytes()), "é".as_bytes());
        assert_eq!(as_blender_reads(b"\xe9t\xe9"), "été".as_bytes());
    }

    #[test]
    fn each_body_line_is_held_to_its_file() {
        let root = std::env::temp_dir().join(format!("cosmosis-body-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("scripts")).unwrap();
        std::fs::write(root.join("scripts/a.py"), b"print(1)\n").unwrap();
        std::fs::write(root.join("scripts/crlf.py"), b"print(1)\r\n").unwrap();
        let text = [
            "text @a \"a.py\" { body \"scripts/a.py\" }",
            "text @b \"b.py\" { body \"scripts/b.py\" }",
            "text @c \"c.py\" { body \"//c.py\" }",
            "text @d \"d.py\" { body \"/tmp/d.py\" }",
            "text @e \"crlf.py\" { body \"scripts/crlf.py\" }",
            "text @f \"Empty\"",
        ]
        .join("\n");
        let sources = [SourceFile {
            path: "scripts/texts.noll".into(),
            text,
        }];
        let found: Vec<(&str, bool)> = check(&root, &sources)
            .iter()
            .map(|d| (d.code, d.is_error()))
            .collect();
        assert_eq!(
            found,
            [
                ("cosmosis-body-missing", true),
                ("cosmosis-body-path", true),
                ("cosmosis-body-path", true),
                ("cosmosis-body-bytes", false),
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
