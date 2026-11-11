//! The open issues of the tracker a project names. `docs/concepts/tracker.md`, *Choosing a tracker*.
//!
//! **Read on ask and held five minutes**, as a repository's list is (`crate::tracking`), and a view that writes nothing.
//! A project that names no tracker has no issues of its own, so it is refused rather than answered from a repository's:
//! that list is `list_issues`'s, one repository at a time.

use adapter_traits::{AgentHarness, Delivery, NotTracked, TrackerChoice, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Instant, ManifestId, ProjectIssueList};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::tracking::{not_tracked, tracked, Held, Holders, HELD_FOR, UNREADABLE};

/// The project names no tracker of its own. A 422.
pub(crate) const NO_TRACKER: &str = "fleet.project_no_tracker";

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
    /// `list_project_issues`.
    pub(crate) async fn project_issues_listed(&self, id: String, refresh: bool) -> Result<ProjectIssueList, Refusal> {
        let project = self.project_held(&id).await?;
        let Some(named) = project.values().tracker.as_ref() else {
            let said = format!("Project {id} names no tracker of its own, so it has no issues of its own: its repositories' are listed one at a time");
            return Err(Refusal::Unacceptable(self.project_refusal(NO_TRACKER, said)));
        };
        let choice = TrackerChoice::named(named.provider.as_deref(), &named.keys);
        // The read runs in the first member Fleet serves, else in the repository Fleet started in.
        let members: Vec<_> = project.members().iter().filter_map(|member| self.repositories().serving(member.repository().as_str())).collect();
        let root = match members.first() {
            Some(served) => served.root().to_string(),
            None => self.served_named(None)?.root().to_string(),
        };
        let key = format!("project:{id}");
        let now = self.now();
        let held = self.tracking().held(&key);
        let (held, stale) = match held.clone().filter(|held| !refresh && elapsed(&held.fetched_at, &now) < HELD_FOR) {
            Some(fresh) => (fresh, None),
            None => {
                let tracker = self.trackers().tracker(&choice);
                let read = tokio::task::spawn_blocking(move || tracker.open_issues(&root))
                    .await
                    .unwrap_or_else(|panicked| Err(NotTracked::of("listing the open issues", panicked.to_string())));
                match (read, held) {
                    (Ok(issues), _) => {
                        let fresh = Held { fetched_at: now, issues };
                        self.tracking().keep(&key, fresh.clone());
                        (fresh, None)
                    }
                    (Err(not), Some(held)) => (held, Some(format!("{} did not happen: {}", not.doing, not.said))),
                    (Err(not), None) => return Err(Refusal::Fault(not_tracked(self.run_id(), UNREADABLE, &not))),
                }
            }
        };
        let mut holders = Holders::new();
        for served in &members {
            let manifest = ManifestId::carried(served.manifest().id().as_str());
            for (issue, jobs) in self.issue_holders(&manifest).await? {
                holders.entry(issue).or_default().extend(jobs);
            }
        }
        holders.values_mut().for_each(|jobs: &mut Vec<String>| {
            jobs.sort_unstable();
            jobs.dedup();
        });
        let issues = held.issues.iter().map(|one| tracked(one, &holders)).collect();
        Ok(ProjectIssueList { project: id, fetched_at: Instant::carried(held.fetched_at.as_str()), issues, stale })
    }
}
