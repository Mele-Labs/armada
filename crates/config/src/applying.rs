//! Which lifecycles apply to one repository, from Armada's, Kit's and the repository's own.
//!
//! **Where two of the same id apply, the narrower scope wins, then yours over the repository's**
//! (the design "Workspaces, lifecycles and rules", *Where they are kept*). A lifecycle scoped to
//! the repository beats one scoped to a Workspace that holds it, which beats one that applies
//! everywhere, whoever kept it; only at the same scope does the place decide, and Kit's is last in
//! [`PieceSource`] so it wins. The one that loses is **still listed, marked as overridden**, so a
//! person is not left wondering where a file they wrote went.
//!
//! **A lifecycle that applies nowhere here is not listed at all**: one scoped to a Workspace that
//! does not hold this repository is another repository's. A file that will not parse belongs to
//! none, so it is left out for every repository and says why.
//!
//! **A Workspace committed to one repository is kept in that repository, its home, and the others
//! read it from there.** That covers what is scoped to the Workspace as well as the Workspace's
//! file: a member is handed the home's lifecycle files ([`Written::in_home_of`]) and takes only
//! those scoped to that Workspace. The home's files scoped to itself, or to another Workspace,
//! stay the home's.
//!
//! The Workspaces that hold the repository are read first ([`Workspaces`]); a scope that is the
//! repository's own id is the repository's, and any other must be a Workspace that holds it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{Lifecycle, LifecycleId, LifecycleScope, ManifestId, PieceSource, ScopeId, Workspace};

use crate::lifecycle::parse;
use crate::piece::{hides, named, LeftOut, Written};
use crate::workspace::Workspaces;

/// How narrowly a lifecycle's scope names the repository it is asked about.
///
/// **Declared in the order they lose**, as a place is, so the narrowest compares greatest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reach {
    /// Every repository, where the lifecycles Armada carries apply.
    Everywhere,
    /// A Workspace that holds the repository.
    Workspace,
    /// The repository itself.
    Repository,
}

/// What decides which of two lifecycles with one id runs: the scope first, then the place, then
/// whether it came from the home of the Workspace it is scoped to.
///
/// **Among committed copies at one scope, the home repository's is the one a member sees**, so
/// `home` is last and a copy a member kept itself loses to it. Kit's still wins over both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rank {
    pub reach: Reach,
    pub source: PieceSource,
    pub home: bool,
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
        let own = ScopeId::new(repository.as_str()).ok();
        let held: Vec<&ScopeId> = workspaces.holding(repository).into_iter().map(Workspace::id).collect();
        let reach_of = |written: &LifecycleScope| match written {
            LifecycleScope::Everywhere => Some(Reach::Everywhere),
            LifecycleScope::Within(scope) if Some(scope) == own.as_ref() => Some(Reach::Repository),
            LifecycleScope::Within(scope) if held.contains(&scope) => Some(Reach::Workspace),
            LifecycleScope::Within(_) => None,
        };

        let mut left_out = Vec::new();
        let mut placed: BTreeMap<(LifecycleId, Rank), Vec<(Lifecycle, Option<PathBuf>)>> = BTreeMap::new();
        for built in Lifecycle::built_in() {
            let rank = Rank { reach: Reach::Everywhere, source: PieceSource::Armada, home: false };
            placed.entry((built.id().clone(), rank)).or_default().push((built, None));
        }
        for one in written {
            match parse(one.path(), one.text()) {
                Ok(lifecycle) => {
                    // A file from a Workspace's home is the member's only where it is scoped to it.
                    let theirs = |home: &ScopeId| lifecycle.scope() == &LifecycleScope::Within(home.clone());
                    if one.home_of().is_some_and(|home| !theirs(home)) {
                        continue;
                    }
                    if let Some(reach) = reach_of(lifecycle.scope()) {
                        let rank = Rank { reach, source: one.source(), home: one.home_of().is_some() };
                        placed
                            .entry((lifecycle.id().clone(), rank))
                            .or_default()
                            .push((lifecycle, Some(one.path().to_path_buf())));
                    }
                }
                Err(why) => left_out.push(LeftOut::new(one.source(), one.path().to_path_buf(), why.to_string())),
            }
        }

        let mut held: BTreeMap<LifecycleId, Vec<(Lifecycle, Rank, Option<PathBuf>)>> = BTreeMap::new();
        for ((id, rank), mut defs) in placed {
            if defs.len() > 1 {
                let paths: Vec<&Path> = defs.iter().filter_map(|(_, path)| path.as_deref()).collect();
                let said = format!("`{}` is written more than once for this repository in one place, in {}", id.as_str(), named(&paths));
                if let Some(first) = defs.swap_remove(0).1 {
                    left_out.push(LeftOut::new(rank.source, first, said));
                }
                continue;
            }
            let (lifecycle, path) = defs.remove(0);
            held.entry(id).or_default().push((lifecycle, rank, path));
        }

        let mut files = Vec::new();
        for (_, mut candidates) in held {
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let mut candidates = candidates.into_iter();
            let Some((running, by, path)) = candidates.next() else { continue };
            let mut hid = None;
            let mut beaten = Vec::new();
            for (lifecycle, rank, path) in candidates {
                if hides(by.source, &running, rank.source, &lifecycle) {
                    hid = Some(rank.source);
                }
                beaten.push(LifecycleFile { lifecycle, rank, path, overridden_by: Some(by), hides: None });
            }
            files.push(LifecycleFile { lifecycle: running, rank: by, path, overridden_by: None, hides: hid });
            files.extend(beaten);
        }
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
