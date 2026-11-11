//! The tracker a Job asks, and the tracker a project's issues are read from. `docs/concepts/tracker.md`,
//! *Choosing a tracker*.
//!
//! **A project that names a tracker replaces its members' trackers for the Jobs in it**, and a Job in no
//! project, or in one that names none, asks its repository's. The root stays the Job's own repository's:
//! only the choice moves. A call that is not tied to a Job asks [`Fleet::tracker_of`], the repository's.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Backlog, Delivery, Tracker, TrackerChoice, Vcs, WorkProduct};
use core_model::ScopeId;
use store::Store;

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::tracking::choice_of;

/// The project a Job was started for, by id, where it was started for one.
pub(crate) fn job_project(store: &Store, job: &str) -> Option<String> {
    store.hosting(job).ok().flatten().and_then(|hosting| hosting.project)
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
    /// The project a Job was started for, read off the ledger.
    pub(crate) async fn project_of_job(&self, job: &str) -> Option<String> {
        job_project(&*self.store().lock().await, job)
    }

    /// The tracker project `id` names, where it names one. **A project whose file is not held names none**, said
    /// on the log: the Job then asks its repository's, and is not stopped for a file it cannot see.
    pub(crate) fn project_tracker(&self, id: &str) -> Option<TrackerChoice> {
        let held = self.projects_held();
        let Some(project) = ScopeId::new(id).ok().and_then(|scope| held.get(&scope).cloned()) else {
            self.values_warned("a project's tracker was not read, so its repository's is asked", Some(id), "Fleet holds no readable file for it");
            return None;
        };
        let named = project.values().tracker.as_ref()?;
        Some(TrackerChoice::named(named.provider.as_deref(), &named.keys))
    }

    /// Whether project `id`'s file names a tracker. **Quiet where the file is not held**: a loose row has no project.
    pub(crate) fn project_names_tracker(&self, id: &str) -> bool {
        ScopeId::new(id).ok().is_some_and(|scope| self.projects_held().get(&scope).is_some_and(|project| project.values().tracker.is_some()))
    }

    /// What a Job in `served` and `project` chooses: the project's tracker where it names one, else the repository's.
    pub(crate) fn choice_in(&self, served: &Served, project: Option<&str>) -> TrackerChoice {
        project.and_then(|id| self.project_tracker(id)).unwrap_or_else(|| choice_of(served.manifest()))
    }

    /// The tracker a Job in `served` and `project` asks. Cloned, because every call to it runs off the async thread.
    pub(crate) fn tracker_for_job(&self, served: &Served, project: Option<&str>) -> Arc<dyn Tracker + Send + Sync> {
        self.trackers().tracker(&self.choice_in(served, project))
    }

    /// What writes to the tracker a Job in `served` and `project` asks, where it is one Fleet writes to.
    pub(crate) fn backlog_for_job(&self, served: &Served, project: Option<&str>) -> Option<Arc<dyn Backlog + Send + Sync>> {
        self.trackers().backlog(&self.choice_in(served, project))
    }
}
