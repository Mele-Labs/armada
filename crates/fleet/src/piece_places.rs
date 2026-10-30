//! The lifecycles and Workspaces a repository has, read from the places a piece is kept. A
//! Workspace here is a set of repositories, and not the Manifest's monorepo unit (`workspaces`).
//!
//! **Read when asked, never cached**, as the review tools are: a piece is a small file a person
//! edits, and a Job started afterwards reads it again, so what was saved is what the next Job
//! freezes. A Job already running keeps the copy it started with and is not touched here.
//!
//! The two folders are Kit's on this Mac, `lifecycles` and `workspaces`, and the same names under
//! each repository's `.armada/`. **The files of every repository Fleet serves are read for the
//! Workspaces**, because one that spans several is committed to a single repository, its home, and
//! the others read it from there. They read from there what is scoped to it too: a repository that
//! belongs to a Workspace is also handed the lifecycle files its home holds, and `config` keeps
//! only those scoped to that Workspace. What merges, and which of two with one id wins, is `config`'s.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{Lifecycles, PieceWritten, Workspaces};
use core_model::{ManifestId, ScopeId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The folder lifecycles are kept in, under Kit's home and under a repository's `.armada/`.
pub(crate) const LIFECYCLES: &str = "lifecycles";
/// The folder Workspaces are kept in, in the same two places.
pub(crate) const WORKSPACES: &str = "workspaces";
/// What a piece file may be written as, the same set a workflow's is.
const EXTENSIONS: &[&str] = &["json", "yml", "yaml"];

/// Every piece file in `folder`, by name. A folder that is not there holds none, and a file that
/// will not read is skipped: a person cannot be told what is not readable except as a file that
/// did not fit, and that sentence needs text.
fn files_in(folder: &Path, place: impl Fn(PathBuf, String) -> PieceWritten) -> Vec<PieceWritten> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| EXTENSIONS.contains(&ext)))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|text| place(path, text)))
        .collect()
}

fn in_repository(root: &str, kind: &str) -> PathBuf {
    Path::new(root).join(".armada").join(kind)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Where Kit keeps a kind of piece on this Mac.
    fn kit_pieces(&self, kind: &str) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(kind)
    }

    /// Every Workspace this Fleet can see: Kit's, and each served repository's own committed.
    pub fn workspaces_read(&self) -> Workspaces {
        let mut written = files_in(&self.kit_pieces(WORKSPACES), PieceWritten::in_kit);
        for one in self.repositories().hosted() {
            written.extend(files_in(&in_repository(one.root(), WORKSPACES), PieceWritten::in_repository));
        }
        Workspaces::of(written)
    }

    /// The lifecycles that apply to one repository: the four Armada carries, Kit's and the
    /// repository's own, narrowest scope first. **Nothing is frozen here**; a Job copies what this
    /// answers when it starts.
    pub fn lifecycles_applying_to(&self, repository: &ManifestId) -> Result<Lifecycles, Refusal> {
        let served = self.repositories().serving(repository.as_str()).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: repository.as_str().to_string(),
                held: self.held_manifests(),
            })
        })?;
        let workspaces = self.workspaces_read();
        let mut written = files_in(&self.kit_pieces(LIFECYCLES), PieceWritten::in_kit);
        written.extend(files_in(&in_repository(served.root(), LIFECYCLES), PieceWritten::in_repository));
        for workspace in workspaces.holding(served.manifest().id()) {
            let Some(home) = self.home_of(&workspaces, workspace.id()) else { continue };
            // A repository's own folder is read above, and is no less its own for being the home.
            if home != served.root() {
                let folder = in_repository(&home, LIFECYCLES);
                written.extend(files_in(&folder, |path, text| PieceWritten::in_home_of(workspace.id().clone(), path, text)));
            }
        }
        Ok(Lifecycles::applying_to(served.manifest().id(), &workspaces, written))
    }

    /// The root of the repository a Workspace is committed to, where it is: the served one whose
    /// `.armada/workspaces/` holds the file that runs.
    fn home_of(&self, workspaces: &Workspaces, id: &ScopeId) -> Option<String> {
        let kept = workspaces.kept_at(id)?.parent()?;
        self.repositories()
            .hosted()
            .into_iter()
            .find(|one| in_repository(one.root(), WORKSPACES) == kept)
            .map(|one| one.root().to_string())
    }
}
