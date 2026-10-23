//! A repository's open issues, read from its tracker for Bridge to choose
//! from. #91, `docs/concepts/tracker.md`.
//!
//! **Read on ask and held five minutes, never polled.** Nobody reads the list
//! while Bridge is not showing it, so a timer would spend the forge's rate
//! limit on nothing; a person who wants it newer presses refresh.
//!
//! **A view.** Listing makes no Session and writes nothing; the owner's call of
//! 11 Oct 2026. What is written back on an issue is `crate::writing_back`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{
    AgentHarness, Delivery, LinkLookup, LinkedPullRequestState, OpenIssue, ReadIssue, Tracker, TrackerChoice,
    Vcs, WorkProduct,
};
use api::{Issues, Refusal};
use core_model::Timestamp;
use ipc::{
    Instant, IssueComment, IssueField, IssueList, IssueRead, IssueReference, IssueState, LinkedPullRequest,
    ManifestId, TrackedIssue, WireError,
};
use store::HolderKind;

use crate::clock::elapsed;
use crate::daemon::Fleet;

/// How long a list read from the tracker is answered again without asking.
pub(crate) const HELD_FOR: Duration = Duration::from_secs(5 * 60);

/// The tracker would not answer and nothing is held. A 500: asking again is reasonable.
const UNREADABLE: &str = "fleet.issues_unreadable";

/// The tracker has no issue by that reference, or the number is a pull request's. A 404.
const NO_SUCH_ISSUE: &str = "fleet.no_such_issue";

/// What was last read from each repository's tracker. **In memory**: a list is the tracker's, and is read again after
/// a restart; what was *said* on an issue is the store's.
pub(crate) struct Tracking {
    lists: std::sync::Mutex<BTreeMap<String, Held>>,
    /// What Autopilot's pass remembers about these lists between intervals. #2092.
    pub(crate) autopilot: crate::autopilot::Remembered,
}

#[derive(Clone)]
struct Held {
    fetched_at: Timestamp,
    issues: Vec<OpenIssue>,
}

impl Tracking {
    pub(crate) fn new() -> Tracking {
        Tracking {
            lists: std::sync::Mutex::new(BTreeMap::new()),
            autopilot: crate::autopilot::Remembered::default(),
        }
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

/// What a Manifest chose, as the seam takes it. **No `tracker:` is the forge's own**, which is
/// what every repository had before the key existed.
fn choice_of(manifest: &config::Manifest) -> TrackerChoice {
    match manifest.tracker() {
        Some(block) => TrackerChoice::named(block.provider(), block.keys()),
        None => TrackerChoice::of_the_forge(),
    }
}

/// An issue's fields on the wire.
fn fields_of(fields: &[adapter_traits::IssueField]) -> Vec<IssueField> {
    fields
        .iter()
        .map(|field| IssueField {
            name: field.name.clone(),
            values: field.values.iter().map(|one| one.as_written().to_string()).collect(),
        })
        .collect()
}

/// The Jobs holding each issue of one repository, by the ledger's target for it. Ids, ascending.
pub(crate) type Holders = BTreeMap<String, Vec<String>>;

fn holders_at(holders: &Holders, reference: &str) -> Vec<String> {
    IssueReference::read(reference)
        .and_then(|issue| holders.get(issue.as_str()).cloned())
        .unwrap_or_default()
}

/// One issue on the wire.
pub(crate) fn tracked(issue: &OpenIssue, holders: &Holders) -> TrackedIssue {
    let url = issue.url.as_written().to_string();
    let author = issue.author.as_written().trim();
    TrackedIssue {
        reference: issue.reference.clone(),
        title: issue.title.as_written().to_string(),
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
        author: (!author.is_empty()).then(|| author.to_string()),
        state: if issue.closed { IssueState::Closed } else { IssueState::Open },
        fields: fields_of(&issue.fields),
        holders: holders_at(holders, &issue.reference),
    }
}

/// One issue read whole, on the wire.
fn read_whole(issue: ReadIssue, holders: &Holders) -> IssueRead {
    let written = |one: &adapter_traits::FromOutside| one.as_written().to_string();
    let holders = holders_at(holders, &issue.reference);
    IssueRead {
        reference: issue.reference,
        title: written(&issue.title),
        url: written(&issue.url),
        state: match issue.closed {
            true => IssueState::Closed,
            false => IssueState::Open,
        },
        body: written(&issue.body),
        author: written(&issue.author),
        opened_at: Instant::carried(issue.opened_at),
        labels: issue.labels.iter().map(written).collect(),
        assignees: issue.assignees.iter().map(written).collect(),
        comments: issue
            .comments
            .iter()
            .map(|one| IssueComment {
                author: written(&one.author),
                at: Instant::carried(one.at.clone()),
                body: written(&one.body),
            })
            .collect(),
        linked_pull_requests: issue
            .linked_pull_requests
            .iter()
            .map(|one| LinkedPullRequest {
                number: one.number,
                title: written(&one.title),
                url: written(&one.url),
                state: match one.state {
                    LinkedPullRequestState::Open => ipc::LinkedPullRequestState::Open,
                    LinkedPullRequestState::Merged => ipc::LinkedPullRequestState::Merged,
                    LinkedPullRequestState::Closed => ipc::LinkedPullRequestState::Closed,
                },
            })
            .collect(),
        fields: fields_of(&issue.fields),
        holders,
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
    /// The tracker `served`'s Manifest chose. Cloned, because every call to it
    /// runs off the async thread.
    pub(crate) fn tracker_of(&self, served: &crate::repositories::Served) -> Arc<dyn Tracker + Send + Sync> {
        self.trackers().tracker(&choice_of(served.manifest()))
    }

    /// What reads a link into the tracker `served`'s Manifest chose.
    pub(crate) fn links_of(&self, served: &crate::repositories::Served) -> Arc<dyn LinkLookup + Send + Sync> {
        self.trackers().links(&choice_of(served.manifest()))
    }

    /// The list as the wire carries it.
    async fn issue_list(
        &self,
        manifest_id: ManifestId,
        held: Held,
        stale: Option<String>,
    ) -> Result<IssueList, Refusal> {
        let holders = self.issue_holders(&manifest_id).await?;
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried(held.fetched_at.as_str()),
            issues: held.issues.iter().map(|one| tracked(one, &holders)).collect(),
            stale,
        })
    }

    /// The Jobs holding each issue of the repository, read off the ledger once, whichever tracker the
    /// issues are on. **Standing rows only**: a Job that gave an issue back no longer works it.
    pub(crate) async fn issue_holders(&self, manifest_id: &ManifestId) -> Result<Holders, Refusal> {
        let rows = self
            .store()
            .lock()
            .await
            .standing_of_kind(crate::session_issues::ISSUE, manifest_id.as_str())
            .map_err(|why| self.ledger_fault(why))?;
        let mut holders = Holders::new();
        for row in rows.into_iter().filter(|row| row.holder.kind == HolderKind::Session) {
            holders.entry(row.target).or_default().push(row.holder.id);
        }
        holders.values_mut().for_each(|ids| {
            ids.sort_unstable();
            ids.dedup();
        });
        Ok(holders)
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

    /// **Read from the tracker each time**: it is for an issue the held list does not have, which
    /// is closed or not an issue, and a person pressing it wants what is there now.
    async fn get_issue(&self, manifest_id: ManifestId, reference: String) -> Result<IssueRead, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let tracker = self.tracker_of(&served);
        let root = served.root().to_string();
        let asked = reference.clone();
        let read = tokio::task::spawn_blocking(move || tracker.read_issue(&root, &asked))
            .await
            .unwrap_or_else(|panicked| {
                Err(adapter_traits::NotTracked::of(
                    "reading the issue",
                    panicked.to_string(),
                ))
            });
        match read {
            Ok(Some(issue)) => Ok(read_whole(issue, &self.issue_holders(&manifest_id).await?)),
            Ok(None) => Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_ISSUE,
                format!("The tracker has no issue {reference}"),
                self.run_id(),
            ))),
            Err(not) => Err(Refusal::Fault(WireError::raised(
                UNREADABLE,
                format!("{} did not happen: {}", not.doing, not.said),
                self.run_id(),
            ))),
        }
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
        let tracker = self.tracker_of(&served);
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
