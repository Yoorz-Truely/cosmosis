//! This crate's integration tests, in one test binary (Sau task 0-26, S69): each file beside this
//! one is a module here, not a binary of its own, so `cargo test` links one executable, not
//! 4. The files keep their names, so their snapshots and proptest regressions stay where
//! they were. A new test file must be listed here: `every_test_file_is_a_module` fails otherwise.

#[path = "layout.rs"]
mod layout;
#[path = "manifest.rs"]
mod manifest;
#[path = "manifest_fields.rs"]
mod manifest_fields;
#[path = "migrate.rs"]
mod migrate;
#[path = "raw.rs"]
mod raw;

/// The test files of this folder, each a module above.
const MODULES: &[&str] = &[
    "layout.rs",
    "manifest.rs",
    "manifest_fields.rs",
    "migrate.rs",
    "raw.rs",
];

#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs") && n != "main.rs")
        .collect();
    found.sort();
    assert_eq!(
        found, MODULES,
        "a test file in tests/ is not a module of tests/main.rs, so it would never run; add it there"
    );
}
