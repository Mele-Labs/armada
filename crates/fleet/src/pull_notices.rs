//! Telling the owner of a pull request what happened to it, so nobody has to ask a session to
//! watch. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **Each notice is told once.** The key is (pull request, commit, cause, recipient) in the
//! store, so a restart reads what was already told and a failure is told again only for a commit
//! that fails anew. **A job that hung is not a failure**: one cancelled after about the whole
//! time a job is given is started again once, through the forge adapter, and nothing is said.
//!
//! A Session is sent a message from Fleet that wakes it. A merge is a note in an old Job's log, and
//! a failure is told to no Job. **A recipient that cannot be told now is asked again next
//! reading**, with nothing kept. Who owns a pull request is `crate::pull_owners`.
//!
//! **A pull request that will merge itself settles the Session's waits on it at every reading**, not
//! only the first: auto-merge on or a place in the merge queue, and an item stated after the notice
//! is taken off the next time. The take-off is its own guard, so a reading that takes nothing says
//! nothing.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CiState, Delivery, PullQueue, Vcs, FollowedCheck, FollowedPull, WorkProduct,
};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::my_pulls::Mine;
use crate::pull_waits::PullNamed;
use crate::pull_owners::Owner;
use crate::repositories::Served;

/// How many lines of a failing job's log go in the message.
const TAIL_LINES: usize = 15;

/// How many failing jobs' logs are read for it.
const LOGS_READ: usize = 2;

/// A merge older than this is history, not news: the newest few are listed on every reading.
const MERGE_IS_NEWS: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// The key a hung job's single rerun is kept under.
const RERUN: &str = "rerun";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    ChecksFailed,
    Conflicts,
    Dropped,
    Unmergeable,
    Merged,
    AutoMerge,
    Queued,
}

impl Cause {
    fn key(self) -> &'static str {
        match self {
            Cause::ChecksFailed => "checks_failed",
            Cause::Conflicts => "conflicts",
            Cause::Dropped => "dropped",
            Cause::Unmergeable => "unmergeable",
            Cause::Merged => "merged",
            Cause::AutoMerge => "auto_merge",
            // Never kept: a queued pull request is said only as the lead of a take-off.
            Cause::Queued => "queued",
        }
    }
}

impl Cause {
    fn notice(self) -> ipc::Notice {
        match self {
            Cause::ChecksFailed => ipc::Notice::PrChecksFailed,
            Cause::Conflicts => ipc::Notice::PrConflicts,
            Cause::Dropped => ipc::Notice::PrDropped,
            Cause::Unmergeable => ipc::Notice::PrUnmergeable,
            Cause::Merged => ipc::Notice::PrMerged,
            Cause::AutoMerge => ipc::Notice::PrAutoMerge,
            Cause::Queued => ipc::Notice::PrQueued,
        }
    }
}

/// The pull request a notice is about.
struct About<'a> {
    number: u64,
    url: &'a str,
    branch: &'a str,
    head: &'a str,
    base: &'a str,
    title: Option<&'a str>,
}

/// What is said, in bare facts. `detail` is the failing checks and a log's tail.
fn said(about: &About<'_>, cause: Cause, detail: &str) -> String {
    let mut out = match cause {
        Cause::ChecksFailed => format!(
            "#{} failed its checks at {}.",
            about.number,
            short(about.head)
        ),
        Cause::Conflicts => format!("#{} conflicts with {}.", about.number, about.base),
        Cause::Dropped => format!("#{} left the merge queue.", about.number),
        Cause::Unmergeable => format!("#{} is unmergeable in the merge queue.", about.number),
        Cause::Merged => format!(
            "#{} merged into {}. finish_up releases its slot and deletes its branch; call it \
             before all_done. Never delete the branch by hand or ask the owner to: a job's \
             slot is parked when it ends, and its branch is kept.",
            about.number, about.base
        ),
        Cause::AutoMerge => format!(
            "Auto-merge is on for #{}: it merges into {} once its checks pass.",
            about.number, about.base
        ),
        Cause::Queued => format!(
            "#{} is in the merge queue: it merges into {} once its checks pass.",
            about.number, about.base
        ),
    };
    out.push_str(&format!("\n{}\nBranch: {}", about.url, about.branch));
    if !detail.is_empty() {
        out.push_str("\n\n");
        out.push_str(detail);
    }
    out
}

pub(crate) fn short(commit: &str) -> &str {
    &commit[..commit.len().min(7)]
}

/// The failing checks, one line each with its log address.
fn failing_lines(checks: &[&FollowedCheck]) -> String {
    checks
        .iter()
        .map(|check| match &check.log_url {
            Some(url) => format!("{}: {}", check.name.as_written(), url.as_written()),
            None => check.name.as_written().to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    /// Every served repository's pull requests, once an interval. Its own gate, beside main's.
    pub(crate) async fn pulls_told_when_due(self: &Arc<Self>) {
        let now = self.now();
        {
            let mut sweep = self.sweeping().lock().await;
            if let Some(last) = sweep.pulls_told_last.as_ref() {
                if elapsed(last, &now) < self.noticing().interval() {
                    return;
                }
            }
            sweep.pulls_told_last = Some(now);
        }
        for served in self.repositories().served() {
            self.pulls_told(&served).await;
        }
    }

    /// Read one repository's pull requests and tell each owner what changed. **Nothing here fails
    /// the turn**, and a forge that will not answer leaves everything as it was.
    pub(crate) async fn pulls_told(self: &Arc<Self>, served: &Served) {
        let base = served.manifest().base().unwrap_or("the base").to_string();
        let root = served.root().to_string();
        // Before the forge is asked, which may not answer: the pool is local.
        self.slot_rows_followed(served).await;
        let gate = served.manifest().ci_gate().to_string();
        let list_gate = gate.clone();
        let Some(mut followed) = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.pull_reading(root, &list_gate)
            })
            .await
        else {
            return;
        };
        // The list stops at 100: a pull request someone holds that it left out is read by number.
        let listed: Vec<u64> = followed.iter().map(|pull| pull.number).collect();
        for number in self.held_beyond(served, &listed).await {
            let gate = gate.clone();
            let one = self
                .forge_asked(&root, move |vcs: &V, root: &str| {
                    vcs.followed_pull(root, &gate, number)
                })
                .await;
            followed.extend(one);
        }
        let reading = self.pulls_reading(&root).await;
        let before = self
            .store()
            .lock()
            .await
            .pulls_seen_queued(&root)
            .unwrap_or_default();
        // **Only the person's own**: another author's pull request is nobody here's to tell.
        let mine = self.mine_in(&root).await;
        let mut queued = Vec::new();
        for pull in followed
            .iter()
            .filter(|pull| mine.owns(pull.author.as_ref().map(|it| it.as_written())))
        {
            if self.followed_told(served, &base, &before, pull).await {
                queued.push(pull.number);
            }
        }
        let _ = self.store().lock().await.keep_pulls_queued(&root, &queued);
        self.merges_told(served, &base, &mine).await;
        self.pulls_attached(served, &followed).await;
        drop(reading);
        let manifest_id = ipc::ManifestId::carried(served.manifest().id().as_str());
        self.pull_ledger_kept_current(&manifest_id, &root).await;
    }

    /// One repository's pull request reading, held while it reads and writes the queue last seen
    /// (`pulls_seen_queued`), **so a refresh and the pull reading never read it at once**.
    pub(crate) async fn pulls_reading(&self, root: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let one = Arc::clone(self.sweeping().lock().await.pull_readings.entry(root.to_string()).or_default());
        one.lock_owned().await
    }

    /// One followed pull request of the person's own: the queue's bookkeeping, then its owners
    /// told. `before` is the queue last seen. **True where it stays in the queue last seen**:
    /// queued now, or a drop not yet told to everyone, looked at again next reading.
    pub(crate) async fn followed_told(
        self: &Arc<Self>,
        served: &Served,
        base: &str,
        before: &[u64],
        pull: &FollowedPull,
    ) -> bool {
        let queued = pull.queue != PullQueue::Outside;
        // Queued again: its next drop is news, though no commit was pushed between.
        if queued && !before.contains(&pull.number) {
            let key = Cause::Dropped.key();
            let root = served.root();
            let _ = self.store().lock().await.forget_pull_notices(root, pull.number, key);
        }
        let dropped = !queued && before.contains(&pull.number);
        let everyone = self.pull_told(served, base, pull, dropped).await;
        queued || (dropped && !everyone)
    }

    /// Tell the owners of one open pull request. **False where a drop out of the queue was not
    /// told to everyone.**
    async fn pull_told(
        self: &Arc<Self>,
        served: &Served,
        base: &str,
        pull: &FollowedPull,
        dropped: bool,
    ) -> bool {
        let Some(head) = pull.head.as_deref() else {
            return true;
        };
        let (url, branch) = (pull.url.as_written(), pull.branch.as_written());
        let about = About {
            number: pull.number,
            url,
            branch,
            head,
            base,
            title: pull.title.as_ref().map(|it| it.as_written()),
        };
        // Auto-merge is told once a pull request, whatever commit it is at when it is turned on.
        // The waits are looked at on every reading: one stated after the notice, or after it
        // left the queue and was queued again, is still the owner's to be rid of.
        if pull.auto_merge || pull.queue == PullQueue::Waiting {
            let cause = if pull.auto_merge { Cause::AutoMerge } else { Cause::Queued };
            let once = About { head: "", ..about };
            let named = PullNamed { number: pull.number, branch, title: once.title };
            for owner in self.owners_of(served, pull.number, branch, false).await {
                // Waiting in the queue is nothing to say by itself: only a wait taken off speaks.
                if cause == Cause::AutoMerge {
                    let mut none = None;
                    self.told(served, &once, cause, &owner, || async { String::new() }, &mut none)
                        .await;
                }
                let Owner::Session(id) = &owner;
                let taken = self.waits_taken_off(id, &named).await;
                if taken.is_empty() {
                    continue;
                }
                let mut text = said(&once, cause, "");
                for one in taken {
                    text.push_str(&format!("\nTaken off Needs you: {one}"));
                }
                text.push_str(
                    "\nThe merge is Fleet's to follow, not the owner's to do: do not put it back in waiting_for.",
                );
                let _ = self.told_news_by_fleet(id, &text, cause.notice()).await;
            }
        }
        self.auto_land_followed(served, pull, head).await;
        let mut causes = Vec::new();
        if pull.failed() && !self.hung_started_again(served, pull, head).await {
            causes.push(Cause::ChecksFailed);
        }
        if pull.conflicting {
            causes.push(Cause::Conflicts);
        }
        if pull.queue == PullQueue::Unmergeable {
            causes.push(Cause::Unmergeable);
        }
        if dropped {
            causes.push(Cause::Dropped);
        }
        let mut everyone = true;
        for cause in causes {
            let owners = self.owners_of(served, pull.number, branch, false).await;
            let mut detail = None;
            for owner in owners {
                if self
                    .told(
                        served,
                        &about,
                        cause,
                        &owner,
                        || self.failure_detail(served, pull, head, cause),
                        &mut detail,
                    )
                    .await
                    == false
                    && cause == Cause::Dropped
                {
                    everyone = false;
                }
            }
        }
        everyone
    }

    /// Whether this failure is a job that hung and was started again, so nothing is said. **Once
    /// a commit**: a second hang is a failure.
    async fn hung_started_again(&self, served: &Served, pull: &FollowedPull, head: &str) -> bool {
        let deciding: Vec<&FollowedCheck> = pull
            .checks
            .iter()
            .filter(|check| check.required && check.state == CiState::Failed)
            .collect();
        if deciding.is_empty() || !deciding.iter().all(|check| check.hung) {
            return false;
        }
        let root = served.root().to_string();
        let kept = self
            .store()
            .lock()
            .await
            .pull_notice_kept(&root, pull.number, head, RERUN, "fleet")
            .unwrap_or(true);
        if kept {
            return false;
        }
        let number = pull.number.to_string();
        let started = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.rerun_failed(root, &number)
            })
            .await
            .is_ok();
        if started {
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(
                &root,
                pull.number,
                head,
                RERUN,
                "fleet",
                &now,
            );
        }
        started
    }

    /// The failing checks and the tail of the first few logs, for a failure, and for a drop the
    /// queue's own: its run is on a commit of its own, and can fail what the pull request's skips.
    /// Empty otherwise.
    async fn failure_detail(
        &self,
        served: &Served,
        pull: &FollowedPull,
        head: &str,
        cause: Cause,
    ) -> String {
        match cause {
            Cause::ChecksFailed => {
                let failing: Vec<&FollowedCheck> = pull
                    .checks
                    .iter()
                    .filter(|check| check.state == CiState::Failed)
                    .collect();
                let (root, commit) = (served.root().to_string(), head.to_string());
                let runs = self
                    .forge_asked(&root, move |vcs: &V, root: &str| {
                        vcs.ci_runs_on(root, &commit)
                    })
                    .await
                    .unwrap_or_default();
                self.logs_told(served, failing_lines(&failing), &failing, &runs).await
            }
            Cause::Dropped => self.queue_failure_detail(served, pull.number).await,
            _ => String::new(),
        }
    }

    /// What the queue's newest run for `number` failed. Empty where it failed nothing or the
    /// forge would not say: a drop by hand is told bare.
    async fn queue_failure_detail(&self, served: &Served, number: u64) -> String {
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return String::new();
        };
        let root = served.root().to_string();
        let Some(tested) = self
            .forge_asked(&root, move |vcs: &V, root: &str| vcs.queue_commit_of(root, &base, number))
            .await
        else {
            return String::new();
        };
        let asked = tested.clone();
        let runs = self
            .forge_asked(&root, move |vcs: &V, root: &str| vcs.ci_runs_on(root, &asked))
            .await
            .unwrap_or_default();
        let failing: Vec<FollowedCheck> = runs
            .iter()
            .filter(|run| run.state == CiState::Failed)
            .map(|run| FollowedCheck {
                name: run.name.clone(),
                state: run.state,
                required: false,
                hung: false,
                log_url: run.log_url.clone(),
            })
            .collect();
        let failing: Vec<&FollowedCheck> = failing.iter().collect();
        if failing.is_empty() {
            return String::new();
        }
        let named = format!("The merge queue failed, at {}:\n{}", short(&tested), failing_lines(&failing));
        self.logs_told(served, named, &failing, &runs).await
    }

    /// `out`, then the tail of the first few failing checks' logs, from `runs`.
    async fn logs_told(
        &self,
        served: &Served,
        mut out: String,
        failing: &[&FollowedCheck],
        runs: &[adapter_traits::CiRun],
    ) -> String {
        let root = served.root().to_string();
        for check in failing.iter().take(LOGS_READ) {
            let Some(run) = runs.iter().find(|run| {
                run.state == CiState::Failed && run.name.as_written() == check.name.as_written()
            }) else {
                continue;
            };
            let run = run.clone();
            let Some(log) = self
                .forge_asked(&root, move |vcs: &V, root: &str| vcs.ci_log(root, &run))
                .await
            else {
                continue;
            };
            let lines: Vec<&str> = log.as_written().lines().collect();
            let tail = &lines[lines.len().saturating_sub(TAIL_LINES)..];
            out.push_str(&format!(
                "\n\n{}:\n{}",
                check.name.as_written(),
                tail.join("\n")
            ));
        }
        out
    }

    /// Pull requests that merged lately, told to the Sessions and Jobs that held them.
    pub(crate) async fn merges_told(self: &Arc<Self>, served: &Served, base: &str, mine: &Mine) {
        let root = served.root().to_string();
        let merged = self
            .sweeping()
            .lock()
            .await
            .merged
            .get(&root)
            .cloned()
            .unwrap_or_default();
        let now = self.now();
        for one in merged {
            let at = core_model::Timestamp::from_rfc3339(one.pull.merged_at.as_written());
            // A task a person linked this to is asked about, whoever opened the pull request.
            self.tasks_asked_by_linked_pull(served, one.pull.number, one.pull.title.as_written()).await;
            if !mine.owns(one.pull.author.as_ref().map(|it| it.as_written())) {
                continue;
            }
            // Marks the issues of the Jobs that held it, however long ago it merged and whether or
            // not they still stand: this waits on no `bd`, and the backlog's next look closes them.
            self.issues_merged(served, one.pull.number, one.pull.url.as_written()).await;
            // The same merge asks whether the project tasks the Job held are done.
            self.tasks_asked_by_merge(served, one.pull.number).await;
            if elapsed(&at, &now) > MERGE_IS_NEWS {
                continue;
            }
            let (url, branch) = (one.pull.url.as_written(), one.pull.branch.as_written());
            let about = About {
                number: one.pull.number,
                url,
                branch,
                head: "",
                base,
                title: Some(one.pull.title.as_written()),
            };
            let owners = self.owners_of(served, one.pull.number, branch, true).await;
            for owner in owners {
                let mut none = None;
                self.told(
                    served,
                    &about,
                    Cause::Merged,
                    &owner,
                    || async { String::new() },
                    &mut none,
                )
                .await;
            }
            self.slots_told_of_merge(served, one.pull.number, branch).await;
        }
    }

    /// Tell one owner, unless it was told. **True where it is settled**: told now, told before,
    /// or nothing to be done for it. `detail` is read once for all the owners of a pull request.
    async fn told<F, Fut>(
        self: &Arc<Self>,
        served: &Served,
        about: &About<'_>,
        cause: Cause,
        owner: &Owner,
        detail: F,
        read: &mut Option<String>,
    ) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = String>,
    {
        let root = served.root().to_string();
        let (key, recipient) = (cause.key(), owner.key());
        let kept = self
            .store()
            .lock()
            .await
            .pull_notice_kept(&root, about.number, about.head, key, &recipient)
            .unwrap_or(true);
        if kept {
            return true;
        }
        if read.is_none() {
            *read = Some(detail().await);
        }
        let text = said(about, cause, read.as_deref().unwrap_or_default());
        let named = PullNamed { number: about.number, branch: about.branch, title: about.title };
        let settling = matches!(cause, Cause::Merged | Cause::AutoMerge);
        let done = match owner {
            Owner::Session(id) if settling => {
                let mut text = text.clone();
                for taken in self.waits_taken_off(id, &named).await {
                    text.push_str(&format!("\nTaken off Needs you: {taken}"));
                }
                matches!(self.told_news_by_fleet(id, &text, cause.notice()).await, Ok(true))
            }
            Owner::Session(id) => {
                matches!(self.told_by_fleet(id, &text, cause.notice()).await, Ok(true))
            }
        };
        if done {
            let now = self.now();
            let _ = self.store().lock().await.keep_pull_notice(
                &root,
                about.number,
                about.head,
                key,
                &recipient,
                &now,
            );
        }
        done
    }
}
