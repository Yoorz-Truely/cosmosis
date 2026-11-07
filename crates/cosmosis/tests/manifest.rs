//! Tests of reading `cosmosis.noll` from hand-made trees: the fields, and every error with
//! its code, message and help, word for word (Sau task C-02).

use cosmosis::manifest::{Dependency, Kind, Manifest, Version, is_kebab, is_requirement, to_kebab};
use cosmosis::tree::{Node, Span, Value};

fn s(text: &str) -> Value {
    Value::string(text)
}

fn w(text: &str) -> Value {
    Value::word(text)
}

fn field(name: &str, value: Value) -> Node {
    Node::new(name).arg(value)
}

/// `package "<name>" { null-format 1 … }` with the given extra children.
fn package(name: &str, children: Vec<Node>) -> Node {
    let mut p = Node::new("package")
        .arg(s(name))
        .child(field("null-format", w("1")));
    for c in children {
        p = p.child(c);
    }
    p
}

/// The errors of reading `nodes`, each as (code, message, help).
fn errors(nodes: &[Node]) -> Vec<(String, String, String)> {
    match Manifest::read(nodes) {
        Ok(m) => panic!("expected errors, read {m:?}"),
        Err(es) => es
            .iter()
            .map(|e| (e.code().into(), e.message(), e.help(b"")))
            .collect(),
    }
}

/// The one error of reading `nodes`.
fn error(nodes: &[Node]) -> (String, String, String) {
    let mut es = errors(nodes);
    assert_eq!(es.len(), 1, "{es:#?}");
    es.remove(0)
}

fn e(code: &str, message: &str, help: &str) -> (String, String, String) {
    (code.into(), message.into(), help.into())
}

#[test]
fn a_name_and_null_format_are_a_manifest() {
    let m = Manifest::read(&[package("harbor-at-dusk", vec![])]).unwrap();
    assert_eq!(
        m,
        Manifest {
            name: "harbor-at-dusk".into(),
            kind: Kind::Project,
            version: None,
            sau: None,
            null_format: 1,
            license: None,
            dependencies: vec![],
        }
    );
}

#[test]
fn the_specs_example_reads_every_field() {
    let deps = Node::new("dependencies")
        .child(Node::new("cosmosis-materials").arg(s("^1.2")))
        .child(Node::new("my-trees").prop("path", s("../Trees.sau")));
    let p = package(
        "harbor-at-dusk",
        vec![
            field("kind", w("project")),
            field("version", w("0.1.0")),
            field("sau", w("0.1")),
            field("license", s("All rights reserved")),
            deps,
        ],
    );
    let m = Manifest::read(&[p]).unwrap();
    assert_eq!(m.kind, Kind::Project);
    assert_eq!(
        m.version,
        Some(Version {
            major: 0,
            minor: 1,
            patch: Some(0)
        })
    );
    assert_eq!(
        m.sau,
        Some(Version {
            major: 0,
            minor: 1,
            patch: None
        })
    );
    assert_eq!(m.version.unwrap().to_string(), "0.1.0");
    assert_eq!(m.sau.unwrap().to_string(), "0.1");
    assert_eq!(m.license.as_deref(), Some("All rights reserved"));
    assert_eq!(
        m.dependencies,
        [
            Dependency {
                name: "cosmosis-materials".into(),
                requirement: Some("^1.2".into()),
                path: None
            },
            Dependency {
                name: "my-trees".into(),
                requirement: None,
                path: Some("../Trees.sau".into())
            },
        ]
    );
}

#[test]
fn fields_may_be_properties_on_the_packages_line() {
    let p = Node::new("package")
        .arg(s("oak"))
        .prop("null-format", w("1"))
        .prop("kind", w("material"));
    let m = Manifest::read(&[p]).unwrap();
    assert_eq!((m.kind, m.null_format), (Kind::Material, 1));
}

#[test]
fn every_kind_reads() {
    for kind in Kind::ALL {
        let m = Manifest::read(&[package("p", vec![field("kind", w(kind.as_str()))])]).unwrap();
        assert_eq!(m.kind, kind);
    }
}

#[test]
fn a_missing_null_format_is_an_error() {
    let p = Node::new("package").arg(s("harbor")).block();
    assert_eq!(
        error(&[p]),
        e(
            "cosmosis-manifest-no-null-format",
            "the package has no `null-format`",
            "add `null-format 1`, the Null format its files are written in"
        )
    );
}

#[test]
fn an_unknown_field_suggests_the_nearest() {
    let p = Node::new("package")
        .arg(s("harbor"))
        .child(field("null_format", w("1")));
    assert_eq!(
        errors(&[p]),
        [
            e(
                "cosmosis-manifest-unknown-field",
                "the package has no field `null_format`",
                "did you mean `null-format`?"
            ),
            e(
                "cosmosis-manifest-no-null-format",
                "the package has no `null-format`",
                "add `null-format 1`, the Null format its files are written in"
            ),
        ]
    );
    assert_eq!(
        error(&[package("harbor", vec![field("colour", w("red"))])]),
        e(
            "cosmosis-manifest-unknown-field",
            "the package has no field `colour`",
            "the fields are kind, version, sau, null-format, license and dependencies"
        )
    );
}

#[test]
fn a_name_field_points_at_the_packages_argument() {
    assert_eq!(
        error(&[package("harbor", vec![field("name", s("harbor"))])]),
        e(
            "cosmosis-manifest-unknown-field",
            "the package has no field `name`",
            "the name is the quoted string after `package`: `package \"my-project\"`"
        )
    );
}

#[test]
fn a_repeated_field_names_the_first_line() {
    let source = b"package \"harbor\" {\n  null-format 1\n  null-format 1\n}\n";
    let p = Node::new("package")
        .arg(s("harbor"))
        .child(field("null-format", w("1")).at(Span::new(21, 32)))
        .child(field("null-format", w("1")).at(Span::new(37, 48)));
    let es = Manifest::read(&[p]).unwrap_err();
    assert_eq!(es.len(), 1);
    assert_eq!(es[0].code(), "cosmosis-manifest-repeated-field");
    assert_eq!(es[0].message(), "`null-format` is written twice");
    assert_eq!(
        es[0].help(source),
        "remove one of them; the first is on line 2"
    );
    assert_eq!(
        es[0].to_diagnostic(source).to_string(),
        "error[cosmosis-manifest-repeated-field]: `null-format` is written twice\n  \
         --> cosmosis.noll:3:3\n  help: remove one of them; the first is on line 2"
    );
}

#[test]
fn a_value_of_the_wrong_form_is_an_error() {
    assert_eq!(
        error(&[Node::new("package")
            .arg(s("harbor"))
            .child(field("null-format", s("1")))]),
        e(
            "cosmosis-manifest-type",
            "`null-format` is a whole number, written bare, not `\"1\"`",
            "write it as `null-format 1`"
        )
    );
    assert_eq!(
        error(&[package("harbor", vec![field("license", w("MIT"))])]),
        e(
            "cosmosis-manifest-type",
            "`license` is a quoted string, not `MIT`",
            "write it as `license \"MIT\"`"
        )
    );
    assert_eq!(
        error(&[package("harbor", vec![field("kind", s("project"))])]),
        e(
            "cosmosis-manifest-type",
            "`kind` is a package kind, written bare, not `\"project\"`",
            "write it as `kind project`"
        )
    );
    assert_eq!(
        error(&[package(
            "harbor",
            vec![field("version", w("0.1.0")).arg(w("x"))]
        )])
        .0,
        "cosmosis-manifest-extra-value"
    );
}

#[test]
fn null_format_is_a_whole_number_from_one() {
    for bad in ["0", "1.5", "-1", "01", "+1", "one", "99999999999"] {
        let p = Node::new("package")
            .arg(s("harbor"))
            .child(field("null-format", w(bad)));
        assert_eq!(
            error(&[p]),
            e(
                "cosmosis-manifest-null-format",
                &format!("`null-format {bad}` is not a format number"),
                "formats are whole numbers from 1; this Cosmósis reads up to format 1"
            ),
            "{bad}"
        );
    }
}

#[test]
fn a_newer_null_format_cannot_be_read() {
    let p = Node::new("package")
        .arg(s("harbor"))
        .child(field("null-format", w("2")));
    assert_eq!(
        error(&[p]),
        e(
            "cosmosis-manifest-newer-format",
            "the project is written in Null format 2, newer than this Cosmósis reads (format 1)",
            "open it with a newer Sau: older formats are migrated, newer ones cannot be"
        )
    );
}

#[test]
fn an_unknown_kind_is_an_error() {
    assert_eq!(
        error(&[package("p", vec![field("kind", w("materal"))])]),
        e(
            "cosmosis-manifest-kind",
            "`materal` is not a package kind",
            "did you mean `material`?"
        )
    );
    assert_eq!(
        error(&[package("p", vec![field("kind", w("scene"))])]),
        e(
            "cosmosis-manifest-kind",
            "`scene` is not a package kind",
            "the kinds are project, material, node-group, model, rig, template and library"
        )
    );
}

#[test]
fn versions_are_whole_numbers() {
    let m = Manifest::read(&[package("p", vec![field("sau", w("0.1.2"))])]).unwrap();
    assert_eq!(
        m.sau,
        Some(Version {
            major: 0,
            minor: 1,
            patch: Some(2)
        })
    );
    for bad in ["0.1", "01.0.0", "0.1.x", "1.2.3.4", "0.1."] {
        assert_eq!(
            error(&[package("p", vec![field("version", w(bad))])]),
            e(
                "cosmosis-manifest-version",
                &format!("`version {bad}` is not a version"),
                "write major.minor.patch in whole numbers, such as `version 0.1.0`"
            ),
            "{bad}"
        );
    }
    assert_eq!(
        error(&[package("p", vec![field("sau", w("1"))])]),
        e(
            "cosmosis-manifest-version",
            "`sau 1` is not a version",
            "write major.minor or major.minor.patch in whole numbers, such as `sau 0.1`"
        )
    );
}

#[test]
fn the_name_is_a_quoted_kebab_case_string() {
    let no_name = Node::new("package").child(field("null-format", w("1")));
    assert_eq!(
        error(&[no_name]),
        e(
            "cosmosis-manifest-no-name",
            "the package has no name",
            "write it as a quoted string after `package`: `package \"my-project\"`"
        )
    );
    let bare = Node::new("package")
        .arg(w("harbor"))
        .child(field("null-format", w("1")));
    assert_eq!(
        error(&[bare]),
        e(
            "cosmosis-manifest-name-type",
            "the package name `harbor` is not a quoted string",
            "write it in quotes: `package \"harbor\"`"
        )
    );
    assert_eq!(
        error(&[package("Harbor at Dusk", vec![])]),
        e(
            "cosmosis-manifest-bad-name",
            "the package name \"Harbor at Dusk\" is not kebab-case",
            "names are lower-case words joined by `-`: \"harbor-at-dusk\""
        )
    );
    assert_eq!(
        error(&[package("3d-scene", vec![])]),
        e(
            "cosmosis-manifest-bad-name",
            "the package name \"3d-scene\" is not kebab-case",
            "names are lower-case letters and digits joined by `-`, starting with a letter, such as \"harbor-at-dusk\""
        )
    );
    let two = Node::new("package")
        .arg(s("a"))
        .arg(s("b"))
        .child(field("null-format", w("1")));
    assert_eq!(
        error(&[two]),
        e(
            "cosmosis-manifest-extra-value",
            "`package` takes one value, but `\"b\"` follows it",
            "remove `\"b\"`"
        )
    );
}

#[test]
fn the_file_holds_one_package_node() {
    assert_eq!(
        errors(&[]),
        [e(
            "cosmosis-manifest-no-package",
            "cosmosis.noll has no `package` node",
            "a manifest is one node: `package \"my-project\" { null-format 1 }`"
        )]
    );
    assert_eq!(
        errors(&[field("null-format", w("1"))]),
        [
            e(
                "cosmosis-manifest-no-package",
                "cosmosis.noll has no `package` node",
                "a manifest is one node: `package \"my-project\" { null-format 1 }`"
            ),
            e(
                "cosmosis-manifest-unknown-node",
                "`null-format` is not a manifest node; cosmosis.noll holds one `package` node",
                "`null-format` is a field of the package: move it inside `package \"…\" { }`"
            ),
        ]
    );
    assert_eq!(
        error(&[package("a", vec![]), Node::new("pakage").arg(s("b"))]),
        e(
            "cosmosis-manifest-unknown-node",
            "`pakage` is not a manifest node; cosmosis.noll holds one `package` node",
            "did you mean `package`?"
        )
    );
    assert_eq!(
        error(&[package("a", vec![]), Node::new("scene")]).2,
        "remove it, or move it into another file"
    );
    assert_eq!(
        error(&[package("a", vec![]), package("b", vec![])]),
        e(
            "cosmosis-manifest-repeated-package",
            "cosmosis.noll has a second `package` node",
            "a manifest describes one package: remove this one or merge it into the first"
        )
    );
}

#[test]
fn a_field_takes_one_value_and_no_block() {
    assert_eq!(
        error(&[Node::new("package")
            .arg(s("p"))
            .child(field("null-format", w("1")).arg(w("2")))]),
        e(
            "cosmosis-manifest-extra-value",
            "`null-format` takes one value, but `2` follows it",
            "remove `2`"
        )
    );
    assert_eq!(
        error(&[package(
            "p",
            vec![field("kind", w("rig")).prop("type", w("x"))]
        )]),
        e(
            "cosmosis-manifest-extra-value",
            "`kind` takes one value, but `type=x` follows it",
            "remove `type=x`"
        )
    );
    assert_eq!(
        error(&[package(
            "p",
            vec![Node::new("kind").child(Node::new("project"))]
        )]),
        e(
            "cosmosis-manifest-unexpected-block",
            "`kind` takes no `{ }` block",
            "write it as `kind project`"
        )
    );
    assert_eq!(
        error(&[package("p", vec![Node::new("license")])]),
        e(
            "cosmosis-manifest-no-value",
            "`license` has no value",
            "write it as `license \"MIT\"`"
        )
    );
}

fn deps(children: Vec<Node>) -> Node {
    let mut d = Node::new("dependencies").block();
    for c in children {
        d = d.child(c);
    }
    package("p", vec![d])
}

#[test]
fn dependency_errors() {
    assert_eq!(
        error(&[deps(vec![Node::new("oak").arg(s("^x"))])]),
        e(
            "cosmosis-manifest-requirement",
            "\"^x\" is not a version requirement (dependency `oak`)",
            "write versions such as \"^1.2\", \"~1.2.3\", \"=1.0.0\" or \">=1, <2\""
        )
    );
    assert_eq!(
        error(&[deps(vec![Node::new("oak").arg(w("^1.2"))])]),
        e(
            "cosmosis-manifest-type",
            "the dependency `oak` takes a quoted version requirement, not `^1.2`",
            "write it as `oak \"^1.0\"` or `oak path=\"../oak\"`"
        )
    );
    assert_eq!(
        error(&[deps(vec![Node::new("oak")])]),
        e(
            "cosmosis-manifest-dependency-source",
            "the dependency `oak` says neither which versions nor where it is",
            "give a version requirement, `oak \"^1.0\"`, or a folder, `oak path=\"../oak\"`"
        )
    );
    assert_eq!(
        error(&[deps(vec![Node::new("oak").prop("pth", s("../Oak.sau"))])]),
        e(
            "cosmosis-manifest-dependency-property",
            "the dependency `oak` has no property `pth`",
            "did you mean `path`?"
        )
    );
    assert_eq!(
        error(&[deps(vec![
            Node::new("oak").arg(s("^1")),
            Node::new("oak").arg(s("^2"))
        ])]),
        e(
            "cosmosis-manifest-repeated-dependency",
            "the dependency `oak` is listed twice",
            "keep one of them"
        )
    );
    assert_eq!(
        error(&[deps(vec![Node::new("My_Trees").arg(s("^1"))])]),
        e(
            "cosmosis-manifest-bad-name",
            "the dependency name \"My_Trees\" is not kebab-case",
            "names are lower-case words joined by `-`: \"my-trees\""
        )
    );
    assert_eq!(
        error(&[package("p", vec![]).prop("dependencies", w("oak"))]),
        e(
            "cosmosis-manifest-type",
            "`dependencies` is a block of dependencies, not `oak`",
            "write it as `dependencies { other-package \"^1.0\" }`"
        )
    );
    // A version requirement and a folder together are allowed: the folder for now, the
    // requirement for when the package comes from the registry.
    let both = Manifest::read(&[deps(vec![
        Node::new("oak").arg(s("^1")).prop("path", s("../Oak.sau")),
    ])]);
    assert_eq!(
        both.unwrap().dependencies[0].path.as_deref(),
        Some("../Oak.sau")
    );
}

#[test]
fn every_error_is_reported_not_only_the_first() {
    let p = Node::new("package")
        .arg(s("Bad Name"))
        .child(field("kind", w("scene")))
        .child(field("vresion", w("0.1.0")));
    let codes: Vec<String> = errors(&[p]).into_iter().map(|e| e.0).collect();
    assert_eq!(
        codes,
        [
            "cosmosis-manifest-bad-name",
            "cosmosis-manifest-kind",
            "cosmosis-manifest-unknown-field",
            "cosmosis-manifest-no-null-format"
        ]
    );
}

#[test]
fn names_and_requirements() {
    for good in ["harbor", "harbor-at-dusk", "a1-b2", "cosmosis-materials"] {
        assert!(is_kebab(good), "{good}");
    }
    for bad in ["", "Harbor", "-a", "a-", "a--b", "a_b", "1a", "a b", "café"] {
        assert!(!is_kebab(bad), "{bad}");
    }
    assert_eq!(
        to_kebab("Harbor at Dusk").as_deref(),
        Some("harbor-at-dusk")
    );
    assert_eq!(to_kebab("Cube.001").as_deref(), Some("cube-001"));
    assert_eq!(to_kebab("42"), None);
    for good in ["^1.2", "~1.2.3", "=1.0.0", ">=1, <2", "1.2", "*", ">= 1.2"] {
        assert!(is_requirement(good), "{good}");
    }
    for bad in ["", "^", "^x", "1.2.3.4", "1..2", ">=1,", "latest"] {
        assert!(!is_requirement(bad), "{bad}");
    }
}
