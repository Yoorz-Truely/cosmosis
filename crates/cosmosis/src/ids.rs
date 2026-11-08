//! The project's handle map, `.sau/ids.noll` (Sau's decisions S17, S61 (2) and S66; Null's spec,
//! section 7, "The handle map"): each datablock handle's persistent ID, kept in Git with the
//! project. Sau's core writes it after an apply (Sau task 1b-01f); `sau rename` moves an entry to
//! its new handle (task 1b-01h). The form is the one Sau writes, so either may write the file:
//!
//! ```text
//! // Sau's map of handles to persistent IDs (decisions S61 (2) and S66). Sau writes this file;
//! // keep it in Git with the project.
//! ids {
//!   object @cube "4b43ea435eaf647774ed7d63bbb80432"
//! }
//! ```
//!
//! One `ids` block of `kind @handle "<32 hex digits>"` entries, sorted by kind and handle. An
//! entry may carry a block of its owner's sub-objects (Sau's decision S73 (1)); this module does
//! not read it, but keeps its text with its entry, through a rename too (S73 (4)).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::layout::IDS_FILE;

/// (kind, handle) to its entry.
pub type Map = BTreeMap<(String, String), Entry>;

/// One datablock's entry: its ID, and its block of sub-object entries as written, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The ID's 32 lower-case hexadecimal digits.
    pub id: String,
    /// The entry's block, from `{` to `}`, exactly as the file has it (S73's sub-objects).
    pub block: Option<String>,
}

/// Reads the map's entries; the second list names, by line, the entries it could not use (another
/// form, a handle twice, an ID twice), which are left out, the first of two kept.
pub fn parse(text: &str) -> (Map, Vec<usize>) {
    let file = null_syntax::parse(text);
    let mut map = Map::new();
    let mut seen = std::collections::HashSet::new();
    let mut bad = Vec::new();
    let line = |at: usize| text[..at].matches('\n').count() + 1;
    for top in file.document().nodes() {
        if top.name() != "ids" {
            bad.push(line(top.span().start));
            continue;
        }
        for entry in top.children() {
            let mut args = entry.args();
            let handle = args
                .next()
                .and_then(|a| a.as_word().map(str::to_string))
                .filter(|w| w.starts_with('@') && w.len() > 1);
            let id = args
                .next()
                .and_then(|a| a.as_string().map(|s| s.to_ascii_lowercase()))
                .filter(|s| {
                    s.len() == 32
                        && s.bytes().all(|b| b.is_ascii_hexdigit())
                        && s.bytes().any(|b| b != b'0')
                });
            let key = handle.map(|h| (entry.name().into_owned(), h));
            match (key, id, args.next()) {
                (Some(key), Some(id), None)
                    if !map.contains_key(&key) && seen.insert(id.clone()) =>
                {
                    let block = entry.block().map(|b| b.text().to_string());
                    map.insert(key, Entry { id, block });
                }
                _ => bad.push(line(entry.span().start)),
            }
        }
    }
    (map, bad)
}

/// The map's text, sorted by kind and handle, as Sau writes it.
pub fn text(map: &Map) -> String {
    let mut out = String::from(
        "// Sau's map of handles to persistent IDs (decisions S61 (2) and S66). Sau writes this file;\n\
         // keep it in Git with the project.\nids {\n",
    );
    for ((kind, handle), entry) in map {
        out.push_str(&format!("  {kind} {handle} \"{}\"", entry.id));
        if let Some(block) = &entry.block {
            out.push(' ');
            out.push_str(block);
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

/// Refuses a map that would be read or written through a link: `.sau` or the map itself (the
/// layout check does not follow one either).
pub fn check_not_linked(root: &Path) -> std::io::Result<()> {
    for p in [root.join(".sau"), root.join(IDS_FILE)] {
        if p.symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(std::io::Error::other(format!(
                "{} is a link; the map is not read or written through one",
                p.display()
            )));
        }
    }
    Ok(())
}

/// Reads the map of the project at `root`: `None` when it has none (only a missing file means
/// that), and an error for a link, a file that cannot be read, or a line that cannot be used,
/// since writing such a map again would lose what it holds.
pub fn read(root: &Path) -> std::io::Result<Option<Map>> {
    check_not_linked(root)?;
    let text = match std::fs::read_to_string(root.join(IDS_FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let (map, bad) = parse(&text);
    if !bad.is_empty() {
        let lines: Vec<String> = bad.iter().map(usize::to_string).collect();
        return Err(std::io::Error::other(format!(
            "{IDS_FILE} has lines it cannot read ({})",
            lines.join(", ")
        )));
    }
    Ok(Some(map))
}

/// Writes `text` beside the map, to be renamed over it by [`commit`]: the first half of an
/// atomic write, so a caller can stage every file before it replaces any.
pub fn stage(root: &Path, text: &str) -> std::io::Result<PathBuf> {
    check_not_linked(root)?;
    let folder = root.join(".sau");
    std::fs::create_dir_all(&folder)?;
    let temporary = folder.join("ids.noll.sau-write");
    let mut out = std::fs::File::create(&temporary)?;
    out.write_all(text.as_bytes())?;
    out.sync_all()?;
    Ok(temporary)
}

/// Renames a [`stage`]d map over the map.
pub fn commit(root: &Path, staged: &Path) -> std::io::Result<()> {
    std::fs::rename(staged, root.join(IDS_FILE))
}

/// Writes the map of the project at `root` atomically (beside it, then renamed over it), never
/// through a `.sau` or a map that is a link.
pub fn write(root: &Path, text: &str) -> std::io::Result<()> {
    let staged = stage(root, text)?;
    commit(root, &staged)
}

/// Moves the entry of `kind`'s `old` handle to `new`: whether the map had one.
pub fn rename(map: &mut Map, kind: &str, old: &str, new: &str) -> bool {
    match map.remove(&(kind.to_string(), old.to_string())) {
        Some(id) => {
            map.insert((kind.to_string(), new.to_string()), id);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_reads_back_and_a_rename_keeps_the_id_under_the_new_handle() {
        let text = "ids {\n  object @cube \"4B43EA435EAF647774ED7D63BBB80432\"\n  mesh @m \"00000000000000000000000000000001\"\n}\n";
        let (mut map, bad) = parse(text);
        assert!(bad.is_empty());
        assert!(rename(&mut map, "object", "@cube", "@box"));
        assert!(!rename(&mut map, "object", "@cube", "@box"));
        let written = text_of(&map);
        assert_eq!(
            written,
            "// Sau's map of handles to persistent IDs (decisions S61 (2) and S66). Sau writes this file;\n// keep it in Git with the project.\nids {\n  mesh @m \"00000000000000000000000000000001\"\n  object @box \"4b43ea435eaf647774ed7d63bbb80432\"\n}\n"
        );
        assert_eq!(parse(&written).0, map);
    }

    fn text_of(map: &Map) -> String {
        text(map)
    }

    #[test]
    fn the_block_of_sub_objects_of_an_entry_is_kept_and_moves_with_it() {
        // S73 (1) and (4): the block is the core's to read; every writer keeps it with its entry.
        let text = "ids {\n  object @cube \"00000000000000000000000000000001\" {\n    modifier @bevel 7\n    constraint @aim \"Aim\"\n  }\n  mesh @m \"00000000000000000000000000000002\"\n}\n";
        let (mut map, bad) = parse(text);
        assert!(bad.is_empty(), "{bad:?}");
        let block = map[&("object".to_string(), "@cube".to_string())]
            .block
            .clone();
        assert_eq!(
            block.as_deref(),
            Some("{\n    modifier @bevel 7\n    constraint @aim \"Aim\"\n  }")
        );
        assert!(rename(&mut map, "object", "@cube", "@box"));
        let written = text_of(&map);
        assert!(
            written.contains("  object @box \"00000000000000000000000000000001\" {\n    modifier @bevel 7\n    constraint @aim \"Aim\"\n  }\n"),
            "{written}"
        );
        assert_eq!(parse(&written).0, map);
    }

    #[test]
    fn bad_entries_are_named_and_left_out() {
        let text = "ids {\n  object @a \"00000000000000000000000000000001\"\n  object @b \"zz\"\n  object @c \"00000000000000000000000000000001\"\n  object @a \"00000000000000000000000000000002\"\n  object @d \"00000000000000000000000000000000\"\n}\nother {}\n";
        let (map, bad) = parse(text);
        assert_eq!(map.len(), 1);
        assert_eq!(bad, vec![3, 4, 5, 6, 8]);
    }
}
