//! Every Cosmósis project is also a Blender project (S45; Sau task 1d-04c): its
//! `.blender_project/project.toml`, Blender's own project file, is written from `cosmosis.noll`
//! and kept in step with it. `cosmosis.noll` is the source: the Blender project's name is the
//! package's, and a name changed in Blender's Project Settings is replaced at the next sync.
//!
//! Blender's file (`BKE_blender_project.hh`, `blender_project_config.cc`) is TOML: a
//! `schema_version`, the `name`, and what Blender's settings add (an OpenColorIO config, a splash
//! image, `[[variables]]`, `[[asset_libraries]]`). Only the top-level `name` is ever written
//! here; everything else in an existing file is kept byte for byte.

use std::path::{Path, PathBuf};

/// The folder Blender looks for in a project's root.
pub const DIR: &str = ".blender_project";
/// Blender's project file in [`DIR`].
pub const FILE: &str = "project.toml";
/// The `schema_version` of Blender's project file this writes (`project_schema_version`).
pub const SCHEMA_VERSION: u32 = 1;

/// The path of `root`'s Blender project file.
pub fn path(root: &Path) -> PathBuf {
    root.join(DIR).join(FILE)
}

/// `name` as a TOML basic string, quotes included.
fn toml_string(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    out.push('"');
    for c in name.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether `line` sets the top-level key `key` (`key = …`, spaces allowed around the `=`).
fn sets(line: &str, key: &str) -> bool {
    line.trim_start()
        .strip_prefix(key)
        .is_some_and(|rest| rest.trim_start().starts_with('='))
}

/// The project file's text for the project `name`: a new file when there is none, or
/// `existing` with its top-level `name` replaced (or added after `schema_version`), every
/// other line kept. A `name` inside a table (`[[variables]]`) is the table's, not the project's.
pub fn text(name: &str, existing: Option<&str>) -> String {
    let line = format!("name = {}", toml_string(name));
    let Some(existing) = existing else {
        return format!("schema_version = {SCHEMA_VERSION}\n{line}\n");
    };
    let mut out = Vec::new();
    let mut top = true;
    let mut named = false;
    let mut version_at = None;
    for l in existing.lines() {
        if l.trim_start().starts_with('[') {
            top = false;
        }
        if top && sets(l, "name") {
            if !named {
                out.push(line.clone());
                named = true;
            }
            continue;
        }
        if top && sets(l, "schema_version") {
            version_at = Some(out.len());
        }
        out.push(l.to_string());
    }
    if !named {
        // After `schema_version`, or first: a top-level key must come before any table.
        let at = version_at.map_or(0, |i| i + 1);
        out.insert(at, line);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// Writes `root`'s Blender project file for the project `name`, keeping what Blender's settings
/// added; true if the file changed. The folder is made when missing.
pub fn write(root: &Path, name: &str) -> std::io::Result<bool> {
    let file = path(root);
    let existing = match std::fs::read_to_string(&file) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let new = text(name, existing.as_deref());
    if existing.as_deref() == Some(new.as_str()) {
        return Ok(false);
    }
    std::fs::create_dir_all(root.join(DIR))?;
    std::fs::write(&file, new)?;
    Ok(true)
}

/// Brings `root`'s Blender project file in step with its `cosmosis.noll`: the package's name,
/// written when it differs (true then). None when the manifest cannot be read or has errors,
/// which `sau check` reports; then nothing is written.
pub fn sync(root: &Path) -> Option<std::io::Result<bool>> {
    let text = std::fs::read_to_string(root.join(crate::manifest::MANIFEST_FILE)).ok()?;
    let manifest = crate::Manifest::parse(&text).ok()?;
    Some(write(root, &manifest.name))
}

/// The project name `root`'s Blender project file gives, if it has one: the top-level `name`,
/// a basic string without escapes as this module writes it. None when the file or the key is
/// missing.
pub fn name(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path(root)).ok()?;
    for l in text.lines() {
        if l.trim_start().starts_with('[') {
            return None;
        }
        if sets(l, "name") {
            let value = l.split_once('=')?.1.trim();
            return value
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_string);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_file_has_the_schema_version_and_the_name() {
        assert_eq!(
            text("harbor-at-dusk", None),
            "schema_version = 1\nname = \"harbor-at-dusk\"\n"
        );
    }

    #[test]
    fn only_the_top_level_name_changes_and_the_rest_is_kept() {
        let blender = "schema_version = 1\nname = \"old\"\nocio_config = \"//ocio/config.ocio\"\n\n\
                       [[variables]]\nname = \"shot\"\ntype = \"STRING\"\nvalue = \"010\"\n";
        let out = text("harbor-at-dusk", Some(blender));
        assert_eq!(
            out,
            "schema_version = 1\nname = \"harbor-at-dusk\"\nocio_config = \"//ocio/config.ocio\"\n\n\
             [[variables]]\nname = \"shot\"\ntype = \"STRING\"\nvalue = \"010\"\n"
        );
        // Written again, nothing changes.
        assert_eq!(text("harbor-at-dusk", Some(&out)), out);
    }

    #[test]
    fn a_file_without_a_name_gets_one_after_its_version_before_any_table() {
        let out = text(
            "p",
            Some("schema_version = 1\n[[asset_libraries]]\nname = \"lib\"\n"),
        );
        assert_eq!(
            out,
            "schema_version = 1\nname = \"p\"\n[[asset_libraries]]\nname = \"lib\"\n"
        );
    }

    #[test]
    fn a_name_is_escaped_as_toml_needs() {
        assert_eq!(toml_string("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn write_makes_the_folder_says_whether_it_changed_and_reads_back() {
        let root = std::env::temp_dir().join(format!("cosmosis-bp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(name(&root), None);
        assert!(write(&root, "first").unwrap());
        assert_eq!(name(&root).as_deref(), Some("first"));
        assert!(!write(&root, "first").unwrap());
        assert!(write(&root, "second").unwrap());
        assert_eq!(name(&root).as_deref(), Some("second"));
        // In step with the manifest: a rename there renames it here.
        assert!(sync(&root).is_none(), "no manifest yet");
        std::fs::write(
            root.join("cosmosis.noll"),
            "package \"harbor\" {\n  null-format 1\n}\n",
        )
        .unwrap();
        assert!(sync(&root).unwrap().unwrap());
        assert_eq!(name(&root).as_deref(), Some("harbor"));
        assert!(!sync(&root).unwrap().unwrap());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
