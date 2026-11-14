//! What Fleet does to bring Workspaces and task-list projects to the files a project is, at every
//! start. **Idempotent**: each step looks at what is there and does nothing that is done.
//!
//! 1. Each Kit `workspaces/<id>.yml` becomes `projects/<id>.yml`, whole repositories as members,
//!    with a row at the end of his order, and the old Kit file is removed.
//! 2. Each served repository's committed `.armada/workspaces/*.yml` becomes the same, kept on this
//!    Mac, unless a project of that id is there. **The repository's file is never edited**, and each
//!    import is recorded by the path it was read from, so a project he deletes is not brought back.
//! 3. Each row of his order that has no file becomes a file on this Mac whose members are the
//!    repositories its rows name, in the order they first appear. A subtask takes its parent's.
//!
//! Loose tasks, in no project, are not touched. A file that will not read is left where it is.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::{parse_workspace_v1, project_text, PieceWritten};
use core_model::{ManifestId, Member, Project, ScopeId, Title, Ulid};
use store::{KeptProject, KeptRowItem};

use super::{file_safe, place_of_file};
use crate::daemon::Fleet;
use crate::piece_places::files_in;
use crate::run_memory::{place_of, places};

/// The folder a Workspace was kept in before Projects, under Kit's and under a repository's `.armada/`.
const WORKSPACES: &str = "workspaces";

/// What a start's migration did, and what it left alone and why.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Migrated {
    /// Kit workspaces now projects.
    pub converted: usize,
    /// Committed workspaces now projects.
    pub imported: usize,
    /// Rows of his order that were given a file.
    pub derived: usize,
    /// A file or a row that was not migrated, with the sentence that says why.
    pub left: Vec<String>,
}

/// The members a task-list project's rows name: each repository once, in the order first seen.
fn members_of(project: &KeptProject) -> Vec<Member> {
    let own: BTreeMap<&str, &str> = project.rows.iter().map(|row| (row.id.as_str(), row.manifest_id.as_str())).collect();
    let mut seen: Vec<&str> = Vec::new();
    for row in &project.rows {
        let named = match &row.item {
            KeptRowItem::Task { parent: Some(parent), .. } => own.get(parent.as_str()).copied().unwrap_or(row.manifest_id.as_str()),
            _ => row.manifest_id.as_str(),
        };
        if !named.is_empty() && !seen.contains(&named) {
            seen.push(named);
        }
    }
    seen.into_iter().map(|one| Member::whole(ManifestId::carried(Ulid::carried(one)))).collect()
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
    /// The migration, then every project read and held. **Never stops Fleet**: what cannot be done
    /// is said in [`Migrated::left`].
    pub async fn projects_migrated(&self) -> Migrated {
        let mut done = Migrated::default();
        let kit = self.kit_folder();

        for old in files_in(&kit.join(WORKSPACES), PieceWritten::in_kit) {
            match parse_workspace_v1(old.path(), old.text()) {
                Ok(project) => match self.project_made_here(&project).await {
                    Ok(_) => {
                        let _ = std::fs::remove_file(old.path());
                        done.converted += 1;
                    }
                    Err(why) => done.left.push(why),
                },
                Err(why) => done.left.push(why.to_string()),
            }
        }

        for served in self.repositories().hosted() {
            let folder = Path::new(served.root()).join(".armada").join(WORKSPACES);
            for committed in files_in(&folder, PieceWritten::in_repository) {
                let source = committed.path().display().to_string();
                if self.store().lock().await.project_imported(&source).ok().flatten().is_some() {
                    continue;
                }
                let project = match parse_workspace_v1(committed.path(), committed.text()) {
                    Ok(project) => project,
                    Err(why) => {
                        done.left.push(why.to_string());
                        continue;
                    }
                };
                let id = project.id().as_str().to_string();
                let held = self.store().lock().await.projects().map(|kept| kept.projects.iter().any(|one| one.id == id)).unwrap_or(false);
                let there = file_safe(&id) && places::project_file(&kit, &id).exists();
                if !held && !there {
                    match self.project_made_here(&project).await {
                        Ok(_) => done.imported += 1,
                        Err(why) => {
                            done.left.push(why);
                            continue;
                        }
                    }
                }
                let _ = self.store().lock().await.record_project_import(&source, &id);
            }
        }

        let kept = self.store().lock().await.projects().map(|kept| kept.projects).unwrap_or_default();
        for one in kept {
            let chosen = self.store().lock().await.project_place(&one.id).ok().flatten();
            let elsewhere = chosen.is_some_and(|chosen| place_of_file(place_of(&chosen.place)) != config::ProjectPlace::ThisMac);
            if elsewhere || !file_safe(&one.id) || places::project_file(&kit, &one.id).exists() {
                continue;
            }
            let derived = ScopeId::new(&one.id).ok().and_then(|id| {
                let name = Title::new(&one.name).or_else(|_| Title::new(&one.id)).ok()?;
                Project::declared(id, name, members_of(&one)).ok()
            });
            let Some(project) = derived else {
                done.left.push(format!("the project {} could not be made a file", one.id));
                continue;
            };
            match self.project_file_here(&project) {
                Ok(()) => done.derived += 1,
                Err(why) => done.left.push(why),
            }
        }

        self.projects_reload().await;
        // After every project is held, so a project that already holds the repository is seen.
        if let Err(why) = self.built_in_projects_made().await {
            done.left.push(why);
        }
        done
    }

    /// `project` as the file on this Mac, and a row at the end of his order where it has none.
    pub(super) async fn project_made_here(&self, project: &Project) -> Result<(), String> {
        let id = project.id().as_str();
        if !file_safe(id) {
            return Err(format!("the project {id} has an id that cannot name a file"));
        }
        if !places::project_file(&self.kit_folder(), id).exists() {
            self.project_file_here(project)?;
        }
        let (name, at) = (project.name().as_str().to_string(), self.now());
        let mut store = self.store().lock().await;
        let held = store.projects().map_err(|why| why.to_string())?.projects.iter().any(|one| one.id == id);
        if !held {
            store.add_project(id, &name, at.as_str()).map_err(|why| why.to_string())?;
        }
        Ok(())
    }

    fn project_file_here(&self, project: &Project) -> Result<(), String> {
        let path = places::project_file(&self.kit_folder(), project.id().as_str());
        let text = project_text(project)?;
        crate::piece_saving::write_over(&path, &text).map_err(|why| format!("{} could not be written: {why}", path.display()))
    }
}
