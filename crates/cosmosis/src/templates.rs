//! The files Cosmósis ships for a new project (Sau task 1d-04b; Cosmósis spec, section 2,
//! "Version control"), from the repository's `templates/project/`. Writing them into a new
//! project is `sau new`'s.

/// The project's `.gitignore`: Sau's cache but its ID map, `view.noll` and `renders/` (S66).
pub const GITIGNORE: &str = include_str!("../../../templates/project/gitignore");

/// The project's `.gitattributes`: `media/` and `data/` in Git LFS.
pub const GITATTRIBUTES: &str = include_str!("../../../templates/project/gitattributes");
