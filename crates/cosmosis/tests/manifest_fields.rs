//! The manifest complete (Sau task 1d-04a): every field of the spec's section 4, one case per
//! field and per wrong value, read from text through Null's parser as `sau check` reads it; the
//! spec's field list and the code's are the same; and every error the reader has is met by at
//! least one case.

use std::collections::BTreeSet;

use cosmosis::manifest::{FIELDS, Kind, Manifest};

/// What a case expects: the manifest reads, or its errors' codes, in order.
enum Want {
    Reads(fn(&Manifest) -> bool),
    Codes(&'static [&'static str]),
}

use Want::{Codes, Reads};

/// A whole manifest: `package "harbor" { null-format 1 <lines> }`, unless `lines` holds its own
/// `package` line.
fn manifest(lines: &str) -> String {
    if lines.contains("package") {
        lines.to_string()
    } else {
        format!("package \"harbor\" {{\n  null-format 1\n{lines}}}\n")
    }
}

/// (field, what is written, what it gives). Each field's right forms first, then each wrong one.
const CASES: &[(&str, &str, Want)] = &[
    // The name: the quoted kebab-case string after `package`.
    (
        "name",
        "package \"harbor-at-dusk\" {\n  null-format 1\n}\n",
        Reads(|m| m.name == "harbor-at-dusk"),
    ),
    (
        "name",
        "package {\n  null-format 1\n}\n",
        Codes(&["cosmosis-manifest-no-name"]),
    ),
    (
        "name",
        "package harbor {\n  null-format 1\n}\n",
        Codes(&["cosmosis-manifest-name-type"]),
    ),
    (
        "name",
        "package \"Harbor At Dusk\" {\n  null-format 1\n}\n",
        Codes(&["cosmosis-manifest-bad-name"]),
    ),
    (
        "name",
        "package \"harbor\" {\n  null-format 1\n  name \"x\"\n}\n",
        Codes(&["cosmosis-manifest-unknown-field"]),
    ),
    // The file: one package node, nothing else.
    (
        "package",
        "// no package node here\n",
        Codes(&["cosmosis-manifest-no-package"]),
    ),
    (
        "package",
        "package \"a\" {\n  null-format 1\n}\npackage \"b\" {\n  null-format 1\n}\n",
        Codes(&["cosmosis-manifest-repeated-package"]),
    ),
    (
        "package",
        "package \"a\" {\n  null-format 1\n}\nkind project\n",
        Codes(&["cosmosis-manifest-unknown-node"]),
    ),
    // kind: one of seven words, `project` when not written.
    ("kind", "", Reads(|m| m.kind == Kind::Project)),
    (
        "kind",
        "  kind project\n",
        Reads(|m| m.kind == Kind::Project),
    ),
    (
        "kind",
        "  kind material\n",
        Reads(|m| m.kind == Kind::Material),
    ),
    (
        "kind",
        "  kind node-group\n",
        Reads(|m| m.kind == Kind::NodeGroup),
    ),
    ("kind", "  kind model\n", Reads(|m| m.kind == Kind::Model)),
    ("kind", "  kind rig\n", Reads(|m| m.kind == Kind::Rig)),
    (
        "kind",
        "  kind template\n",
        Reads(|m| m.kind == Kind::Template),
    ),
    (
        "kind",
        "  kind library\n",
        Reads(|m| m.kind == Kind::Library),
    ),
    ("kind", "  kind scene\n", Codes(&["cosmosis-manifest-kind"])),
    (
        "kind",
        "  kind \"project\"\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    (
        "kind",
        "  kind [project]\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    // version: major.minor.patch, whole numbers, bare.
    (
        "version",
        "  version 0.1.0\n",
        Reads(|m| m.version.map(|v| v.to_string()).as_deref() == Some("0.1.0")),
    ),
    (
        "version",
        "  version 12.30.4\n",
        Reads(|m| m.version.map(|v| v.to_string()).as_deref() == Some("12.30.4")),
    ),
    (
        "version",
        "  version 0.1\n",
        Codes(&["cosmosis-manifest-version"]),
    ),
    (
        "version",
        "  version 1.2.3.4\n",
        Codes(&["cosmosis-manifest-version"]),
    ),
    (
        "version",
        "  version 01.2.3\n",
        Codes(&["cosmosis-manifest-version"]),
    ),
    (
        "version",
        "  version 1.2.x\n",
        Codes(&["cosmosis-manifest-version"]),
    ),
    (
        "version",
        "  version \"0.1.0\"\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    // sau: major.minor or major.minor.patch.
    (
        "sau",
        "  sau 0.1\n",
        Reads(|m| m.sau.map(|v| v.to_string()).as_deref() == Some("0.1")),
    ),
    (
        "sau",
        "  sau 0.1.2\n",
        Reads(|m| m.sau.map(|v| v.to_string()).as_deref() == Some("0.1.2")),
    ),
    ("sau", "  sau 1\n", Codes(&["cosmosis-manifest-version"])),
    (
        "sau",
        "  sau 0.1.2.3\n",
        Codes(&["cosmosis-manifest-version"]),
    ),
    ("sau", "  sau \"0.1\"\n", Codes(&["cosmosis-manifest-type"])),
    // null-format: required, a whole number from 1, no newer than this Sau reads.
    (
        "null-format",
        "package \"harbor\" {\n  null-format 1\n}\n",
        Reads(|m| m.null_format == 1),
    ),
    (
        "null-format",
        "package \"harbor\" {\n}\n",
        Codes(&["cosmosis-manifest-no-null-format"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format 0\n}\n",
        Codes(&["cosmosis-manifest-null-format"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format 1.5\n}\n",
        Codes(&["cosmosis-manifest-null-format"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format -1\n}\n",
        Codes(&["cosmosis-manifest-null-format"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format 2\n}\n",
        Codes(&["cosmosis-manifest-newer-format"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format \"1\"\n}\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    // license: a quoted string.
    (
        "license",
        "  license \"All rights reserved\"\n",
        Reads(|m| m.license.as_deref() == Some("All rights reserved")),
    ),
    (
        "license",
        "  license MIT\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    (
        "license",
        "  license [\"MIT\" \"Apache-2.0\"]\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    // dependencies: each a requirement, a path, or both.
    (
        "dependencies",
        "  dependencies {\n    cosmosis-materials \"^1.2\"\n  }\n",
        Reads(|m| {
            m.dependencies.len() == 1 && m.dependencies[0].requirement.as_deref() == Some("^1.2")
        }),
    ),
    (
        "dependencies",
        "  dependencies {\n    my-trees path=\"../Trees.sau\"\n  }\n",
        Reads(|m| m.dependencies[0].path.as_deref() == Some("../Trees.sau")),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees \">=1, <2\" path=\"../Trees.sau\"\n  }\n",
        Reads(|m| m.dependencies[0].requirement.is_some() && m.dependencies[0].path.is_some()),
    ),
    (
        "dependencies",
        "  dependencies {\n  }\n",
        Reads(|m| m.dependencies.is_empty()),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees\n  }\n",
        Codes(&["cosmosis-manifest-dependency-source"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees \"one point two\"\n  }\n",
        Codes(&["cosmosis-manifest-requirement"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees ^1.2\n  }\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees path=\"\"\n  }\n",
        Codes(&["cosmosis-manifest-empty-path"]),
    ),
    (
        "null-format",
        "package \"harbor\" {\n  null-format 99999999999999999999\n}\n",
        Codes(&["cosmosis-manifest-newer-format"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees \"^1\" version=\"2\"\n  }\n",
        Codes(&["cosmosis-manifest-dependency-property"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees \"^1\"\n    trees \"^2\"\n  }\n",
        Codes(&["cosmosis-manifest-repeated-dependency"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    Trees \"^1\"\n  }\n",
        // Null's parser finds the name first, as it does every node's (S16).
        Codes(&["null-name"]),
    ),
    (
        "dependencies",
        "  dependencies {\n    trees \"^1\" {\n    }\n  }\n",
        Codes(&["cosmosis-manifest-unexpected-block"]),
    ),
    (
        "dependencies",
        "  dependencies trees\n",
        Codes(&["cosmosis-manifest-type"]),
    ),
    // Any field: written once, with one value and no block; unknown names are errors.
    (
        "any",
        "  license \"MIT\"\n  license \"MIT\"\n",
        Codes(&["cosmosis-manifest-repeated-field"]),
    ),
    (
        "any",
        "  licence \"MIT\"\n",
        Codes(&["cosmosis-manifest-unknown-field"]),
    ),
    ("any", "  license\n", Codes(&["cosmosis-manifest-no-value"])),
    (
        "any",
        "  version 0.1.0 0.2.0\n",
        Codes(&["cosmosis-manifest-extra-value"]),
    ),
    (
        "any",
        "  kind project {\n  }\n",
        Codes(&["cosmosis-manifest-unexpected-block"]),
    ),
    (
        "any",
        "package \"harbor\" null-format=1 kind=material\n",
        Reads(|m| m.kind == Kind::Material),
    ),
];

#[test]
fn every_field_reads_and_every_wrong_value_is_its_error() {
    let mut wrong = Vec::new();
    for (field, lines, want) in CASES {
        let text = manifest(lines);
        match (Manifest::parse(&text), want) {
            (Ok(m), Reads(check)) if check(&m) => {}
            (Err(ds), Codes(codes)) if ds.iter().map(|d| d.code).collect::<Vec<_>>() == *codes => {}
            (got, _) => wrong.push(format!(
                "{field}: {text:?} gave {:?}",
                got.map_err(|ds| ds.iter().map(|d| d.code).collect::<Vec<_>>())
            )),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} cases:\n{}",
        wrong.len(),
        CASES.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_field_has_a_case_and_every_error_is_met() {
    let fielded: BTreeSet<&str> = CASES.iter().map(|(f, _, _)| *f).collect();
    for f in FIELDS.iter().chain(&["name", "null-format"]) {
        assert!(fielded.contains(f), "no case for the field `{f}`");
    }
    // Every code the reader can give, from its source, is given by some case.
    let source = include_str!("../src/manifest.rs");
    let all: BTreeSet<&str> = source
        .match_indices("\"cosmosis-manifest-")
        .map(|(i, _)| {
            let rest = &source[i + 1..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect();
    let met: BTreeSet<&str> = CASES
        .iter()
        .filter_map(|(_, _, w)| match w {
            Codes(c) => Some(c.iter().copied()),
            Reads(_) => None,
        })
        .flatten()
        .collect();
    let missing: Vec<_> = all.difference(&met).collect();
    assert!(missing.is_empty(), "codes no case gives: {missing:?}");
}

#[test]
fn the_specs_fields_and_the_codes_are_the_same_list() {
    let spec = include_str!("../../../docs/spec/cosmosis-v0.md");
    let section =
        &spec[spec.find("## 4. The manifest").unwrap()..spec.find("## 5. Packages").unwrap()];
    // The example's fields: the package block's own lines, two spaces in.
    let start = section.find("```null\n").unwrap();
    let example = &section[start..start + section[start + 3..].find("```").unwrap()];
    let in_example: BTreeSet<&str> = example
        .lines()
        .filter(|l| {
            l.starts_with("  ") && !l.starts_with("   ") && !l.trim_start().starts_with('}')
        })
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    let in_code: BTreeSet<&str> = FIELDS.iter().copied().collect();
    assert_eq!(
        in_example, in_code,
        "the spec's example and manifest::FIELDS"
    );
    // And the reading rules name every field, in backticks.
    let rules = &section[section.find("How it is read").unwrap()..];
    for f in FIELDS {
        let named = rules.contains(&format!("`{f}`"))
            || (f == "dependencies" && rules.contains("A dependency"));
        assert!(named, "the spec's reading rules do not name `{f}`");
    }
}
