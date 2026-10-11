//! A repository's open issues, read from its tracker for Bridge to choose
//! from. #91, `docs/concepts/tracker.md`.
//!
//! **Read on ask and held five minutes, never polled.** Nobody reads the list
//! while Bridge is not showing it, so a timer would spend the forge's rate
//! limit on nothing; a person who wants it newer presses refresh.
//!
//! **A view.** Listing makes no Job and writes nothing; the owner's call of
//! 11 Oct 2026. What is written back on an issue is `crate::writing_back`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, OpenIssue, Tracker, Vcs, WorkProduct};
use api::{Issues, Refusal};
use core_model::Timestamp;
use ipc::{Instant, IssueList, JobId, ManifestId, TrackedIssue, WireError};

use crate::converging::elapsed;
use crate::daemon::Fleet;

/// How long a list read from the tracker is answered again without asking.
pub(crate) const HELD_FOR: Duration = Duration::from_secs(5 * 60);

/// The tracker would not answer and nothing is held. A 500: asking again is reasonable.
const UNREADABLE: &str = "fleet.issues_unreadable";

/// The tracker, what was last read from it per repository, and when Fleet last
/// wrote back. **In memory**: a list is the tracker's, and is read again after
/// a restart; what was *said* on an issue is the store's.
pub(crate) struct Tracking {
    tracker: Arc<dyn Tracker + Send + Sync>,
    lists: std::sync::Mutex<BTreeMap<String, Held>>,
    pub(crate) written_back: tokio::sync::Mutex<Option<Timestamp>>,
    /// What Autopilot's pass remembers about these lists between intervals. #2092.
    pub(crate) autopilot: crate::autopilot::Remembered,
}

#[derive(Clone)]
struct Held {
    fetched_at: Timestamp,
    issues: Vec<OpenIssue>,
}

impl Tracking {
    pub(crate) fn with(tracker: Arc<dyn Tracker + Send + Sync>) -> Tracking {
        Tracking {
            tracker,
            lists: std::sync::Mutex::new(BTreeMap::new()),
            written_back: tokio::sync::Mutex::new(None),
            autopilot: crate::autopilot::Remembered::default(),
        }
    }

    /// Cloned, because every call to it runs off the async thread.
    pub(crate) fn tracker(&self) -> Arc<dyn Tracker + Send + Sync> {
        Arc::clone(&self.tracker)
    }

    fn held(&self, manifest: &str) -> Option<Held> {
        self.lists
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(manifest)
            .cloned()
    }

    fn keep(&self, manifest: &str, held: Held) {
        self.lists
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(manifest.to_string(), held);
    }
}

/// One issue on the wire, with the Job still going that came from it.
fn tracked(issue: &OpenIssue, going: &BTreeMap<String, JobId>) -> TrackedIssue {
    let url = issue.url.as_written().to_string();
    TrackedIssue {
        number: issue.number,
        title: issue.title.as_written().to_string(),
        job_id: going.get(&url).cloned(),
        url,
        labels: issue.labels.iter().map(|one| one.as_written().to_string()).collect(),
        assignees: issue
            .assignees
            .iter()
            .map(|one| one.as_written().to_string())
            .collect(),
        updated_at: Instant::carried(issue.updated_at.clone()),
        excerpt: issue.excerpt.as_written().to_string(),
        linked_pull_request: issue.linked_pull_request,
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
    /// The list as the wire carries it: each issue with the Job dispatched from
    /// it, where one has not ended.
    async fn issue_list(
        &self,
        manifest_id: ManifestId,
        held: Held,
        stale: Option<String>,
    ) -> Result<IssueList, Refusal> {
        let going: BTreeMap<String, JobId> = self
            .store()
            .lock()
            .await
            .issue_sources_in_flight()
            .map_err(|why| {
                Refusal::Fault(WireError::raised(
                    UNREADABLE,
                    format!("the Jobs that came from issues would not read: {why:?}"),
                    self.run_id(),
                ))
            })?
            .into_iter()
            .map(|(job, source)| (source.url, JobId::from(&job)))
            .collect();
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried(held.fetched_at.as_str()),
            issues: held.issues.iter().map(|one| tracked(one, &going)).collect(),
            stale,
        })
    }
}

impl<H, V, W> Issues for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_issues(
        &self,
        manifest_id: ManifestId,
        refresh: bool,
    ) -> Result<IssueList, Refusal> {
        let (held, stale) = self.held_or_read(&manifest_id, refresh).await?;
        self.issue_list(manifest_id, held, stale).await
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
    /// The open issues as Fleet holds them, read again where they are old: Autopilot picks from
    /// the same list Bridge shows, and with the same five minutes. #2092.
    pub(crate) async fn open_issues(&self, manifest_id: &ManifestId) -> Result<Vec<OpenIssue>, Refusal> {
        Ok(self.held_or_read(manifest_id, false).await?.0.issues)
    }

    /// What is held, or a fresh read, and why an older list was answered where one was.
    async fn held_or_read(
        &self,
        manifest_id: &ManifestId,
        refresh: bool,
    ) -> Result<(Held, Option<String>), Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let now = self.now();
        let held = self.tracking().held(manifest_id.as_str());
        if let Some(held) = held.as_ref().filter(|_| !refresh) {
            if elapsed(&held.fetched_at, &now) < HELD_FOR {
                return Ok((held.clone(), None));
            }
        }
        let tracker = self.tracking().tracker();
        let root = served.root().to_string();
        let read = tokio::task::spawn_blocking(move || tracker.open_issues(&root))
            .await
            .unwrap_or_else(|panicked| {
                Err(adapter_traits::NotTracked::of(
                    "listing the open issues",
                    panicked.to_string(),
                ))
            });
        match (read, held) {
            (Ok(issues), _) => {
                let fresh = Held {
                    fetched_at: now,
                    issues,
                };
                self.tracking().keep(manifest_id.as_str(), fresh.clone());
                Ok((fresh, None))
            }
            (Err(not), Some(held)) => {
                let said = format!("{} did not happen: {}", not.doing, not.said);
                Ok((held, Some(said)))
            }
            (Err(not), None) => Err(Refusal::Fault(WireError::raised(
                UNREADABLE,
                format!("{} did not happen: {}", not.doing, not.said),
                self.run_id(),
            ))),
        }
    }
}
