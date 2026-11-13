//! The project Fleet makes by itself for the armada repository, so Armada is ready to use on a Mac that
//! has never had a project (`docs/concepts/project.md`, *The built-in project*).
//!
//! **Once per Mac**, remembered in `project_imports` under `built-in:<manifest id>`, so a project he
//! deletes or renames is never made again. A repository some project already holds is recorded as made
//! and nothing is made. It never asks him.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{DeliverySetting, ManifestId, Member, Project, ScopeId, Title, Ulid, Values};

use crate::daemon::Fleet;

/// The manifest id of Armada's own repository.
const ARMADA: &str = "armada";
/// What the project is called.
const NAME: &str = "Armada";

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
    /// Make Armada's project if this Mac serves the repository and has not made it before, and hold it.
    /// **Never stops Fleet**: what cannot be made is not recorded, so the next start tries again.
    pub(crate) async fn built_in_projects_made(&self) -> Result<(), String> {
        if !self.repositories().served().iter().any(|one| one.manifest().id().as_str() == ARMADA) {
            return Ok(());
        }
        let source = format!("built-in:{ARMADA}");
        if self.store().lock().await.project_imported(&source).map_err(|why| why.to_string())?.is_some() {
            return Ok(());
        }
        let repository = ManifestId::carried(Ulid::carried(ARMADA));
        let held = self.projects_held().holding(&repository).first().map(|one| one.id().as_str().to_string());
        let project = match held {
            Some(id) => id,
            None => {
                let id = self.minted();
                let scope = ScopeId::new(&id).map_err(|why| why.to_string())?;
                let name = Title::new(NAME).map_err(|why| why.to_string())?;
                let made = Project::declared(scope, name, vec![Member::whole(repository)])
                    .map_err(|why| why.to_string())?
                    .with_values(Values { delivery: Some(DeliverySetting::AutoLand), ..Values::default() });
                self.project_made_here(&made).await?;
                self.projects_reload().await;
                id
            }
        };
        self.store().lock().await.record_project_import(&source, &project).map_err(|why| why.to_string())
    }
}
