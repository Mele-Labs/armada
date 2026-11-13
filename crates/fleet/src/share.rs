//! What may leave this Mac for his other Macs, and what an act from one may reach. `docs/concepts/project.md`,
//! *Sharing with his other Macs*.
//!
//! **One gate.** Anything sent to another Mac is a [`Crosses`], and only this module makes one. It reads the
//! Job's project, and that project's switch, at the moment of sending, so a project turned off stops what it
//! held at once and the other Mac is never sent private work to hide.
//!
//! **A Job in an unshared project and a Job in none are one case**: nothing of either leaves, and an act on
//! either is answered as an act on a Job that does not exist. The phone's routes do not pass here yet.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::ScopeId;
use ipc::{BugReports, Event, Projects, SessionId, WireError};

use crate::daemon::Fleet;
use crate::projects::NO_SUCH_PROJECT;
use crate::session_host::NO_SUCH_SESSION;

/// Something the share filter let through to another Mac. **Made only here**, so a link that sends only these
/// cannot send what the filter did not see.
#[derive(Debug)]
pub struct Crosses<T>(T);

impl<T> Crosses<T> {
    /// What crosses.
    pub fn get(&self) -> &T {
        &self.0
    }

    /// What crosses, taken to be sent.
    pub fn sent(self) -> T {
        self.0
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
    /// Whether project `id` is shared with his other Macs: its file read, and the switch on.
    pub fn project_shared(&self, id: &str) -> bool {
        ScopeId::new(id).ok().and_then(|scope| self.projects_held().get(&scope).map(|one| one.shared_with_macs())).unwrap_or(false)
    }

    /// Whether Job `job` is in a shared project. A Job in none, and a Job this Fleet does not hold, is not.
    pub async fn job_shared(&self, job: &str) -> bool {
        match self.project_of_job(job).await {
            Some(project) => self.project_shared(&project),
            None => false,
        }
    }

    /// A Job's row, its thread, its live changes, its documents, sketches and shown files: `item`, where
    /// Job `job` is in a shared project.
    pub async fn of_job_for_other_mac<T>(&self, job: &SessionId, item: T) -> Option<Crosses<T>> {
        self.job_shared(job.as_str()).await.then_some(Crosses(item))
    }

    /// `event`, where it may be sent to another Mac whole or cut to what is shared. **Every kind is ruled on
    /// here by name**, so a new kind does not build until someone says whether it crosses. A kind that is
    /// not about one Job, or is about this Mac (its servers, repositories, settings, mods, Operator, plan
    /// and terminals), stays.
    pub async fn event_for_other_mac(&self, event: Event) -> Option<Crosses<Event>> {
        let job = match &event {
            Event::SessionChanged(record) => record.id.clone(),
            Event::SessionRow(row) => row.session_id.clone(),
            Event::SessionSubagent(subagent) => subagent.session_id.clone(),
            Event::JobThreadTail(tail) => tail.session_id.clone(),
            Event::JobThreadChanged(changed) => changed.session_id.clone(),
            Event::ProjectsChanged(projects) => {
                return Some(Crosses(Event::ProjectsChanged(self.projects_for_other_mac(projects.clone()).sent())));
            }
            Event::BugReportsChanged(reports) => {
                return Some(Crosses(Event::BugReportsChanged(self.bug_reports_for_other_mac(reports.clone()).await.sent())));
            }
            Event::ManifestReread(_)
            | Event::CheckoutRunFinished(_)
            | Event::ServerStarting(_)
            | Event::ServerServing(_)
            | Event::ServerExited(_)
            | Event::AppRunChanged(_)
            | Event::RepositoriesChanged(_)
            | Event::MergeLinesChanged(_)
            | Event::MergeLineViewChanged(_)
            | Event::WriteClashesChanged(_)
            | Event::CallsChanged(_)
            | Event::ModsChanged(_)
            | Event::SettingsChanged(_)
            | Event::StudioChanged(_)
            | Event::StudioDeleted(_)
            | Event::StudioJobActed(_)
            | Event::WordsSent(_)
            | Event::OperatorBoardChanged(_)
            | Event::OperatorPageChanged(_)
            | Event::OperatorDispatched(_)
            | Event::TerminalOpened(_)
            | Event::TerminalExited(_)
            | Event::TerminalShown(_)
            | Event::BrowserRequest(_)
            | Event::BrowserAskAnswered(_)
            | Event::AutopilotChanged(_)
            | Event::AutopilotDecisionsChanged(_)
            | Event::PlanUsageChanged(_)
            | Event::ModelDownshiftChanged(_)
            | Event::AgentSignInChanged(_)
            | Event::ReviewRunChanged { .. }
            | Event::ReviewWalkChanged { .. }
            | Event::WatchesChanged(_)
            | Event::JobLifecyclesChanged(_)
            | Event::KeepDraftsChanged(_)
            | Event::TimelineAppended(_)
            | Event::ActFailed(_) => return None,
        };
        self.of_job_for_other_mac(&job, event).await
    }

    /// An act from another Mac on Job `job` (a message, an answer, an approval, a stop, a merge) goes on only
    /// where the Job is in a shared project. **Otherwise it is refused exactly as an act on a Job this Fleet
    /// does not hold**, so the refusal does not say the Job is here.
    pub async fn act_from_other_mac(&self, job: &SessionId) -> Result<(), Refusal> {
        if self.job_shared(job.as_str()).await {
            return Ok(());
        }
        Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("No Job has the id {}.", job.as_str()), self.run_id())))
    }

    /// A Job started from another Mac goes only into a shared project. An unshared project, one he does not
    /// have, and none at all are refused alike, so the refusal names no project of this Mac.
    pub fn start_from_other_mac(&self, project: Option<&str>) -> Result<(), Refusal> {
        match project {
            Some(id) if self.project_shared(id) => Ok(()),
            Some(id) => Err(Refusal::NotFound(WireError::raised(NO_SUCH_PROJECT, format!("there is no project {id}"), self.run_id()))),
            None => Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_PROJECT,
                "a Job started from another Mac needs a project",
                self.run_id(),
            ))),
        }
    }

    /// A message from Job `from` to a Job on another Mac leaves only where `from` is in a shared project. The
    /// receiving Mac answers for its own Job as [`Fleet::act_from_other_mac`] does, so both must be shared.
    pub async fn message_to_other_mac<T>(&self, from: &SessionId, message: T) -> Option<Crosses<T>> {
        self.of_job_for_other_mac(from, message).await
    }

    /// The bug reports filed by Jobs in shared projects. **A report's text can quote private work**, so it
    /// follows its Job, and one filed by no Job stays.
    pub async fn bug_reports_for_other_mac(&self, reports: BugReports) -> Crosses<BugReports> {
        let mut kept = Vec::with_capacity(reports.reports.len());
        for report in reports.reports {
            if self.job_shared(&report.session_id).await {
                kept.push(report);
            }
        }
        Crosses(BugReports { reports: kept })
    }

    /// The projects named to another Mac: the shared ones, with their tasks. Tasks in no project and files that
    /// would not read stay.
    pub fn projects_for_other_mac(&self, projects: Projects) -> Crosses<Projects> {
        let shared = projects.projects.into_iter().filter(|one| one.unread.is_none() && self.project_shared(&one.id)).collect();
        Crosses(Projects { projects: shared, loose: Vec::new(), left_out: Vec::new() })
    }
}
