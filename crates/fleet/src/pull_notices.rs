//! Telling the owner of a pull request what happened to it, so nobody has to ask a session to
//! watch. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **Each notice is told once.** The key is (pull request, commit, cause, recipient) in the
//! store, so a restart reads what was already told and a failure is told again only for a commit
//! that fails anew. **A job that hung is not a failure**: one cancelled after about the whole
//! time a job is given is started again once, through the forge adapter, and nothing is said.
//!
//! A Session is sent a message from Fleet that wakes it. A Job at its review gate takes the
//! failure as a requested change, an ended one is redispatched as the red-main path does, and a
//! merge is a note in its log. **A recipient that cannot be told now is asked again next
//! reading**, with nothing kept. Who owns a pull request is `crate::pull_owners`.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CiState, Delivery, PullQueue, Vcs, WatchedCheck, WatchedPull, WorkProduct,
};
use core_model::{
    AcceptanceCriterion, Actor, Component, CriterionId, CriterionOrigin, CriterionSource, Envelope,
    Facts, FieldValue, JobId, JobStatus, Level,
};

use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::redispatch::Carrying;
use crate::repositories::Served;
use crate::resume::Redirection;

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
}

impl Cause {
    fn key(self) -> &'static str {
        match self {
            Cause::ChecksFailed => "checks_failed",
            Cause::Conflicts => "conflicts",
            Cause::Dropped => "dropped",
            Cause::Unmergeable => "unmergeable",
            Cause::Merged => "merged",
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
            "#{} merged into {}. Its slot and branch can be released.",
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

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(7)]
}

/// The failing checks, one line each with its log address.
fn failing_lines(checks: &[&WatchedCheck]) -> String {
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
        let Some(watched) = self
            .forge_asked(&root, |vcs: &V, root: &str| vcs.pull_watch(root))
            .await
        else {
            return;
        };
        let before = self
            .store()
            .lock()
            .await
            .pulls_seen_queued(&root)
            .unwrap_or_default();
        let mut queued = Vec::new();
        for pull in &watched {
            if pull.queue != PullQueue::Outside {
                queued.push(pull.number);
            }
            let dropped = pull.queue == PullQueue::Outside && before.contains(&pull.number);
            if !self.pull_told(served, &base, pull, dropped).await && dropped {
                // Not everyone could be told: look again next reading.
                queued.push(pull.number);
            }
        }
        let _ = self.store().lock().await.keep_pulls_queued(&root, &queued);
        self.merges_told(served, &base).await;
        let manifest_id = ipc::ManifestId::carried(served.manifest().id().as_str());
        self.pull_ledger_kept_current(&manifest_id, &root).await;
    }

    /// Tell the owners of one open pull request. **False where a drop out of the queue was not
    /// told to everyone.**
    async fn pull_told(
        self: &Arc<Self>,
        served: &Served,
        base: &str,
        pull: &WatchedPull,
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
        };
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
    async fn hung_started_again(&self, served: &Served, pull: &WatchedPull, head: &str) -> bool {
        let deciding: Vec<&WatchedCheck> = pull
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

    /// The failing checks and the tail of the first few logs, for a failure. Empty otherwise.
    async fn failure_detail(
        &self,
        served: &Served,
        pull: &WatchedPull,
        head: &str,
        cause: Cause,
    ) -> String {
        if cause != Cause::ChecksFailed {
            return String::new();
        }
        let failing: Vec<&WatchedCheck> = pull
            .checks
            .iter()
            .filter(|check| check.state == CiState::Failed)
            .collect();
        let mut out = failing_lines(&failing);
        let (root, commit) = (served.root().to_string(), head.to_string());
        let runs = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.ci_runs_on(root, &commit)
            })
            .await
            .unwrap_or_default();
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
    async fn merges_told(self: &Arc<Self>, served: &Served, base: &str) {
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
            };
            let mut owners = self.owners_of(served, one.pull.number, branch, true).await;
            if let Some(job) = one.job {
                if !owners.contains(&Owner::Job(job.clone())) {
                    owners.push(Owner::Job(job));
                }
            }
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
        let done = match owner {
            Owner::Session(id) => {
                matches!(self.told_by_fleet(id, &text).await, Ok(true))
            }
            Owner::Job(id) => self.job_told(served, id, about, cause, &text).await,
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

    /// Cut the replacement's worktree from the pull request's own branch and aim its pull request
    /// there, so the fix lands in the same pull request. **Only where the repository still holds
    /// the branch**: a branch that is gone leaves the Job on a fresh branch from the base.
    async fn cut_from_the_pull_request(&self, served: &Served, job: &JobId, branch: &str) {
        let held = self
            .vcs()
            .branches(served.root(), served.manifest().base())
            .is_ok_and(|held| held.iter().any(|one| one.name == branch));
        let Some(named) = core_model::branch_named(Some(branch)).filter(|_| held) else {
            return;
        };
        let landing = core_model::Landing {
            from_ref: Some(named.clone()),
            target: Some(named),
            ..core_model::Landing::as_ever()
        };
        let _ = self.store().lock().await.set_landing(job, &landing);
    }

    /// A Job's part: a merge is a note in its log, and a failure sends the work back the way a
    /// red main does. **A Job still working is left to its Drone** and asked again.
    async fn job_told(
        &self,
        served: &Served,
        id: &JobId,
        about: &About<'_>,
        cause: Cause,
        text: &str,
    ) -> bool {
        let Ok(job) = self.load(id).await else {
            return true;
        };
        if cause == Cause::Merged {
            self.logged(
                id,
                Envelope::new(
                    self.now(),
                    Level::Info,
                    Component::Fleet,
                    self.run().clone(),
                    "its pull request merged",
                )
                .in_job(id.as_ulid().clone())
                .with_field("pull_request", FieldValue::Str(about.url.to_string())),
            );
            return true;
        }
        let sent = match job.status() {
            JobStatus::AwaitingReview => match Redirection::saying(text) {
                Some(note) => self.request_changes(id, &note).await.is_ok(),
                None => true,
            },
            JobStatus::CompletedSuccess | JobStatus::CompletedFailed | JobStatus::Killed => {
                let carrying = Carrying {
                    facts: Facts::new(text.to_string()),
                    criteria: vec![AcceptanceCriterion {
                        criterion_id: CriterionId::new("c1"),
                        text: format!("#{} passes its checks and merges cleanly.", about.number),
                        source: CriterionSource::Judge,
                        origin: CriterionOrigin::Unsaid,
                    }],
                };
                match self
                    .mint_replacement(&job, Some(carrying), Actor::Fleet)
                    .await
                {
                    Ok(minted) if minted.status() == JobStatus::AwaitingApproval => {
                        self.cut_from_the_pull_request(served, minted.id(), about.branch)
                            .await;
                        self.approve(minted.id()).await.is_ok()
                    }
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            JobStatus::Rejected => true,
            _ => false,
        };
        let _ = served;
        sent
    }
}
