//! The pull requests waiting on a person, and resolving a conversation on one.
//! `docs/concepts/fleet.md`, *The pull requests waiting on a person*.
//!
//! **Read when somebody looks and never polled**: Fleet keeps no copy of other people's pull
//! requests, so the reply is as old as the forge's answer. Four lists, one forge call each read at
//! once, each naming every served repository; a list the forge would not answer is empty and named in
//! `silent`, so one failure does not blank the others. A fifth, read beside them, is the open pull
//! requests he ran a review tool on that no list names: Fleet's record of runs, read by number.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, ListFailure, ListedChecks, ListedPull, NumberedPull, ReviewList, ReviewListAsk, Vcs, WorkProduct,
};
use api::Refusal;
use ipc::{
    ManifestId, PullInbox, PullReviewChecks, PullReviewHolder, PullReviewList, PullReviewLists, PullReviewRow, PullThreadResolved,
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

/// The kinds of run that are reviewing a pull request, as `review_runs` keeps them: a tool or an
/// explanation, the change sets, the overview and a practice check. Not a question, a fix or a search.
const REVIEWING_KINDS: [&str; 5] = ["tool", "explanation", "change_sets", "overview", "practices"];

/// The most pull requests read by number for the fifth list. More than it shows, as some have closed.
const MOST_READ_BY_NUMBER: usize = 2 * MOST_PER_LIST;

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
        asked_at: pull.asked_at,
        team: pull.team,
        holder: None,
        seen: None,
        my_review: pull.my_latest.map(crate::pull_review_memory::my_review_of),
        inbox: PullInbox::NeedsFirstLook,
        reviewing: false,
        ignored: false,
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

    /// Each pull request of a served repository he ran a review tool on, newest run first.
    pub(crate) async fn reviewed_pulls(&self, served: &[Named]) -> Vec<(ManifestId, u64)> {
        let ran = self.store().lock().await.review_run_pulls(&REVIEWING_KINDS).unwrap_or_default();
        ran.into_iter()
            .filter(|it| served.iter().any(|named| named.manifest_id.as_str() == it.manifest_id))
            .map(|it| (ManifestId::carried(&it.manifest_id), it.number))
            .collect()
    }

    /// The four lists, and the fifth beside them, read at once: the forge answers each in a call,
    /// and a person waits on the slowest of five calls rather than Fleet holding a copy that goes
    /// stale.
    ///
    /// With `project`, only that project's member repositories are asked about, so every list keeps
    /// the rows of its members and no others. A project with no served member has five empty lists.
    pub(crate) async fn review_lists_read(&self, project: Option<&str>) -> Result<PullReviewLists, Refusal> {
        let members = match project {
            Some(id) => Some(self.project_repositories(id).await?),
            None => None,
        };
        let from = self.served_named(None)?;
        let unreadable = |said: &str| Refusal::Fault(self.pr_refusal(LISTS_UNREADABLE, said));
        let Some(me) = self.mine_in(from.root()).await.login().map(str::to_string) else {
            return Err(unreadable("nobody is signed in to the forge, so there is no one to list for"));
        };
        let mut served = self.named_repositories().await;
        if let Some(members) = &members {
            served.retain(|it| members.iter().any(|member| member.as_str() == it.manifest_id.as_str()));
            if served.is_empty() {
                return Ok(self.lists_of(me, Default::default()));
            }
        }
        let repositories: Vec<String> = served.iter().map(|it| it.slug.clone()).collect();
        if repositories.is_empty() {
            return Err(unreadable("no repository served has a remote the forge could be asked about"));
        }
        // The forge's wait is on: no call goes out, and the lists last read are shown, marked.
        let now = self.clock().now().epoch_millis().unwrap_or(0);
        if let Some(hold) = self.review_lists_kept().held(now) {
            return self.lists_limited(project, hold).await;
        }
        let began = std::time::Instant::now();
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
        let reviewed = self.reviewed_pulls(&served).await;
        let numbered: Vec<NumberedPull> = reviewed
            .iter()
            .take(MOST_READ_BY_NUMBER)
            .filter_map(|(manifest_id, number)| {
                let named = served.iter().find(|it| &it.manifest_id == manifest_id)?;
                Some(NumberedPull { repository: named.slug.clone(), number: *number })
            })
            .collect();
        // Beside the four rather than after them, so it costs no wait; one a list names is dropped below.
        let by_number = async {
            if numbered.is_empty() {
                return Some(Vec::new());
            }
            let login = me.clone();
            self.forge_asked(from.root(), move |vcs: &V, root: &str| vcs.open_pulls_numbered(root, &login, &numbered))
                .await
        };
        let [requested, team, assigned, mine] = LISTS.map(|(list, _)| list);
        let answers = tokio::join!(read(requested), read(team), read(assigned), read(mine), by_number);
        let numbered = answers.4;
        let answers = [answers.0, answers.1, answers.2, answers.3];
        let forge_ms = began.elapsed().as_millis();
        let mut lists: [Vec<PullReviewRow>; 5] = Default::default();
        let mut silent = Vec::new();
        let mut failures = Vec::new();
        for (at, ((_, wire), asked)) in LISTS.into_iter().zip(answers).enumerate() {
            match asked {
                Ok(pulls) => {
                    lists[at] = pulls
                        .into_iter()
                        .filter_map(|pull| row_of(pull, &served))
                        .take(MOST_PER_LIST)
                        .collect();
                }
                Err(failure) => {
                    silent.push(wire);
                    failures.push(failure);
                }
            }
        }
        if silent.len() == LISTS.len() {
            return match crate::pull_review_kept::worst(failures) {
                Some(ListFailure::RateLimited { until, said }) => {
                    let hold = self.review_lists_kept().hold(now, until, said);
                    self.lists_limited(project, hold).await
                }
                Some(other) => Err(unreadable(other.said())),
                None => Err(unreadable("The forge did not answer. Open Reviews again.")),
            };
        }
        let was_reviewed = |row: &PullReviewRow| reviewed.iter().any(|(id, number)| *id == row.manifest_id && *number == row.number);
        for row in lists.iter_mut().flatten() {
            row.reviewing = was_reviewed(row);
        }
        match numbered {
            Some(pulls) => {
                let listed = |row: &PullReviewRow| {
                    lists.iter().flatten().any(|it| it.manifest_id == row.manifest_id && it.number == row.number)
                };
                let mut rows: Vec<PullReviewRow> = pulls
                    .into_iter()
                    .filter_map(|pull| row_of(pull, &served))
                    .filter(|row| was_reviewed(row) && !listed(row))
                    .collect();
                rows.sort_by_key(|row| reviewed.iter().position(|(id, number)| *id == row.manifest_id && *number == row.number));
                rows.dedup_by(|one, other| one.manifest_id == other.manifest_id && one.number == other.number);
                rows.truncate(MOST_PER_LIST);
                for row in &mut rows {
                    row.reviewing = true;
                }
                lists[4] = rows;
            }
            None => silent.push(PullReviewList::Reviewing),
        }
        let overlaid = self.overlays_onto(&mut lists.iter_mut().collect::<Vec<_>>()).await;
        eprintln!("review_lists timing: forge {forge_ms}ms; {overlaid}; total {}ms", began.elapsed().as_millis());
        let mut read = self.lists_of(me, lists);
        read.silent = silent;
        self.keep_lists(project, &read).await;
        Ok(read)
    }

    /// The lists last read, marked as kept for the rate limit; with none read, the refusal that names it.
    pub(crate) async fn lists_limited(
        &self,
        project: Option<&str>,
        hold: crate::pull_review_kept::Hold,
    ) -> Result<PullReviewLists, Refusal> {
        let kept = self.kept_lists(project).await;
        crate::pull_review_kept::limited(kept, hold).map_err(|(until, said)| {
            crate::pull_review_kept::rate_limited_refusal(|code, message| self.pr_refusal(code, message), until, said)
        })
    }

    /// What Fleet knows locally of each row: the Session that holds it, his review written here where the
    /// forge does not show it yet, and where it stands in his inbox, from it and the head he last left it at.
    /// Cheap and local, so it is applied again to lists kept. Answers how long each took, for the timing line.
    pub(crate) async fn overlays_onto(&self, lists: &mut [&mut Vec<PullReviewRow>]) -> String {
        let began = std::time::Instant::now();
        let mut rows = 0;
        for row in lists.iter_mut().flat_map(|list| list.iter_mut()) {
            row.holder = self.holder_of(row).await;
            rows += 1;
        }
        let holder_ms = began.elapsed().as_millis();
        let (mut written_ms, mut inbox_ms) = (0, 0);
        for list in lists.iter_mut() {
            let at = std::time::Instant::now();
            self.written_onto(list).await;
            written_ms += at.elapsed().as_millis();
            let at = std::time::Instant::now();
            self.inbox_onto(list).await;
            inbox_ms += at.elapsed().as_millis();
        }
        // Only the list that asks him by name, the first.
        let at = std::time::Instant::now();
        if let Some(requested) = lists.first_mut() {
            self.asked_told_onto(requested).await;
        }
        let told_ms = at.elapsed().as_millis();
        format!(
            "rows {rows}: holder_of {holder_ms}ms, written_onto {written_ms}ms, inbox_onto {inbox_ms}ms, asked_told_onto {told_ms}ms (rows {}ms)",
            began.elapsed().as_millis()
        )
    }

    fn lists_of(&self, me: String, lists: [Vec<PullReviewRow>; 5]) -> PullReviewLists {
        let [requested, team, assigned, mine, reviewing] = lists;
        PullReviewLists {
            me,
            fetched_at: self.clock().now().as_str().to_string(),
            requested,
            team,
            assigned,
            mine,
            reviewing,
            silent: Vec::new(),
            rate_limited: None,
            refreshing: false,
        }
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
