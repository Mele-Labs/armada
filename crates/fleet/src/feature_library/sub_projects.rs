//! Sub projects: the Jobs the owner picked in the Jobs list, kept under a name with the features they worked in at the
//! press. Fleet keeps only what he picked; the Jobs list adds the live Jobs that join from `list_features`.
//! `docs/concepts/feature-catalog.md`, *Sub projects*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Instant, SessionId, StartSubProject, SubProject, SubProjects, UngroupSubProject, WireError};
use store::KeptSubProject;

use crate::daemon::Fleet;
use crate::session_host::serving::NO_SUCH_SESSION;

const NO_NAME: &str = "fleet.sub_project_no_name";
const NO_JOBS: &str = "fleet.sub_project_no_jobs";
/// A picked Job is filed in no project, or in another one than the sub project's.
const ACROSS_PROJECTS: &str = "fleet.sub_project_across_projects";
const NO_SUCH_SUB_PROJECT: &str = "fleet.no_such_sub_project";

fn wire_of(kept: KeptSubProject) -> SubProject {
    SubProject {
        id: kept.id,
        project: kept.project,
        name: kept.name,
        jobs: kept.jobs.iter().map(SessionId::carried).collect(),
        features: kept.features,
        started_at: Instant::carried(kept.started_at),
    }
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
    /// `start_sub_project`.
    pub(super) async fn sub_project_started(&self, start: StartSubProject) -> Result<SubProject, Refusal> {
        let name = start.name.trim();
        if name.is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised(NO_NAME, "name the sub project", self.run_id())));
        }
        if start.jobs.is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised(NO_JOBS, "pick a Job to group", self.run_id())));
        }
        self.feature_writes_filed().await;
        let mut store = self.store().lock().await;
        for job in &start.jobs {
            if store.session(job.as_str()).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no job is named {}", job.as_str()), self.run_id())));
            }
        }
        for job in &start.jobs {
            if !self.projects_of_job_in(&store, job.as_str()).contains(&start.project) {
                return Err(Refusal::IllegalMove(WireError::raised(
                    ACROSS_PROJECTS,
                    format!("job {} is not filed in project {}: a sub project holds the Jobs of one project", job.as_str(), start.project),
                    self.run_id(),
                )));
            }
        }
        let on = self.jobs_on_features(&store, &start.project)?;
        let features = store
            .features_in(&start.project)
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|feature| on.get(&feature.id).is_some_and(|jobs| jobs.iter().any(|one| start.jobs.contains(&one.session_id))))
            .map(|feature| feature.slug)
            .collect();
        let kept = KeptSubProject {
            id: self.mint().ulid().as_str().to_lowercase(),
            project: start.project,
            name: name.to_string(),
            jobs: start.jobs.iter().map(|job| job.as_str().to_string()).collect(),
            features,
            started_at: self.now().as_str().to_string(),
        };
        store.put_sub_project(&kept).map_err(|why| self.ledger_fault(why))?;
        Ok(wire_of(kept))
    }

    /// `list_sub_projects`.
    pub(super) async fn sub_projects_listed(&self, project: Option<String>) -> Result<SubProjects, Refusal> {
        let project = project.as_deref().map(str::trim).filter(|project| !project.is_empty());
        let store = self.store().lock().await;
        let kept = store.sub_projects(project).map_err(|why| self.ledger_fault(why))?;
        Ok(SubProjects { sub_projects: kept.into_iter().map(wire_of).collect() })
    }

    /// `ungroup_sub_project`.
    pub(super) async fn sub_project_ungrouped(&self, ungroup: UngroupSubProject) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        match store.drop_sub_project(&ungroup.id).map_err(|why| self.ledger_fault(why))? {
            true => Ok(()),
            false => Err(Refusal::NotFound(WireError::raised(NO_SUCH_SUB_PROJECT, format!("no sub project is kept under {}", ungroup.id), self.run_id()))),
        }
    }
}
