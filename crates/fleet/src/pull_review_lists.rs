//! The pull requests waiting on a person, and resolving a conversation on one.
//! `docs/concepts/fleet.md`, *The pull requests waiting on a person*.
//!
//! **Read when somebody looks and never polled**: Fleet keeps no copy of other people's pull
//! requests, so the reply is as old as the forge's answer. Four lists, one forge call each read at
//! once, each naming every served repository; a list the forge would not answer is empty and named in
//! `silent`, so one failure does not blank the others.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, ListedChecks, ListedPull, ReviewList, ReviewListAsk, Vcs, WorkProduct,
};
use api::Refusal;
use ipc::{
    ManifestId, PullReviewChecks, PullReviewHolder, PullReviewList, PullReviewLists, PullReviewRow, PullThreadResolved,
    ResolvePullThread, MOST_PER_LIST,
};

use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::pull_request_panel::review_state;

/// No list could be read, or nobody is known to ask as. A 500.
pub(crate) const LISTS_UNREADABLE: &str = "fleet.review_lists_unreadable";
/// The forge would not resolve the thread, in its own sentence. A 409.
const THREAD_REFUSED: &str = "fleet.review_thread_refused";

const LISTS: [(ReviewList, PullReviewList); 4] = [
    (ReviewList::Requested, PullReviewList::Requested),
    (ReviewList::Team, PullReviewList::Team),
    (ReviewList::Assigned, PullReviewList::Assigned),
    (ReviewList::Mine, PullReviewList::Mine),
];

/// A served repository as the forge names it, lowercase, with the id it answers to.
pub(crate) struct Named {
    pub(crate) manifest_id: ManifestId,
    pub(crate) slug: String,
    /// Where the repository is on this machine, which is where the forge is asked about it.
    pub(crate) root: String,
}

/// A row of a list, if its repository is one Fleet serves.
pub(crate) fn row_of(pull: ListedPull, served: &[Named]) -> Option<PullReviewRow> {
    let manifest_id = served
        .iter()
        .find(|it| it.slug.eq_ignore_ascii_case(&pull.repository))?
        .manifest_id
        .clone();
    Some(PullReviewRow {
        manifest_id,
        repository: pull.repository,
        number: pull.number,
        title: pull.title,
        branch: pull.branch,
        address: pull.address,
        draft: pull.draft,
        checks: match pull.checks {
            ListedChecks::Passed => PullReviewChecks::Passed,
            ListedChecks::Failed => PullReviewChecks::Failed,
            ListedChecks::Running => PullReviewChecks::Running,
        },
        review: review_state(pull.review),
        head: pull.head,
        author: pull.author,
        updated_at: pull.updated_at,
        team: pull.team,
        holder: None,
    })
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
    /// Every served repository the forge knows by name. One with no remote has none, and is not
    /// searched.
    pub(crate) async fn named_repositories(&self) -> Vec<Named> {
        let held: Vec<(ManifestId, String)> = self
            .repositories()
            .served()
            .iter()
            .map(|it| (ManifestId::carried(it.manifest().id().as_str()), it.root().to_string()))
            .collect();
        let reaching = Arc::clone(self.reaching());
        tokio::task::spawn_blocking(move || {
            held.into_iter()
                .filter_map(|(manifest_id, root)| {
                    let slug = reaching.slug(&root)?.to_lowercase();
                    Some(Named { manifest_id, slug, root })
                })
                .collect()
        })
        .await
        .unwrap_or_default()
    }

    /// The Session that holds a pull request, as the pull request owners say.
    pub(crate) async fn holder_of(&self, row: &PullReviewRow) -> Option<PullReviewHolder> {
        let served = self.served_named(Some(&row.manifest_id)).ok()?;
        let owners = self.owners_of(&served, row.number, &row.branch, false).await;
        let id = owners.into_iter().next().map(|Owner::Session(id)| id)?;
        let title = match self.store().lock().await.session(&id) {
            Ok(Some(record)) => record.title.unwrap_or_default(),
            _ => String::new(),
        };
        Some(PullReviewHolder { id, title })
    }

    /// The four lists, read at once: the forge answers each in a call, and a person waits on
    /// the slowest of four calls rather than Fleet holding a copy that goes stale.
    pub(crate) async fn review_lists_read(&self) -> Result<PullReviewLists, Refusal> {
        let from = self.served_named(None)?;
        let unreadable = |said: &str| Refusal::Fault(self.pr_refusal(LISTS_UNREADABLE, said));
        let Some(me) = self.mine_in(from.root()).await.login().map(str::to_string) else {
            return Err(unreadable("nobody is signed in to the forge, so there is no one to list for"));
        };
        let served = self.named_repositories().await;
        let repositories: Vec<String> = served.iter().map(|it| it.slug.clone()).collect();
        if repositories.is_empty() {
            return Err(unreadable("no repository served has a remote the forge could be asked about"));
        }
        let read = |list: ReviewList| {
            let (login, repositories) = (me.clone(), repositories.clone());
            self.forge_asked(from.root(), move |vcs: &V, root: &str| {
                vcs.pull_review_list(
                    root,
                    &ReviewListAsk {
                        list,
                        login: &login,
                        repositories: &repositories,
                        most: MOST_PER_LIST,
                    },
                )
            })
        };
        // The four do not wait on each other, so a person waits on the slowest, not the sum.
        let [requested, team, assigned, mine] = LISTS.map(|(list, _)| list);
        let answers = tokio::join!(read(requested), read(team), read(assigned), read(mine));
        let answers = [answers.0, answers.1, answers.2, answers.3];
        let mut lists: [Vec<PullReviewRow>; 4] = Default::default();
        let mut silent = Vec::new();
        for (at, ((_, wire), asked)) in LISTS.into_iter().zip(answers).enumerate() {
            match asked {
                Some(pulls) => {
                    lists[at] = pulls
                        .into_iter()
                        .filter_map(|pull| row_of(pull, &served))
                        .take(MOST_PER_LIST)
                        .collect();
                }
                None => silent.push(wire),
            }
        }
        if silent.len() == LISTS.len() {
            return Err(unreadable("the forge would not answer any of the four lists"));
        }
        // Which Session of the owner's works each pull request, for a note that goes to it.
        for row in lists.iter_mut().flatten() {
            row.holder = self.holder_of(row).await;
        }
        let [requested, team, assigned, mine] = lists;
        Ok(PullReviewLists {
            me,
            fetched_at: self.clock().now().as_str().to_string(),
            requested,
            team,
            assigned,
            mine,
            silent,
        })
    }

    /// Resolve or reopen a thread, as the forge now holds it.
    pub(crate) fn review_thread_resolved(
        &self,
        root: &str,
        resolve: ResolvePullThread,
    ) -> Result<PullThreadResolved, Refusal> {
        let resolved = self
            .vcs()
            .resolve_review_thread(root, &resolve.thread, resolve.resolved)
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(THREAD_REFUSED, said)))?;
        Ok(PullThreadResolved {
            thread: resolve.thread,
            resolved,
        })
    }
}
