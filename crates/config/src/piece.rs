//! A piece's file as text, and the place it was read from. A lifecycle and a Project (a set of
//! repositories) are pieces, and take the places every piece takes.
//!
//! **Text in, not directories**, as a workflow's is, so a test drives the reading with no file.
//! Where the folders are and which are read is Fleet's.

use std::path::{Path, PathBuf};

use core_model::{PieceSource, ScopeId};

/// One piece file, as text, and the place it was read from.
///
/// **No way to say a file is Armada's.** What Armada carries is a value built in `core-model`, and
/// a file on a machine cannot pass itself off as it.
#[derive(Debug, Clone)]
pub struct Written {
    source: PieceSource,
    path: PathBuf,
    text: String,
    home_of: Option<ScopeId>,
}

impl Written {
    /// A file read from Kit's folder.
    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written { source: PieceSource::Kit, path, text, home_of: None }
    }

    /// A file read from a repository's own `.armada/`.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written { source: PieceSource::Repository, path, text, home_of: None }
    }

    /// A file read from another member of the Project `project`, for a repository that is a member
    /// of it too. Only a file scoped to that Project is the reader's to take; the member's own, and
    /// those scoped to another Project, stay the member's.
    pub fn in_home_of(project: ScopeId, path: PathBuf, text: String) -> Written {
        Written { source: PieceSource::Repository, path, text, home_of: Some(project) }
    }

    pub fn source(&self) -> PieceSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The Project whose other member this was read from, where it was.
    pub fn home_of(&self) -> Option<&ScopeId> {
        self.home_of.as_ref()
    }
}

/// A file set aside, and why. **Kept so a person who wrote one is told where it went**, as a
/// workflow that does not fit is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    source: PieceSource,
    path: PathBuf,
    said: String,
}

impl LeftOut {
    pub(crate) fn new(source: PieceSource, path: PathBuf, said: String) -> LeftOut {
        LeftOut { source, path, said }
    }

    pub fn source(&self) -> PieceSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The sentence a person reads, naming the key or the id at fault.
    pub fn said(&self) -> &str {
        &self.said
    }
}

/// The paths of files that share one id in one place, named in a sentence.
pub(crate) fn named(paths: &[&Path]) -> String {
    paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(" and ")
}

/// Whether a person's own file hides a repository's that says something else. **Two copies that
/// are the same hide nothing**, so they carry no mark.
pub(crate) fn hides<T: PartialEq>(by: PieceSource, running: &T, under: PieceSource, replaced: &T) -> bool {
    by == PieceSource::Kit && under == PieceSource::Repository && running != replaced
}
