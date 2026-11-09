//! Cosmósis, Sau's project framework (`docs/spec/cosmosis-v0.md`): the project folder's
//! layout, the `cosmosis.noll` manifest and the diagnostics `sau check` reports.
//!
//! Files are read with Null's parser ([`source`]) into a parser-neutral [`tree::Node`]
//! tree, from which the manifest is read ([`manifest::Manifest::parse`], Sau task 1a-11).
//! What each file means beyond its syntax is Null's checker's (Sau task 1a-06).

pub mod blender_project;
pub mod data_files;
pub mod diagnostic;
pub mod ids;
pub mod layout;
pub mod manifest;
pub mod media;
pub mod schema_check;
pub mod source;
pub mod templates;
pub mod tree;

pub use diagnostic::{Diagnostic, Severity};
pub use manifest::{Manifest, ManifestError};
