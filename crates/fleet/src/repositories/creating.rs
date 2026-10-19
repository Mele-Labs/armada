//! `create_project`: a new folder under one a person picked, started as a git
//! repository, and served the way `add_repository` serves one.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AddRepository, CreateProject, RepositorySummary, WireError};

use super::cloning::{occupied, DESTINATION_OCCUPIED, NO_SUCH_FOLDER};
use crate::daemon::Fleet;

/// A name that is not one folder's. A 422.
const NOT_A_PROJECT_NAME: &str = "fleet.project_name_refused";
/// A folder that could not be made or started as a repository. A 500.
const PROJECT_NOT_STARTED: &str = "fleet.project_not_started";

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
    /// Spawned, so a caller that stops waiting does not strand a folder that was
    /// made and never served.
    pub(crate) async fn created_project(
        self: Arc<Self>,
        asked: CreateProject,
    ) -> Result<RepositorySummary, Refusal> {
        tokio::spawn(async move { self.creating(asked).await })
            .await
            .unwrap_or_else(|panicked| std::panic::resume_unwind(panicked.into_panic()))
    }

    async fn creating(&self, asked: CreateProject) -> Result<RepositorySummary, Refusal> {
        let name = asked.name.trim();
        if !is_one_folder(name) {
            let why = format!("`{name}` is not the name of one folder");
            return Err(self.raised(Refusal::Unacceptable, NOT_A_PROJECT_NAME, why));
        }
        let parent = Path::new(&asked.parent);
        let parent = match parent.is_absolute().then(|| parent.canonicalize()) {
            Some(Ok(found)) if found.is_dir() => found,
            Some(_) => {
                let why = format!("{} is not a folder on this machine", asked.parent);
                return Err(self.raised(Refusal::Unacceptable, NO_SUCH_FOLDER, why));
            }
            None => {
                let why = format!("{} is relative, and a project needs an absolute folder", asked.parent);
                return Err(self.raised(Refusal::Unacceptable, NO_SUCH_FOLDER, why));
            }
        };
        let destination = parent.join(name);
        let at = destination.to_string_lossy().to_string();
        if occupied(&destination) {
            let why = format!("{at} already exists and is not empty");
            return Err(self.raised(Refusal::IllegalMove, DESTINATION_OCCUPIED, why));
        }
        let Some(_claimed) = self.repositories().cloning().claim(&destination) else {
            let why = format!("a project at {at} is already being made");
            return Err(self.raised(Refusal::IllegalMove, DESTINATION_OCCUPIED, why));
        };
        let made = !destination.exists();
        let (locating, folder) = (Arc::clone(self.locating()), destination.clone());
        let started = tokio::task::spawn_blocking(move || {
            if made {
                std::fs::create_dir(&folder).map_err(|why| why.to_string())?;
            }
            locating.started(&folder).map_err(|why| why.cause().to_string())
        })
        .await
        .expect("starting a project panicked");
        if let Err(why) = started {
            // What this call made goes with it; an empty folder that was there stays.
            let _ = match made {
                true => std::fs::remove_dir_all(&destination),
                false => std::fs::remove_dir_all(destination.join(".git")),
            };
            let said = format!("{at} could not be started as a repository: {why}");
            return Err(self.raised(Refusal::Fault, PROJECT_NOT_STARTED, said));
        }
        self.added_repository(AddRepository { path: at }).await
    }

    fn raised(&self, kind: fn(WireError) -> Refusal, code: &str, why: String) -> Refusal {
        kind(WireError::raised(code, why, self.run_id()))
    }
}

/// One folder's name: no separator, not `.` or `..`, and something.
fn is_one_folder(name: &str) -> bool {
    !matches!(name, "" | "." | "..") && !name.contains(['/', '\\', '\0'])
}
