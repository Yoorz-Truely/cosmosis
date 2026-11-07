//! Cosmósis, Sau's project framework (`docs/spec/cosmosis-v0.md`): the project folder's
//! layout, the `cosmosis.noll` manifest and the diagnostics `sau check` reports.
//!
//! Null's parser is not built yet (Sau task 1a-03). Until it is, the layout is checked
//! without reading any file's meaning, and the manifest is read from a parser-neutral
//! [`tree::Node`] tree that the parser will fill (Sau task 1a-11).

pub mod diagnostic;
pub mod layout;
pub mod manifest;
pub mod tree;

pub use diagnostic::{Diagnostic, Severity};
pub use manifest::{Manifest, ManifestError};
