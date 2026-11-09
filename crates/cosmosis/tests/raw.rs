//! `raw` blocks checked against Null's raw table (Sau task C-11; Null spec, section 19). The
//! table Null ships is a placeholder until a build of Sau writes it, so these tests give the
//! schema a table of the corpus's fields, as the build's would list them.

use std::path::Path;

use cosmosis::layout;
use cosmosis::schema_check::{self, check_with};
use null_check::Schema;
use null_check::raw::RawTable;

/// The corpus's `raw` fields as Blender 5.3's SDNA has them on `Object`.
const RAW: &str = r#"{
  "format": 1,
  "structs": {
    "Object": [
      {"path": "restore_mode", "type": "int"},
      {"path": "col_mask", "type": "ushort"},
      {"path": "collision_boundtype", "type": "char"},
      {"path": "constinv", "type": "float", "dims": [4, 4]}
    ]
  }
}"#;

fn schema() -> &'static Schema {
    static S: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let text = include_str!("../../../../Null/schema/null-schema-project.json");
        Schema::from_json(text)
            .unwrap()
            .with_raw(RawTable::from_json(RAW).unwrap())
    })
}

const PROJECT: &str = "../../corpus/project/raw-fields.sau";
const BUOY: &str = "objects/buoy.noll";

/// The corpus project's diagnostics with `buoy` as its buoy's file: (code, line) for each.
fn found(buoy: Option<&str>) -> Vec<(String, usize)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(PROJECT);
    let mut sources = layout::check(&root, None).sources;
    if let Some(text) = buoy {
        let f = sources
            .iter_mut()
            .find(|f| f.path == Path::new(BUOY))
            .unwrap();
        f.text = text.to_string();
    }
    check_with(schema(), &sources)
        .into_iter()
        .map(|d| (d.code.to_string(), d.position.map_or(0, |p| p.line)))
        .collect()
}

fn buoy() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(PROJECT);
    std::fs::read_to_string(root.join(BUOY)).unwrap()
}

#[test]
fn the_corpus_raw_block_checks_clean_against_the_table() {
    assert_eq!(found(None), []);
}

#[test]
fn an_unknown_raw_field_is_reported_at_its_line() {
    let text = buoy().replace("restore_mode 0", "restore_mood 0");
    let f = found(Some(&text));
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0], ("null-unknown-property".to_string(), 4), "{f:?}");
}

#[test]
fn an_out_of_range_raw_value_is_reported_at_its_line() {
    for (from, to, line) in [
        ("col_mask 255", "col_mask 70000", 5),
        ("collision_boundtype 4", "collision_boundtype 256", 6),
        ("restore_mode 0", "restore_mode 2147483648", 4),
    ] {
        let text = buoy().replace(from, to);
        let f = found(Some(&text));
        assert_eq!(f.len(), 1, "{to}: {f:?}");
        assert_eq!(f[0].1, line, "{to}: {f:?}");
        assert!(f[0].0.starts_with("null-"), "{to}: {f:?}");
    }
    // A list of another length.
    let text = buoy().replace("[0 0 1 0] ", "");
    let f = found(Some(&text));
    assert_eq!(f.len(), 1, "{f:?}");
}

#[test]
fn without_a_table_the_blocks_are_noted_unchecked() {
    // The table Null ships today is the placeholder: no structs, so no table is loaded, and
    // `sau check` notes the blocks it cannot check, once, at the first.
    let shipped = schema_check::raw_table();
    let plain = Schema::from_json(include_str!(
        "../../../../Null/schema/null-schema-project.json"
    ))
    .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(PROJECT);
    let sources = layout::check(&root, None).sources;
    let note = schema_check::raw_unchecked(&plain, &sources).unwrap();
    assert_eq!(note.code, "cosmosis-raw-unchecked");
    assert_eq!(note.position.map(|p| (p.line, p.column)), Some((3, 3)));
    assert!(schema_check::raw_unchecked(schema(), &sources).is_none());
    // The shipped schema carries the table exactly when Null ships one.
    assert_eq!(schema_check::schema().raw.is_some(), shipped.is_some());
}
