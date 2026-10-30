//! Which lifecycles apply to one repository, from Armada's, Kit's and the repository's own.
//!
//! The precedence (the narrower scope, then yours over the repository's, and the one that loses
//! still listed) is [`crate::resolving`]'s, held there once for every kind of piece that has a
//! scope. What is a lifecycle's own is that Armada carries four, and how a file is read.

use std::path::{Path, PathBuf};

use core_model::{Lifecycle, LifecycleId, LifecycleScope, ManifestId, PieceSource};

use crate::lifecycle::parse;
use crate::piece::{LeftOut, Written};
use crate::resolving::{resolve, Resolved, Scoped};
use crate::workspace::Workspaces;

pub use crate::resolving::{Rank, Reach};

impl Scoped for Lifecycle {
    type Id = LifecycleId;

    fn id(&self) -> &LifecycleId {
        Lifecycle::id(self)
    }

    fn id_text(&self) -> &str {
        Lifecycle::id(self).as_str()
    }

    fn scope(&self) -> &LifecycleScope {
        Lifecycle::scope(self)
    }
}

/// One lifecycle that applies, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleFile {
    lifecycle: Lifecycle,
    rank: Rank,
    path: Option<PathBuf>,
    overridden_by: Option<Rank>,
    hides: Option<PieceSource>,
}

impl LifecycleFile {
    pub fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    pub fn rank(&self) -> Rank {
        self.rank
    }

    /// Where it was read. **Absent for what Armada carries**, which is built and not read.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The rank of the one that runs instead of this. Absent on the one that runs.
    pub fn overridden_by(&self) -> Option<Rank> {
        self.overridden_by
    }

    /// The place whose file this one replaced because it says something else.
    pub fn hides(&self) -> Option<PieceSource> {
        self.hides
    }
}

impl From<Resolved<Lifecycle>> for LifecycleFile {
    fn from(one: Resolved<Lifecycle>) -> LifecycleFile {
        LifecycleFile { lifecycle: one.piece, rank: one.rank, path: one.path, overridden_by: one.overridden_by, hides: one.hides }
    }
}

/// The lifecycles that apply to one repository, and the files set aside.
#[derive(Debug, Default)]
pub struct Lifecycles {
    files: Vec<LifecycleFile>,
    left_out: Vec<LeftOut>,
}

impl Lifecycles {
    /// What applies to `repository`: the four Armada carries and every file in `written` whose
    /// scope names it, merged by id. `workspaces` says which Workspaces hold the repository.
    pub fn applying_to(
        repository: &ManifestId,
        workspaces: &Workspaces,
        written: impl IntoIterator<Item = Written>,
    ) -> Lifecycles {
        let (resolved, left_out) = resolve(
            repository,
            workspaces,
            Lifecycle::built_in(),
            written,
            |path, text| parse(path, text).map_err(|why| why.to_string()),
        );
        let files = resolved.into_iter().map(LifecycleFile::from).collect();
        Lifecycles { files, left_out }
    }

    /// Every lifecycle that applies: the one that runs for each id and each one it replaced.
    pub fn files(&self) -> &[LifecycleFile] {
        &self.files
    }

    /// The files set aside, and why.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// One lifecycle for each id: the one that runs.
    pub fn running(&self) -> impl Iterator<Item = &Lifecycle> {
        self.files.iter().filter(|file| file.overridden_by.is_none()).map(|file| &file.lifecycle)
    }

    /// The lifecycle of this id that runs, if one applies.
    pub fn get(&self, id: &LifecycleId) -> Option<&Lifecycle> {
        self.running().find(|lifecycle| lifecycle.id() == id)
    }
}
