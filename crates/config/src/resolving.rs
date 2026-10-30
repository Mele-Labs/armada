//! Which piece of a kind applies to one repository, and which of two with one id wins. **One
//! reading for every kind of piece that has a scope** (a lifecycle, a rule), so the precedence the
//! design states is held in one place and cannot drift between them.
//!
//! **Where two of the same id apply, the narrower scope wins, then yours over the repository's**
//! (the design "Workspaces, lifecycles and rules", *Where they are kept*). One scoped to the
//! repository beats one scoped to a Workspace that holds it, which beats one that applies
//! everywhere, whoever kept it; only at the same scope does the place decide, and Kit's is last in
//! [`PieceSource`] so it wins. The one that loses is **still listed, marked as overridden**.
//!
//! **One that applies nowhere here is not listed at all**: scoped to a Workspace that does not hold
//! this repository, it is another repository's. A file that will not read belongs to none, so it
//! is left out for every repository and says why.
//!
//! **A Workspace committed to one repository is kept in that repository, its home, and the others
//! read it from there.** A member is handed the home's files ([`Written::in_home_of`]) and takes
//! only those scoped to that Workspace. The home's files scoped to itself, or to another
//! Workspace, stay the home's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{LifecycleScope, ManifestId, PieceSource, ScopeId, Workspace};

use crate::piece::{hides, named, LeftOut, Written};
use crate::workspace::Workspaces;

/// How narrowly a piece's scope names the repository it is asked about.
///
/// **Declared in the order they lose**, as a place is, so the narrowest compares greatest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reach {
    /// Every repository, where the pieces Armada carries apply.
    Everywhere,
    /// A Workspace that holds the repository.
    Workspace,
    /// The repository itself.
    Repository,
}

/// What decides which of two pieces with one id runs: the scope first, then the place, then
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

/// A kind of piece that is read off files, scoped, and has an id.
pub(crate) trait Scoped: Clone + PartialEq {
    type Id: Ord + Clone;
    fn id(&self) -> &Self::Id;
    fn id_text(&self) -> &str;
    fn scope(&self) -> &LifecycleScope;
}

/// One piece that applies, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved<T> {
    pub piece: T,
    pub rank: Rank,
    pub path: Option<PathBuf>,
    pub overridden_by: Option<Rank>,
    pub hides: Option<PieceSource>,
}

/// What applies to `repository`: `built` (what Armada carries) and every file in `written` whose
/// scope names it, merged by id, and the files set aside. `read` is the kind's own reading of one
/// file's text, answering in a sentence what is wrong with a file that is not one.
///
/// Each id comes out as the one that runs first and then each one it replaced.
pub(crate) fn resolve<T: Scoped>(
    repository: &ManifestId,
    workspaces: &Workspaces,
    built: Vec<T>,
    written: impl IntoIterator<Item = Written>,
    read: impl Fn(&Path, &str) -> Result<T, String>,
) -> (Vec<Resolved<T>>, Vec<LeftOut>) {
    let own = ScopeId::new(repository.as_str()).ok();
    let held: Vec<&ScopeId> = workspaces.holding(repository).into_iter().map(Workspace::id).collect();
    let reach_of = |written: &LifecycleScope| match written {
        LifecycleScope::Everywhere => Some(Reach::Everywhere),
        LifecycleScope::Within(scope) if Some(scope) == own.as_ref() => Some(Reach::Repository),
        LifecycleScope::Within(scope) if held.contains(&scope) => Some(Reach::Workspace),
        LifecycleScope::Within(_) => None,
    };

    let mut left_out = Vec::new();
    let mut placed: BTreeMap<(T::Id, Rank), Vec<(T, Option<PathBuf>)>> = BTreeMap::new();
    for one in built {
        let rank = Rank { reach: Reach::Everywhere, source: PieceSource::Armada, home: false };
        placed.entry((one.id().clone(), rank)).or_default().push((one, None));
    }
    for file in written {
        match read(file.path(), file.text()) {
            Ok(piece) => {
                // A file from a Workspace's home is the member's only where it is scoped to it.
                let theirs = |home: &ScopeId| piece.scope() == &LifecycleScope::Within(home.clone());
                if file.home_of().is_some_and(|home| !theirs(home)) {
                    continue;
                }
                if let Some(reach) = reach_of(piece.scope()) {
                    let rank = Rank { reach, source: file.source(), home: file.home_of().is_some() };
                    placed.entry((piece.id().clone(), rank)).or_default().push((piece, Some(file.path().to_path_buf())));
                }
            }
            Err(said) => left_out.push(LeftOut::new(file.source(), file.path().to_path_buf(), said)),
        }
    }

    let mut by_id: BTreeMap<T::Id, Vec<(T, Rank, Option<PathBuf>)>> = BTreeMap::new();
    for ((id, rank), mut defs) in placed {
        if defs.len() > 1 {
            let paths: Vec<&Path> = defs.iter().filter_map(|(_, path)| path.as_deref()).collect();
            let said = format!(
                "`{}` is written more than once for this repository in one place, in {}",
                defs[0].0.id_text(),
                named(&paths)
            );
            if let Some(first) = defs.swap_remove(0).1 {
                left_out.push(LeftOut::new(rank.source, first, said));
            }
            continue;
        }
        let (piece, path) = defs.remove(0);
        by_id.entry(id).or_default().push((piece, rank, path));
    }

    let mut resolved = Vec::new();
    for (_, mut candidates) in by_id {
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let mut candidates = candidates.into_iter();
        let Some((running, by, path)) = candidates.next() else { continue };
        let mut hid = None;
        let mut beaten = Vec::new();
        for (piece, rank, path) in candidates {
            if hides(by.source, &running, rank.source, &piece) {
                hid = Some(rank.source);
            }
            beaten.push(Resolved { piece, rank, path, overridden_by: Some(by), hides: None });
        }
        resolved.push(Resolved { piece: running, rank: by, path, overridden_by: None, hides: hid });
        resolved.extend(beaten);
    }
    (resolved, left_out)
}
