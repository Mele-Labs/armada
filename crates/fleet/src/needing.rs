//! A Job's needs: what it says it needs on a file, and the order its landing
//! takes among everyone else's. `#1059`,
//! `decisions/2026-10-02-a-plan-leases-its-numbers.md`.
//!
//! **The same files `armada need` writes.** A need is `adapters::needs`: one
//! JSON file per branch and path under the clone's common git directory, and a
//! Job's branch is its identity. So a Fleet Job and a session on its own branch
//! stand in one order, and `armada land` holds a branch behind a Job's need
//! exactly as Fleet's own merge holds a Job behind a session's. **Nothing about
//! a need is in the store**: the plan's tasks carry them by declaring when the
//! plan is recorded, and the file is the record.
//!
//! | Act | What it does to a need |
//! |---|---|
//! | `declare_scope`, `record_plan`, `add_task` carrying `needs` | Declares it, and tells this Job's Drone who is ahead |
//! | A press to merge, under `forge` and under `push` alike | Refused while a need ahead stands, naming what it waits behind, and refused where the branch changes a watched path (`adapters::undeclared`) with no need declared |
//! | The Job reaching a terminal status | Spends it where it landed, gives it back where it was dropped: one removal |
//!
//! **Under `merge_by: forge` it is still Fleet that holds**, because it is
//! Fleet that asks the forge to merge. A person pressing the forge's own button
//! goes around it; nothing here sees that press. **Nothing expires by time**: a
//! need that stalls is given back by a person, `armada need --release`.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, NotMerged, Vcs, WorkProduct};
use adapters::needs::{waiting_text, Need, Needs};
use adapters::undeclared::undeclared;
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};
use ipc::mcp::NeedClaim;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::peers::News;

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
    /// The needs of this Job's repository, and the branch that is its identity
    /// there. `None` for a Job with no branch yet or a repository git will not
    /// answer for, which have nothing to declare or wait behind.
    fn needs_of(&self, job: &Job) -> Option<(Needs, String)> {
        let branch = job.branch()?.as_str().to_string();
        let served = self.served_by(job).ok()?;
        let needs = Needs::of(Path::new(served.root())).ok()?;
        Some((needs, branch))
    }

    /// Declare what a Drone said its Job needs, and queue for it who is ahead.
    ///
    /// **Declaring again records nothing**, and what the Drone took is written
    /// each time it is said. **Only a first declaration is news**: the Drone
    /// choosing a value is the answer to it, not a reason to say it again.
    /// Nothing here fails the call that reached it, for
    /// [`kept_plan`](Fleet::declare_scope)'s reason: the declaration the Drone
    /// made stands and a fault is a line in the Job's log.
    pub(crate) async fn needs_declared(&self, job: &JobId, claims: &[NeedClaim]) {
        if claims.is_empty() {
            return;
        }
        let Ok(record) = self.load(job).await else {
            return;
        };
        let Some((needs, branch)) = self.needs_of(&record) else {
            self.said_about_needs(
                job,
                Level::Warn,
                "a need was declared and could not be kept: this Job has no branch yet",
                None,
            );
            return;
        };
        let claims = claims.to_vec();
        let done = tokio::task::spawn_blocking(move || {
            claims
                .iter()
                .map(|claim| declare(&needs, &branch, claim))
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        for done in done {
            match done {
                Ok((_, true, _)) => {}
                Ok((path, false, ahead)) if !ahead.is_empty() => {
                    self.owe(
                        job,
                        News::Ahead {
                            path,
                            ahead: ahead.iter().map(Need::describe).collect(),
                        },
                    )
                    .await;
                }
                Ok(_) => {}
                Err(why) => self.said_about_needs(
                    job,
                    Level::Warn,
                    "a need was declared and could not be kept",
                    Some(&why),
                ),
            }
        }
    }

    /// Refuse the merge while a need ahead of this Job's stands, naming what it
    /// waits behind. **Read at the press, never frozen**, so a need given back
    /// frees the Job on its next press without anything being told.
    ///
    /// **And refused outright where the Job changes a watched path it never
    /// declared a need on**, `adapters::undeclared`: the rule `armada land`
    /// holds a session to, so a Job is held to the same one. A diff git cannot
    /// read is a line in the log and not a refusal, as `needs_of` is.
    pub(crate) async fn held_behind_needs(&self, job: &Job) -> Result<(), Adrift> {
        let Some((needs, branch)) = self.needs_of(job) else {
            return Ok(());
        };
        if let Some(said) = self.took_an_undeclared_number(job, &needs, &branch).await {
            return Err(Adrift::NotMerged {
                job: job.id().clone(),
                why: NotMerged::WaitingBehind { said },
            });
        }
        let behind = tokio::task::spawn_blocking(move || needs.behind(&branch))
            .await
            .unwrap_or_default();
        if behind.is_empty() {
            return Ok(());
        }
        Err(Adrift::NotMerged {
            job: job.id().clone(),
            why: NotMerged::WaitingBehind {
                said: waiting_text(&behind),
            },
        })
    }

    /// What to run, where this Job's branch changes a watched path with no need
    /// declared for it.
    async fn took_an_undeclared_number(
        &self,
        job: &Job,
        needs: &Needs,
        branch: &str,
    ) -> Option<String> {
        let served = self.served_by(job).ok()?;
        let root = served.root().to_string();
        let base = self
            .vcs()
            .base_commit(&root, served.manifest().base())
            .ok()??;
        let (needs, branch) = (needs.clone(), branch.to_string());
        let read = tokio::task::spawn_blocking(move || {
            undeclared(Path::new(&root), &base, &branch, &needs)
        })
        .await
        .unwrap_or(Ok(None));
        match read {
            Ok(said) => said,
            Err(why) => {
                self.said_about_needs(
                    job.id(),
                    Level::Warn,
                    "the diff could not be read for a number taken without a need",
                    Some(&why),
                );
                None
            }
        }
    }

    /// The Job reached a terminal status: every need it held is spent, where it
    /// landed, or given back, where it was dropped. **The same removal**, so
    /// nothing here asks which.
    pub(crate) async fn needs_ended(&self, job: &Job) {
        let Some((needs, branch)) = self.needs_of(job) else {
            return;
        };
        let _ = tokio::task::spawn_blocking(move || needs.spend(&branch)).await;
    }

    fn said_about_needs(&self, job: &JobId, level: Level, said: &'static str, cause: Option<&str>) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone());
        if let Some(cause) = cause {
            envelope = envelope.with_field("cause", FieldValue::Str(cause.to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}

/// One claim, written: its path, whether the branch had declared it before, and
/// who is ahead. What the Drone took is recorded where it said.
fn declare(
    needs: &Needs,
    branch: &str,
    claim: &NeedClaim,
) -> Result<(String, bool, Vec<Need>), String> {
    let done = needs.declare(branch, &claim.path, &claim.what)?;
    if let Some(took) = &claim.took {
        needs.took(branch, &claim.path, took)?;
    }
    Ok((done.mine.path, done.already, done.ahead))
}
