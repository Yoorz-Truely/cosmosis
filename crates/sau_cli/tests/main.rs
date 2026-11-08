//! This crate's integration tests, in one test binary (Sau task 0-26, S69): each file beside this
//! one is a module here, not a binary of its own, so `cargo test` links one executable, not
//! 10. The files keep their names, so their snapshots and proptest regressions stay where
//! they were. A new test file must be listed here: `every_test_file_is_a_module` fails otherwise.

#[path = "check.rs"]
mod check;
#[path = "convert.rs"]
mod convert;
#[path = "corpus.rs"]
mod corpus;
#[path = "data_files.rs"]
mod data_files;
#[path = "fmt.rs"]
mod fmt;
#[path = "layout_rules.rs"]
mod layout_rules;
#[path = "new.rs"]
mod new;
#[path = "project_corpus.rs"]
mod project_corpus;
#[path = "project_schema.rs"]
mod project_schema;
#[path = "rename.rs"]
mod rename;
#[path = "v0_corpus.rs"]
mod v0_corpus;

/// The test files of this folder, each a module above.
const MODULES: &[&str] = &[
    "check.rs",
    "convert.rs",
    "corpus.rs",
    "data_files.rs",
    "fmt.rs",
    "layout_rules.rs",
    "new.rs",
    "project_corpus.rs",
    "project_schema.rs",
    "rename.rs",
    "v0_corpus.rs",
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
