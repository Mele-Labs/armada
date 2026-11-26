//! The judge types a person has, and writing and deleting one. `docs/concepts/judge.md`, *Judge types*.
//!
//! **Read when asked, never cached**: a type is a small file a person edits, and an ask reads it again, so
//! what was saved is what the next press asks. The merge and what a file must hold are `config::judge_types`';
//! this is the folders, the write and the delete.
//!
//! **Kit's folders only are written here.** A repository's own types arrive by Commit to repository, a Job
//! and a pull request, as every piece a repository keeps does, so a save into the repository's scope is
//! refused and says so.

use std::path::{Component, Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::judge_types::{self as types, Def, Place, Written, ADVERSARIAL};
use ipc::{
    DeleteJudgeType, JudgeType, JudgeTypes, ManifestId, ReviewToolLeftOut, ReviewToolScope, ReviewToolSource, SaveJudgeType,
    WireError, WireValue,
};

use crate::daemon::Fleet;
use crate::review_tools::files_in;

/// The folder under Kit's home, and under a repository's `.armada/`.
pub(crate) const FOLDER: &str = "judges";
/// Inside Kit's folder, the one a project's own types are in, as workflows keep theirs.
const PROJECTS: &str = "projects";

/// A blank name or prompt. A 422.
const BLANK: &str = "fleet.judge_type_blank";
/// A name that has no words in it, so no id. A 422.
const NOT_A_NAME: &str = "fleet.judge_type_not_a_name";
/// The id is already defined in that place and `overwrite` was not asked for. A 422.
const EXISTS: &str = "fleet.judge_type_exists";
/// A project that is no one folder's name, or one named with the repository's scope, or a save or delete
/// aimed at the repository's own folder. A 422.
const MISPLACED: &str = "fleet.judge_type_misplaced";
/// The place holds no type of that id to delete. A 422.
const NOT_KEPT: &str = "fleet.judge_type_not_kept";
/// An ask names a type that is not defined for the repository and project. A 422.
const UNKNOWN: &str = "fleet.judge_type_unknown";
/// The file could not be written or removed. A 500.
const UNWRITABLE: &str = "fleet.judge_type_unwritable";

fn source_of(place: Place) -> ReviewToolSource {
    match place {
        Place::Armada => ReviewToolSource::Armada,
        Place::Repository => ReviewToolSource::Repository,
        Place::Kit | Place::KitProject => ReviewToolSource::Kit,
    }
}

/// A project id that names one folder, and no path.
fn one_folder(project: &str) -> bool {
    let mut parts = Path::new(project).components();
    matches!((parts.next(), parts.next()), (Some(Component::Normal(part)), None) if part == project)
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
    fn judge_type_refused(&self, code: &'static str, said: String) -> WireError {
        WireError::raised(code, said, self.run_id())
    }

    /// Kit's folder for every project, or for `project`.
    pub(crate) fn judge_kit_folder(&self, project: Option<&str>) -> Result<PathBuf, Refusal> {
        let all = PathBuf::from(&self.host().kit_home).join(FOLDER);
        match project {
            None => Ok(all),
            Some(project) if one_folder(project) => Ok(all.join(PROJECTS).join(project)),
            Some(project) => Err(Refusal::Unacceptable(
                self.judge_type_refused(MISPLACED, format!("`{project}` is not a project id, so it names no folder of Kit's"))
                    .with_field("project", WireValue::Str(project.to_string())),
            )),
        }
    }

    /// The four places merged for `project`, the one the person is in: Armada's, then the repository's, then
    /// Kit's, then Kit's for the project.
    pub(crate) fn judge_types_resolved(&self, root: Option<&str>, project: Option<&str>) -> types::Resolved {
        let mut all = types::carried();
        if let Some(root) = root {
            for (path, text) in files_in(&Path::new(root).join(".armada").join(FOLDER)) {
                all.push(Written::in_repository(path, text));
            }
        }
        if let Ok(folder) = self.judge_kit_folder(None) {
            for (path, text) in files_in(&folder) {
                all.push(Written::in_kit(path, text));
            }
        }
        if let Some(folder) = project.and_then(|project| self.judge_kit_folder(Some(project)).ok()) {
            for (path, text) in files_in(&folder) {
                all.push(Written::in_kit_project(path, text));
            }
        }
        types::resolve(all)
    }

    fn judge_types_listed(&self, root: Option<&str>, project: Option<&str>) -> JudgeTypes {
        let resolved = self.judge_types_resolved(root, project);
        let project_of = |place: Place| (place == Place::KitProject).then(|| project.map(str::to_string)).flatten();
        JudgeTypes {
            types: resolved
                .defs
                .iter()
                .map(|def| JudgeType {
                    id: def.id.clone(),
                    name: def.name.clone(),
                    prompt: def.prompt.clone(),
                    source: source_of(def.place),
                    project: project_of(def.place),
                    replaces: def.replaces.map(source_of),
                    hides: def.hides.map(source_of),
                })
                .collect(),
            left_out: resolved
                .left_out
                .into_iter()
                .map(|one| ReviewToolLeftOut { file: one.path.display().to_string(), source: source_of(one.place), said: one.said })
                .collect(),
        }
    }

    /// `list_judge_types`.
    pub(crate) async fn judge_types_read(&self, manifest_id: Option<ManifestId>, project: Option<String>) -> Result<JudgeTypes, Refusal> {
        let root = match &manifest_id {
            Some(named) => Some(self.served_named(Some(named))?.root().to_string()),
            None => None,
        };
        let project = project.map(|project| project.trim().to_string()).filter(|project| !project.is_empty());
        if let Some(project) = &project {
            self.judge_kit_folder(Some(project))?;
        }
        Ok(self.judge_types_listed(root.as_deref(), project.as_deref()))
    }

    /// The type an ask reads as: `judge` by id, or Adversarial where there is none, resolved for the
    /// repository at `root` and the Job's `project`. An id nobody defines is refused, naming it.
    pub(crate) fn judge_type_chosen(&self, root: Option<&str>, project: Option<&str>, judge: Option<&str>) -> Result<Def, Refusal> {
        let wanted = judge.map(str::trim).filter(|id| !id.is_empty()).unwrap_or(ADVERSARIAL);
        let resolved = self.judge_types_resolved(root, project);
        resolved.get(wanted).cloned().ok_or_else(|| {
            let known: Vec<&str> = resolved.defs.iter().map(|one| one.id.as_str()).collect();
            Refusal::Unacceptable(
                self.judge_type_refused(UNKNOWN, format!("no judge type `{wanted}` is defined here; the types are {}", known.join(", ")))
                    .with_field("judge", WireValue::Str(wanted.to_string())),
            )
        })
    }

    /// The Kit folder a save or delete aims at, once the scope and project are ones this Fleet writes.
    async fn judge_type_folder_written(
        &self,
        manifest_id: &ManifestId,
        scope: ReviewToolScope,
        project: Option<&str>,
    ) -> Result<(String, PathBuf), Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        if scope == ReviewToolScope::Repository {
            return Err(Refusal::Unacceptable(self.judge_type_refused(
                MISPLACED,
                String::from("a repository's judge types are committed to it as a pull request, never written into its checkout"),
            )));
        }
        if let Some(project) = project {
            self.judge_kit_folder(Some(project))?;
            self.workflow_scope_held(&ipc::WorkflowScope::Project { id: project.to_string() }, &served)?;
        }
        let folder = self.judge_kit_folder(project)?;
        Ok((served.root().to_string(), folder))
    }

    /// `save_judge_type`: write one type in Kit's folder, then answer what the places come to.
    pub(crate) async fn judge_type_saved(&self, manifest_id: ManifestId, save: SaveJudgeType) -> Result<JudgeTypes, Refusal> {
        let project = save.project.as_deref().map(str::trim).filter(|project| !project.is_empty());
        let (root, folder) = self.judge_type_folder_written(&manifest_id, save.scope, project).await?;
        let blank = |said: &str| Refusal::Unacceptable(self.judge_type_refused(BLANK, said.to_string()));
        if save.name.trim().is_empty() {
            return Err(blank("a judge type needs a name"));
        }
        if save.prompt.trim().is_empty() {
            return Err(blank("a judge type needs a prompt"));
        }
        let id = types::fit(&save.name, &save.prompt)
            .map_err(|said| Refusal::Unacceptable(self.judge_type_refused(NOT_A_NAME, said)))?;
        // The file that holds this id in this place is the one replaced, whatever it is called.
        let held = judge_type_held(&folder, &id);
        if let (Some(path), false) = (&held, save.overwrite) {
            return Err(Refusal::Unacceptable(self.judge_type_refused(EXISTS, format!("{} already defines `{id}`", path.display()))));
        }
        let path = held.unwrap_or_else(|| folder.join(format!("{id}.md")));
        let unwritable = |why: std::io::Error| {
            Refusal::Fault(self.judge_type_refused(UNWRITABLE, format!("{} could not be written: {why}", path.display())))
        };
        std::fs::create_dir_all(&folder).map_err(unwritable)?;
        let staged = path.with_extension("md.writing");
        std::fs::write(&staged, types::file_of(&save.name, &save.prompt))
            .and_then(|()| std::fs::rename(&staged, &path))
            .map_err(unwritable)?;
        Ok(self.judge_types_listed(Some(&root), project))
    }

    /// `delete_judge_type`: remove the file that defines `id` in Kit's folder, then answer what the places come
    /// to. **A Kit copy of an Armada id going is "Reset to Armada's"**: Armada's prompt answers for it again.
    pub(crate) async fn judge_type_deleted(&self, manifest_id: ManifestId, delete: DeleteJudgeType) -> Result<JudgeTypes, Refusal> {
        let project = delete.project.as_deref().map(str::trim).filter(|project| !project.is_empty());
        let (root, folder) = self.judge_type_folder_written(&manifest_id, delete.scope, project).await?;
        let id = delete.id.trim();
        let Some(path) = judge_type_held(&folder, id) else {
            return Err(Refusal::Unacceptable(self.judge_type_refused(NOT_KEPT, format!("{} holds no judge type `{id}`", folder.display()))));
        };
        std::fs::remove_file(&path).map_err(|why| {
            Refusal::Fault(self.judge_type_refused(UNWRITABLE, format!("{} could not be removed: {why}", path.display())))
        })?;
        Ok(self.judge_types_listed(Some(&root), project))
    }
}

/// The file in `folder` that defines `id`: one that fits and makes that id, else one named for it.
pub(crate) fn judge_type_held(folder: &Path, id: &str) -> Option<PathBuf> {
    files_in(folder)
        .into_iter()
        .find(|(path, text)| types::resolve([Written::in_kit(path.clone(), text.clone())]).get(id).is_some())
        .map(|(path, _)| path)
        .or_else(|| Some(folder.join(format!("{id}.md"))).filter(|path| path.exists()))
}

/// The text of the file in `folder` that defines `id`, and where it is, for a commit to copy.
pub(crate) fn judge_type_defined(folder: &Path, id: &str) -> Option<(PathBuf, String)> {
    let path = judge_type_held(folder, id)?;
    let text = std::fs::read_to_string(&path).ok()?;
    types::resolve([Written::in_kit(path.clone(), text.clone())]).get(id).map(|_| (path, text))
}
