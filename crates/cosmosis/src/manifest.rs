//! The manifest, `cosmosis.noll` (Cosmósis spec, section 4; Sau's decisions S37 and S45): its
//! data model, its fields and every error reading it can give.
//!
//! The manifest is read from a parser-neutral [`Node`] tree, so this module needs no parser;
//! Sau task 1a-11 connects Null's parser to [`Manifest::read`]. A manifest is one node:
//!
//! ```text
//! package "harbor-at-dusk" {
//!   kind project            // optional: project (the default), material, node-group, model,
//!                           //   rig, template or library
//!   version 0.1.0           // optional: the package's own version, major.minor.patch
//!   sau 0.1                 // optional: the Sau it was written for, major.minor[.patch]
//!   null-format 1           // required: the Null format its files are written in (S45)
//!   license "All rights reserved"   // optional
//!   dependencies {          // optional
//!     cosmosis-materials "^1.2"     // a version requirement, for the registry
//!     my-trees path="../Trees.sau"  // a folder
//!   }
//! }
//! ```
//!
//! The name and `null-format` are all a manifest needs. Any field may also be written as a
//! property on the package's line (`package "harbor" null-format=1`), as everywhere in Null
//! (Null spec, section 5). Reading is strict (S24): a field not listed here, a value of the
//! wrong form or a field given twice is an error, with a suggestion where one exists.

use std::collections::HashMap;
use std::fmt;

use crate::diagnostic::{Diagnostic, LineCol, nearest};
use crate::tree::{Node, Property, Span, Value, ValueKind};

/// The manifest's file name, at the root of every project (S37).
pub const MANIFEST_FILE: &str = "cosmosis.noll";

/// The newest Null format this Cosmósis reads. Older formats are migrated (S45); none exist yet.
pub const NEWEST_NULL_FORMAT: u32 = 1;

/// The package's fields, in canonical order.
pub const FIELDS: [&str; 6] = [
    "kind",
    "version",
    "sau",
    "null-format",
    "license",
    "dependencies",
];

/// What kind of package a manifest describes (Cosmósis spec, section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A scene project; it also needs `project.noll`. The default.
    Project,
    /// Shared materials.
    Material,
    /// Shared node groups.
    NodeGroup,
    /// Models.
    Model,
    /// Rigs.
    Rig,
    /// A template for `sau new`.
    Template,
    /// A library of several kinds.
    Library,
}

impl Kind {
    /// Every kind, in the spec's order.
    pub const ALL: [Kind; 7] = [
        Kind::Project,
        Kind::Material,
        Kind::NodeGroup,
        Kind::Model,
        Kind::Rig,
        Kind::Template,
        Kind::Library,
    ];

    /// The kind as written in the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Project => "project",
            Kind::Material => "material",
            Kind::NodeGroup => "node-group",
            Kind::Model => "model",
            Kind::Rig => "rig",
            Kind::Template => "template",
            Kind::Library => "library",
        }
    }

    /// The kind written as `word`, if it is one.
    pub fn parse(word: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == word)
    }
}

/// A version: `0.1.0`; for the `sau` field the patch may be left out (`0.1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    /// Major.
    pub major: u64,
    /// Minor.
    pub minor: u64,
    /// Patch, when written.
    pub patch: Option<u64>,
}

impl Version {
    /// Reads `major.minor.patch`, or `major.minor` when `patch_optional`; whole numbers only.
    pub fn parse(text: &str, patch_optional: bool) -> Option<Version> {
        let parts: Vec<&str> = text.split('.').collect();
        let number = |s: &str| {
            let plain = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
            // No leading zeros, as semantic versioning says.
            (plain && (s == "0" || !s.starts_with('0')))
                .then(|| s.parse().ok())
                .flatten()
        };
        match parts.as_slice() {
            [a, b, c] => Some(Version {
                major: number(a)?,
                minor: number(b)?,
                patch: Some(number(c)?),
            }),
            [a, b] if patch_optional => Some(Version {
                major: number(a)?,
                minor: number(b)?,
                patch: None,
            }),
            _ => None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        if let Some(p) = self.patch {
            write!(f, ".{p}")?;
        }
        Ok(())
    }
}

/// A package the manifest depends on. It names at least one of a version requirement, for
/// the registry (Genesis-Preim, later), and a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    /// The package's name.
    pub name: String,
    /// The versions accepted, as written: `^1.2`, `>=1, <2`.
    pub requirement: Option<String>,
    /// The package's folder, relative to this project: `../Trees.sau`.
    pub path: Option<String>,
}

/// A manifest that has been read and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The package's name, kebab-case (S16).
    pub name: String,
    /// The package's kind; `project` unless written.
    pub kind: Kind,
    /// The package's own version.
    pub version: Option<Version>,
    /// The Sau version it was written for.
    pub sau: Option<Version>,
    /// The Null format its files are written in (S45).
    pub null_format: u32,
    /// Its licence, as written.
    pub license: Option<String>,
    /// The packages it depends on, in the order written.
    pub dependencies: Vec<Dependency>,
}

/// Something wrong in a manifest, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestError {
    /// What is wrong.
    pub kind: ErrorKind,
    /// Where: the value at fault, or the node's name.
    pub span: Span,
}

/// Everything that can be wrong in a manifest. Each has a stable code ([`ManifestError::code`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The file has no `package` node.
    NoPackage,
    /// A top-level node other than `package`.
    UnknownNode {
        /// The node's name.
        name: String,
    },
    /// A second `package` node.
    RepeatedPackage,
    /// `package` with no name.
    NoName,
    /// A name that is not a quoted string.
    NameNotString {
        /// The name as written.
        found: String,
    },
    /// A package or dependency name that is not kebab-case.
    BadName {
        /// `"package"` or `"dependency"`.
        what: &'static str,
        /// The name.
        name: String,
        /// The kebab-case spelling, when there is one.
        suggestion: Option<String>,
    },
    /// A field the package does not have.
    UnknownField {
        /// The field as written.
        field: String,
        /// The nearest field, when it is close.
        suggestion: Option<&'static str>,
    },
    /// A field written twice.
    RepeatedField {
        /// The field.
        field: String,
        /// Where it was first written.
        first: Span,
    },
    /// No `null-format`.
    NoNullFormat,
    /// A field with no value.
    NoValue {
        /// The field.
        field: String,
    },
    /// A field with more than its one value, or with properties.
    ExtraValue {
        /// The field.
        field: String,
        /// What follows its value.
        found: String,
    },
    /// A field written with a `{ }` block that takes none.
    UnexpectedBlock {
        /// The field.
        field: String,
    },
    /// A value of the wrong form: a word where a string goes, or the reverse.
    WrongType {
        /// The field.
        field: String,
        /// What it should be, as a phrase: "a quoted string".
        expected: &'static str,
        /// The value as written.
        found: String,
    },
    /// `null-format` that is not a whole number from 1.
    BadNullFormat {
        /// The value as written.
        found: String,
    },
    /// A `null-format` newer than [`NEWEST_NULL_FORMAT`].
    NewerNullFormat {
        /// The format.
        found: u32,
    },
    /// A kind that does not exist.
    UnknownKind {
        /// The kind as written.
        found: String,
        /// The nearest kind, when it is close.
        suggestion: Option<&'static str>,
    },
    /// `version` or `sau` that is not a version.
    BadVersion {
        /// `version` or `sau`.
        field: &'static str,
        /// The value as written.
        found: String,
    },
    /// A dependency's value of the wrong form.
    DependencyType {
        /// The dependency.
        name: String,
        /// What it should be, as a phrase.
        expected: &'static str,
        /// The value as written.
        found: String,
    },
    /// A dependency written with a `{ }` block.
    DependencyBlock {
        /// The dependency.
        name: String,
    },
    /// A dependency with neither a version requirement nor a path.
    NoSource {
        /// The dependency.
        name: String,
    },
    /// A dependency's version requirement that cannot be read.
    BadRequirement {
        /// The dependency.
        name: String,
        /// The requirement as written.
        found: String,
    },
    /// A dependency property other than `path`.
    UnknownDependencyProperty {
        /// The dependency.
        name: String,
        /// The property.
        key: String,
        /// `path`, when it is close.
        suggestion: Option<&'static str>,
    },
    /// A dependency listed twice.
    RepeatedDependency {
        /// The dependency.
        name: String,
    },
}

/// How each field is written, for messages.
fn example(field: &str) -> &'static str {
    match field {
        "kind" => "`kind project`",
        "version" => "`version 0.1.0`",
        "sau" => "`sau 0.1`",
        "null-format" => "`null-format 1`",
        "license" => "`license \"MIT\"`",
        _ => "`dependencies { other-package \"^1.0\" }`",
    }
}

fn kinds_list() -> String {
    let names: Vec<&str> = Kind::ALL.iter().map(|k| k.as_str()).collect();
    format!(
        "{} and {}",
        names[..names.len() - 1].join(", "),
        names[names.len() - 1]
    )
}

impl ManifestError {
    fn new(kind: ErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// The stable code.
    pub fn code(&self) -> &'static str {
        use ErrorKind::*;
        match self.kind {
            NoPackage => "cosmosis-manifest-no-package",
            UnknownNode { .. } => "cosmosis-manifest-unknown-node",
            RepeatedPackage => "cosmosis-manifest-repeated-package",
            NoName => "cosmosis-manifest-no-name",
            NameNotString { .. } => "cosmosis-manifest-name-type",
            BadName { .. } => "cosmosis-manifest-bad-name",
            UnknownField { .. } => "cosmosis-manifest-unknown-field",
            RepeatedField { .. } => "cosmosis-manifest-repeated-field",
            NoNullFormat => "cosmosis-manifest-no-null-format",
            NoValue { .. } => "cosmosis-manifest-no-value",
            ExtraValue { .. } => "cosmosis-manifest-extra-value",
            UnexpectedBlock { .. } => "cosmosis-manifest-unexpected-block",
            WrongType { .. } => "cosmosis-manifest-type",
            BadNullFormat { .. } => "cosmosis-manifest-null-format",
            NewerNullFormat { .. } => "cosmosis-manifest-newer-format",
            UnknownKind { .. } => "cosmosis-manifest-kind",
            BadVersion { .. } => "cosmosis-manifest-version",
            DependencyType { .. } => "cosmosis-manifest-type",
            DependencyBlock { .. } => "cosmosis-manifest-unexpected-block",
            NoSource { .. } => "cosmosis-manifest-dependency-source",
            BadRequirement { .. } => "cosmosis-manifest-requirement",
            UnknownDependencyProperty { .. } => "cosmosis-manifest-dependency-property",
            RepeatedDependency { .. } => "cosmosis-manifest-repeated-dependency",
        }
    }

    /// What is wrong, in one sentence.
    pub fn message(&self) -> String {
        use ErrorKind::*;
        match &self.kind {
            NoPackage => format!("{MANIFEST_FILE} has no `package` node"),
            UnknownNode { name } => {
                format!("`{name}` is not a manifest node; {MANIFEST_FILE} holds one `package` node")
            }
            RepeatedPackage => format!("{MANIFEST_FILE} has a second `package` node"),
            NoName => "the package has no name".into(),
            NameNotString { found } => format!("the package name `{found}` is not a quoted string"),
            BadName { what, name, .. } => format!("the {what} name {name:?} is not kebab-case"),
            UnknownField { field, .. } => format!("the package has no field `{field}`"),
            RepeatedField { field, .. } => format!("`{field}` is written twice"),
            NoNullFormat => "the package has no `null-format`".into(),
            NoValue { field } => format!("`{field}` has no value"),
            ExtraValue { field, found } => {
                format!("`{field}` takes one value, but `{found}` follows it")
            }
            UnexpectedBlock { field } => format!("`{field}` takes no `{{ }}` block"),
            WrongType {
                field,
                expected,
                found,
            } => {
                format!("`{field}` is {expected}, not `{found}`")
            }
            BadNullFormat { found } => format!("`null-format {found}` is not a format number"),
            NewerNullFormat { found } => format!(
                "the project is written in Null format {found}, newer than this Cosmósis reads \
                 (format {NEWEST_NULL_FORMAT})"
            ),
            UnknownKind { found, .. } => format!("`{found}` is not a package kind"),
            BadVersion { field, found } => format!("`{field} {found}` is not a version"),
            DependencyType {
                name,
                expected,
                found,
            } => {
                format!("the dependency `{name}` takes {expected}, not `{found}`")
            }
            DependencyBlock { name } => format!("the dependency `{name}` takes no `{{ }}` block"),
            NoSource { name } => {
                format!("the dependency `{name}` says neither which versions nor where it is")
            }
            BadRequirement { name, found } => {
                format!("{found:?} is not a version requirement (dependency `{name}`)")
            }
            UnknownDependencyProperty { name, key, .. } => {
                format!("the dependency `{name}` has no property `{key}`")
            }
            RepeatedDependency { name } => format!("the dependency `{name}` is listed twice"),
        }
    }

    /// How to fix it. `source` is the manifest's text, for pointing at an earlier line.
    pub fn help(&self, source: &[u8]) -> String {
        use ErrorKind::*;
        match &self.kind {
            NoPackage => {
                "a manifest is one node: `package \"my-project\" { null-format 1 }`".into()
            }
            UnknownNode { name } if FIELDS.contains(&name.as_str()) => {
                format!("`{name}` is a field of the package: move it inside `package \"…\" {{ }}`")
            }
            UnknownNode { name } => match nearest(name, &["package"]) {
                Some(_) => "did you mean `package`?".into(),
                None => "remove it, or move it into another file".into(),
            },
            RepeatedPackage => {
                "a manifest describes one package: remove this one or merge it into the first"
                    .into()
            }
            NoName => {
                "write it as a quoted string after `package`: `package \"my-project\"`".into()
            }
            NameNotString { found } => format!("write it in quotes: `package \"{found}\"`"),
            BadName {
                suggestion: Some(s),
                ..
            } => {
                format!("names are lower-case words joined by `-`: {s:?}")
            }
            BadName {
                suggestion: None, ..
            } => "names are lower-case letters and digits joined by \
                 `-`, starting with a letter, such as \"harbor-at-dusk\""
                .into(),
            UnknownField { field, .. } if field == "name" => {
                "the name is the quoted string after `package`: `package \"my-project\"`".into()
            }
            UnknownField {
                suggestion: Some(s),
                ..
            } => format!("did you mean `{s}`?"),
            UnknownField { .. } => {
                "the fields are kind, version, sau, null-format, license and dependencies".into()
            }
            RepeatedField { first, .. } if *first != Span::default() => format!(
                "remove one of them; the first is on line {}",
                LineCol::of(source, first.start).line
            ),
            RepeatedField { .. } => "remove one of them".into(),
            NoNullFormat => {
                format!(
                    "add `null-format {NEWEST_NULL_FORMAT}`, the Null format its files are written in"
                )
            }
            NoValue { field } | UnexpectedBlock { field } => {
                format!("write it as {}", example(field))
            }
            ExtraValue { found, .. } => format!("remove `{found}`"),
            WrongType { field, .. } => format!("write it as {}", example(field)),
            DependencyType { name, .. } | DependencyBlock { name } => {
                format!("write it as `{name} \"^1.0\"` or `{name} path=\"../{name}\"`")
            }
            BadNullFormat { .. } => format!(
                "formats are whole numbers from 1; this Cosmósis reads up to format {NEWEST_NULL_FORMAT}"
            ),
            NewerNullFormat { .. } => {
                "open it with a newer Sau: older formats are migrated, newer ones cannot be".into()
            }
            UnknownKind {
                suggestion: Some(s),
                ..
            } => format!("did you mean `{s}`?"),
            UnknownKind { .. } => format!("the kinds are {}", kinds_list()),
            BadVersion { field: "sau", .. } => {
                "write major.minor or major.minor.patch in whole numbers, such as `sau 0.1`".into()
            }
            BadVersion { .. } => {
                "write major.minor.patch in whole numbers, such as `version 0.1.0`".into()
            }
            NoSource { name } => format!(
                "give a version requirement, `{name} \"^1.0\"`, or a folder, `{name} path=\"../{name}\"`"
            ),
            BadRequirement { .. } => {
                "write versions such as \"^1.2\", \"~1.2.3\", \"=1.0.0\" or \">=1, <2\"".into()
            }
            UnknownDependencyProperty {
                suggestion: Some(s),
                ..
            } => format!("did you mean `{s}`?"),
            UnknownDependencyProperty { .. } => {
                "a dependency takes a quoted version requirement and `path=`".into()
            }
            RepeatedDependency { .. } => "keep one of them".into(),
        }
    }

    /// The error as a diagnostic in [`MANIFEST_FILE`], positioned in `source`.
    pub fn to_diagnostic(&self, source: &[u8]) -> Diagnostic {
        Diagnostic::error(self.code(), self.message())
            .in_file(MANIFEST_FILE)
            .at(source, self.span)
            .with_help(self.help(source))
    }
}

/// True for a kebab-case name: lower-case ASCII letters and digits in words joined by single
/// `-`, starting with a letter (S16).
pub fn is_kebab(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.split('-').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// The kebab-case spelling of a name, if one exists: "Harbor at Dusk" gives "harbor-at-dusk".
pub fn to_kebab(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let kebab = words.join("-");
    is_kebab(&kebab).then_some(kebab)
}

/// True for a version requirement: comma-separated comparisons, each an optional operator
/// (`^`, `~`, `=`, `>`, `>=`, `<`, `<=`) and a version of one to three whole numbers, or `*`.
pub fn is_requirement(text: &str) -> bool {
    text.split(',').all(|part| {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        let rest = [">=", "<=", "^", "~", "=", ">", "<"]
            .iter()
            .find_map(|op| part.strip_prefix(op))
            .unwrap_or(part)
            .trim_start();
        let numbers: Vec<&str> = rest.split('.').collect();
        (1..=3).contains(&numbers.len())
            && numbers
                .iter()
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// A field as written: a property on the package's line or a child node.
enum Field<'a> {
    Property(&'a Property),
    Node(&'a Node),
}

impl Field<'_> {
    fn name(&self) -> &str {
        match self {
            Field::Property(p) => &p.key,
            Field::Node(n) => &n.name,
        }
    }

    fn span(&self) -> Span {
        match self {
            Field::Property(p) => p.key_span,
            Field::Node(n) => n.name_span,
        }
    }

    /// The field's one value, reporting a missing value, extra values or a block.
    fn value(&self, errors: &mut Vec<ManifestError>) -> Option<&Value> {
        let node = match self {
            Field::Property(p) => return Some(&p.value),
            Field::Node(n) => n,
        };
        let field = node.name.clone();
        if node.children.is_some() {
            errors.push(ManifestError::new(
                ErrorKind::UnexpectedBlock { field },
                node.name_span,
            ));
            return None;
        }
        let mut args = node.arguments();
        let first = args.next();
        if let Some(extra) = args.next() {
            errors.push(ManifestError::new(
                ErrorKind::ExtraValue {
                    field: field.clone(),
                    found: extra.describe(),
                },
                extra.span,
            ));
        }
        if let Some(p) = node.properties().next() {
            errors.push(ManifestError::new(
                ErrorKind::ExtraValue {
                    field: field.clone(),
                    found: format!("{}={}", p.key, p.value.describe()),
                },
                p.key_span,
            ));
        }
        if first.is_none() {
            errors.push(ManifestError::new(
                ErrorKind::NoValue { field },
                node.name_span,
            ));
        }
        first
    }
}

fn word(v: &Value) -> Option<&str> {
    match &v.kind {
        ValueKind::Word(w) => Some(w),
        _ => None,
    }
}

fn string(v: &Value) -> Option<&str> {
    match &v.kind {
        ValueKind::Str(s) => Some(s),
        _ => None,
    }
}

fn wrong_type(field: &str, expected: &'static str, v: &Value) -> ManifestError {
    ManifestError::new(
        ErrorKind::WrongType {
            field: field.into(),
            expected,
            found: v.describe(),
        },
        v.span,
    )
}

fn dependency_type(name: &str, expected: &'static str, v: &Value) -> ManifestError {
    ManifestError::new(
        ErrorKind::DependencyType {
            name: name.into(),
            expected,
            found: v.describe(),
        },
        v.span,
    )
}

impl Manifest {
    /// Reads and checks a manifest from the nodes of `cosmosis.noll`, returning every error
    /// found, not only the first.
    pub fn read(nodes: &[Node]) -> Result<Manifest, Vec<ManifestError>> {
        let mut errors = Vec::new();
        let mut package = None;
        for node in nodes {
            match (node.name.as_str(), package) {
                ("package", None) => package = Some(node),
                ("package", Some(_)) => errors.push(ManifestError::new(
                    ErrorKind::RepeatedPackage,
                    node.name_span,
                )),
                (name, _) => errors.push(ManifestError::new(
                    ErrorKind::UnknownNode { name: name.into() },
                    node.name_span,
                )),
            }
        }
        let Some(package) = package else {
            errors.insert(0, ManifestError::new(ErrorKind::NoPackage, Span::default()));
            return Err(errors);
        };

        let name = read_name(package, &mut errors);
        let mut manifest = Manifest {
            name: name.unwrap_or_default(),
            kind: Kind::Project,
            version: None,
            sau: None,
            null_format: 0,
            license: None,
            dependencies: Vec::new(),
        };

        let fields = package
            .properties()
            .map(Field::Property)
            .chain(package.children.iter().flatten().map(Field::Node));
        let mut seen: HashMap<String, Span> = HashMap::new();
        for field in fields {
            let span = field.span();
            if let Some(&first) = seen.get(field.name()) {
                errors.push(ManifestError::new(
                    ErrorKind::RepeatedField {
                        field: field.name().into(),
                        first,
                    },
                    span,
                ));
                continue;
            }
            seen.insert(field.name().into(), span);
            manifest.read_field(&field, &mut errors);
        }
        if !seen.contains_key("null-format") {
            errors.push(ManifestError::new(
                ErrorKind::NoNullFormat,
                package.name_span,
            ));
        }
        if errors.is_empty() {
            Ok(manifest)
        } else {
            Err(errors)
        }
    }

    fn read_field(&mut self, field: &Field<'_>, errors: &mut Vec<ManifestError>) {
        let name = field.name();
        if name == "dependencies" {
            match field {
                Field::Node(n) if n.children.is_some() => {
                    if let Some(extra) = n.entries.first() {
                        let found = match extra {
                            crate::tree::Entry::Argument(v) => v.describe(),
                            crate::tree::Entry::Property(p) => {
                                format!("{}={}", p.key, p.value.describe())
                            }
                        };
                        errors.push(ManifestError::new(
                            ErrorKind::ExtraValue {
                                field: name.into(),
                                found,
                            },
                            n.name_span,
                        ));
                    }
                    self.dependencies =
                        read_dependencies(n.children.as_deref().unwrap_or_default(), errors);
                }
                Field::Node(n) => match n.arguments().next() {
                    Some(v) => errors.push(wrong_type(name, "a block of dependencies", v)),
                    None => errors.push(ManifestError::new(
                        ErrorKind::NoValue { field: name.into() },
                        n.name_span,
                    )),
                },
                Field::Property(p) => {
                    errors.push(wrong_type(name, "a block of dependencies", &p.value))
                }
            }
            return;
        }
        if !FIELDS.contains(&name) {
            let suggestion = nearest(name, &FIELDS);
            errors.push(ManifestError::new(
                ErrorKind::UnknownField {
                    field: name.into(),
                    suggestion,
                },
                field.span(),
            ));
            return;
        }
        let Some(value) = field.value(errors) else {
            return;
        };
        match name {
            "kind" => match word(value) {
                Some(w) => match Kind::parse(w) {
                    Some(k) => self.kind = k,
                    None => {
                        let all: Vec<&str> = Kind::ALL.iter().map(|k| k.as_str()).collect();
                        let suggestion = nearest(w, &all);
                        errors.push(ManifestError::new(
                            ErrorKind::UnknownKind {
                                found: w.into(),
                                suggestion,
                            },
                            value.span,
                        ));
                    }
                },
                None => errors.push(wrong_type(name, "a package kind, written bare", value)),
            },
            "version" | "sau" => {
                let field = if name == "sau" { "sau" } else { "version" };
                match word(value) {
                    Some(w) => match Version::parse(w, field == "sau") {
                        Some(v) if field == "sau" => self.sau = Some(v),
                        Some(v) => self.version = Some(v),
                        None => errors.push(ManifestError::new(
                            ErrorKind::BadVersion {
                                field,
                                found: w.into(),
                            },
                            value.span,
                        )),
                    },
                    None => errors.push(wrong_type(name, "a version, written bare", value)),
                }
            }
            "null-format" => match word(value) {
                Some(w) => {
                    // Whole numbers from 1, in plain digits: no sign, no leading zero.
                    let plain = !w.is_empty()
                        && w.bytes().all(|b| b.is_ascii_digit())
                        && !w.starts_with('0');
                    match w.parse::<u32>() {
                        Ok(n) if plain && n > NEWEST_NULL_FORMAT => {
                            errors.push(ManifestError::new(
                                ErrorKind::NewerNullFormat { found: n },
                                value.span,
                            ));
                        }
                        Ok(n) if plain => self.null_format = n,
                        _ => errors.push(ManifestError::new(
                            ErrorKind::BadNullFormat { found: w.into() },
                            value.span,
                        )),
                    }
                }
                None => errors.push(wrong_type(name, "a whole number, written bare", value)),
            },
            _ => match string(value) {
                Some(s) => self.license = Some(s.into()),
                None => errors.push(wrong_type(name, "a quoted string", value)),
            },
        }
    }
}

/// The package's name: its first value, a quoted kebab-case string.
fn read_name(package: &Node, errors: &mut Vec<ManifestError>) -> Option<String> {
    let mut args = package.arguments();
    let first = args.next();
    if let Some(extra) = args.next() {
        errors.push(ManifestError::new(
            ErrorKind::ExtraValue {
                field: "package".into(),
                found: extra.describe(),
            },
            extra.span,
        ));
    }
    let Some(value) = first else {
        errors.push(ManifestError::new(ErrorKind::NoName, package.name_span));
        return None;
    };
    let Some(name) = string(value) else {
        errors.push(ManifestError::new(
            ErrorKind::NameNotString {
                found: value.describe(),
            },
            value.span,
        ));
        return None;
    };
    if !is_kebab(name) {
        errors.push(ManifestError::new(
            ErrorKind::BadName {
                what: "package",
                name: name.into(),
                suggestion: to_kebab(name),
            },
            value.span,
        ));
        return None;
    }
    Some(name.into())
}

/// The `dependencies { }` block: one node per package, named by it.
fn read_dependencies(nodes: &[Node], errors: &mut Vec<ManifestError>) -> Vec<Dependency> {
    let mut out: Vec<Dependency> = Vec::new();
    for node in nodes {
        let name = node.name.clone();
        let before = errors.len();
        if !is_kebab(&name) {
            errors.push(ManifestError::new(
                ErrorKind::BadName {
                    what: "dependency",
                    name: name.clone(),
                    suggestion: to_kebab(&name),
                },
                node.name_span,
            ));
        }
        if out.iter().any(|d| d.name == name) {
            errors.push(ManifestError::new(
                ErrorKind::RepeatedDependency { name },
                node.name_span,
            ));
            continue;
        }
        if node.children.is_some() {
            errors.push(ManifestError::new(
                ErrorKind::DependencyBlock { name: name.clone() },
                node.name_span,
            ));
        }
        let mut dep = Dependency {
            name: name.clone(),
            requirement: None,
            path: None,
        };
        let mut args = node.arguments();
        if let Some(v) = args.next() {
            match string(v) {
                Some(r) if is_requirement(r) => dep.requirement = Some(r.into()),
                Some(r) => errors.push(ManifestError::new(
                    ErrorKind::BadRequirement {
                        name: name.clone(),
                        found: r.into(),
                    },
                    v.span,
                )),
                None => errors.push(dependency_type(&name, "a quoted version requirement", v)),
            }
        }
        if let Some(extra) = args.next() {
            errors.push(ManifestError::new(
                ErrorKind::ExtraValue {
                    field: name.clone(),
                    found: extra.describe(),
                },
                extra.span,
            ));
        }
        for p in node.properties() {
            if p.key != "path" {
                let suggestion = nearest(&p.key, &["path"]);
                errors.push(ManifestError::new(
                    ErrorKind::UnknownDependencyProperty {
                        name: name.clone(),
                        key: p.key.clone(),
                        suggestion,
                    },
                    p.key_span,
                ));
                continue;
            }
            match string(&p.value) {
                Some(s) if !s.is_empty() && dep.path.is_none() => dep.path = Some(s.into()),
                Some(s) if !s.is_empty() => errors.push(ManifestError::new(
                    ErrorKind::RepeatedField {
                        field: format!("{name} path"),
                        first: Span::default(),
                    },
                    p.key_span,
                )),
                _ => errors.push(dependency_type(
                    &name,
                    "a quoted folder path as `path=`",
                    &p.value,
                )),
            }
        }
        if errors.len() == before && dep.requirement.is_none() && dep.path.is_none() {
            errors.push(ManifestError::new(
                ErrorKind::NoSource { name },
                node.name_span,
            ));
        }
        out.push(dep);
    }
    out
}
