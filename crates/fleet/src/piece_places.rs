//! The lifecycles, rules and autonomy defaults a repository has, read from the places a piece is
//! kept. A Project scopes them, and is held as `crate::project_files` reads it.
//!
//! **Read when asked, never cached**, as the review tools are: a piece is a small file a person
//! edits, and a Job started afterwards reads it again, so what was saved is what the next Job
//! freezes. A Job already running keeps the copy it started with and is not touched here.
//!
//! The folders are Kit's on this Mac, `lifecycles`, `rules` and `autonomy`, and the same names under
//! each repository's `.armada/`. **A repository that belongs to a Project also reads what its fellow
//! members hold**: it is handed the files of each member's `.armada/<kind>/`, and `config` keeps only
//! those scoped to that Project. What merges, and which of two with one id wins, is `config`'s.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{Autonomies, Lifecycles, PieceWritten, Projects, Rules};
use core_model::{Lifecycle, ManifestId};
use store::{KeptCopy, KeptPiece, PieceKind};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// The folder lifecycles are kept in, under Kit's home and under a repository's `.armada/`.
pub(crate) const LIFECYCLES: &str = "lifecycles";
/// The folder lifecycle rules are kept in, in the same two places.
pub(crate) const RULES: &str = "rules";
/// The folder autonomy defaults are kept in, in the same two places.
pub(crate) const AUTONOMY: &str = "autonomy";
/// What a piece file may be written as, the same set a workflow's is.
const EXTENSIONS: &[&str] = &["json", "yml", "yaml"];

/// Every piece file in `folder`, by name. A folder that is not there holds none, and a file that
/// will not read is skipped: a person cannot be told what is not readable except as a file that
/// did not fit, and that sentence needs text.
pub(crate) fn files_in(folder: &Path, place: impl Fn(PathBuf, String) -> PieceWritten) -> Vec<PieceWritten> {
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
    pub(crate) fn kit_pieces(&self, kind: &str) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(kind)
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
        let projects = self.projects_held();
        let written = self.pieces_written(&served, &projects, LIFECYCLES);
        Ok(Lifecycles::applying_to(served.manifest().id(), &projects, self.built_in_lifecycles(Some(repository.as_str())), written))
    }

    /// The rules that apply to one repository: Kit's and the repository's own, narrowest scope
    /// first, and those a fellow member's `.armada/` scopes to its Project. **Armada carries none**, so with no file
    /// nothing applies and no Job is handed a rule. A rule naming a lifecycle or a state the
    /// repository's lifecycles lack is set aside with the sentence. Nothing is frozen here.
    pub fn rules_applying_to(&self, repository: &ManifestId) -> Result<Rules, Refusal> {
        let lifecycles = self.lifecycles_applying_to(repository)?;
        let served = self.repositories().serving(repository.as_str()).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: repository.as_str().to_string(),
                held: self.held_manifests(),
            })
        })?;
        let projects = self.projects_held();
        let written = self.pieces_written(&served, &projects, RULES);
        let running: Vec<&Lifecycle> = lifecycles.running().collect();
        Ok(Rules::applying_to(served.manifest().id(), &projects, &running, written))
    }

    /// The autonomy defaults that apply to one repository: Kit's and the repository's own, and those
    /// a fellow member's `.armada/` scopes to its Project, the narrower laid over the wider.
    pub fn autonomy_applying_to(&self, repository: &ManifestId) -> Result<Autonomies, Refusal> {
        let served = self.repositories().serving(repository.as_str()).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: repository.as_str().to_string(),
                held: self.held_manifests(),
            })
        })?;
        let projects = self.projects_held();
        let written = self.pieces_written(&served, &projects, AUTONOMY);
        Ok(Autonomies::applying_to(served.manifest().id(), &projects, written))
    }

    /// What a Job in `repository` freezes when it starts: the text of each lifecycle file and each
    /// rule that applies, and the autonomy defaults laid over each other. **The text, and not the
    /// parsed value**, so what a Job keeps is what was written then. The four lifecycles Armada
    /// carries are not files and are not kept: they are the same for every Job.
    pub(crate) fn job_copy_for(&self, repository: &str, project: Option<&str>) -> Result<KeptCopy, Refusal> {
        let served = self.repositories().serving(repository).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: repository.to_string(),
                held: self.held_manifests(),
            })
        })?;
        let projects = self.projects_held();
        let id = served.manifest().id();
        let text_of = |written: &[PieceWritten], path: &Path| {
            written.iter().find(|one| one.path() == path).map(|one| one.text().to_string()).unwrap_or_default()
        };
        let mut pieces = Vec::new();

        let written = self.pieces_written(&served, &projects, LIFECYCLES);
        let lifecycles = Lifecycles::applying_to(id, &projects, self.built_in_lifecycles_in(Some(repository), project), written.clone());
        for file in lifecycles.files().iter().filter(|file| file.overridden_by().is_none()) {
            let Some(path) = file.path() else { continue };
            pieces.push(KeptPiece {
                kind: PieceKind::Lifecycle,
                piece_id: file.lifecycle().id().as_str().to_string(),
                path: path.display().to_string(),
                text: text_of(&written, path),
            });
        }

        let written = self.pieces_written(&served, &projects, RULES);
        let running: Vec<&Lifecycle> = lifecycles.running().collect();
        let rules = Rules::applying_to(id, &projects, &running, written.clone());
        for file in rules.files().iter().filter(|file| file.overridden_by().is_none()) {
            pieces.push(KeptPiece {
                kind: PieceKind::Rule,
                piece_id: file.rule().id().as_str().to_string(),
                path: file.path().display().to_string(),
                text: text_of(&written, file.path()),
            });
        }

        let written = self.pieces_written(&served, &projects, AUTONOMY);
        let autonomy = Autonomies::applying_to(id, &projects, written)
            .defaults()
            .iter()
            .map(|(kind, autonomy)| (kind.as_wire().to_string(), autonomy.as_wire().to_string()))
            .collect();
        Ok(KeptCopy { pieces, autonomy })
    }

    /// The files of one kind of piece `served` reads: Kit's, its own, and those of each member
    /// repository of the Projects that hold it. **A member's own folder is read once** however many
    /// Projects it shares with `served`, handed to each as that Project's.
    fn pieces_written(&self, served: &Served, projects: &Projects, kind: &str) -> Vec<PieceWritten> {
        let mut written = files_in(&self.kit_pieces(kind), PieceWritten::in_kit);
        written.extend(files_in(&in_repository(served.root(), kind), PieceWritten::in_repository));
        let hosted = self.repositories().hosted();
        for project in projects.holding(served.manifest().id()) {
            for member in project.repositories() {
                let Some(fellow) = hosted.iter().find(|one| one.manifest().id() == member) else { continue };
                // A repository's own folder is read above, and is no less its own for being a member.
                if fellow.root() == served.root() {
                    continue;
                }
                let folder = in_repository(fellow.root(), kind);
                written.extend(files_in(&folder, |path, text| PieceWritten::in_home_of(project.id().clone(), path, text)));
            }
        }
        written
    }
}
