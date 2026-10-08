//! A Job's needs: what it says it needs on a file, and the order its landing
//! takes among everyone else's. `#1059`, `docs/capabilities/needs.md`.
//!
//! **A need is a row on the session ledger** (`ledger`), held by the Job as it
//! holds its slot and its branch (`ledgering`). A session that declares through
//! the intake and a terminal's `armada need` (`served`) hold rows of the same
//! table, so all three stand in ONE order, and the peer turn, the merge refusal
//! and `armada need`'s answer read it. `declare_scope`, `record_plan` and
//! `add_task` declare; a merge is refused while a need ahead stands; `record` spends or gives back at a terminal status.
//! **Under `forge` it is still Fleet that holds**, since Fleet asks the forge to
//! merge. **Nothing expires by time**: a person gives a stalled need back.

mod converting;
pub(crate) mod ledger;
mod served;
pub(crate) mod status;

use adapter_traits::{AgentHarness, Delivery, NotMerged, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, JobStatus, Level};
use ipc::mcp::NeedClaim;
use store::{AttachmentState, Holder};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::peers::News;
use ledger::{behind, standing, waiting_text, Need};

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
        let Some(branch) = record.branch().map(|branch| branch.as_str().to_string()) else {
            self.said_about_the_ledger(
                job,
                Level::Warn,
                "a need was declared and could not be kept: this Job has no branch yet",
                None,
            );
            return;
        };
        let manifest = record.owner_manifest_id().as_str().to_string();
        let holder = Holder::job(job.as_str());
        let now = self.now().as_str().to_string();
        let done: Vec<_> = {
            let mut store = self.store().lock().await;
            claims
                .iter()
                .map(|claim| {
                    declare(&mut store, &holder, &manifest, &branch, claim, &now)
                        .map_err(|why| why.to_string())
                })
                .collect()
        };
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
                Err(why) => self.said_about_the_ledger(
                    job,
                    Level::Warn,
                    "a need was declared and could not be kept",
                    Some(&why),
                ),
            }
        }
        self.needs_status_republished_for(job).await;
    }

    /// Refuse the merge while a need ahead of this Job's stands, naming what it
    /// waits behind. **Read at the press, never frozen**, so a need given back
    /// frees the Job on its next press without anything being told.
    pub(crate) async fn held_behind_needs(&self, job: &Job) -> Result<(), Adrift> {
        if job.branch().is_none() {
            return Ok(());
        }
        let Ok(served) = self.served_by(job) else {
            return Ok(());
        };
        let manifest = job.owner_manifest_id().as_str().to_string();
        self.gone_branches_given_back(&served).await;
        let holder = Holder::job(job.id().as_str());
        let all = {
            let store = self.store().lock().await;
            match standing(&store, &manifest, None) {
                Ok(all) => all,
                Err(why) => {
                    self.said_about_the_ledger(
                        job.id(),
                        Level::Warn,
                        "the needs could not be read, so none held this merge",
                        Some(&why.to_string()),
                    );
                    return Ok(());
                }
            }
        };
        let behind = behind(&all, &holder);
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

    /// The Job reached a terminal status: every need it held is spent, where it
    /// landed, or given back, where it was dropped. **The same call for every
    /// terminal status**, `dispatch.rs`' `record`, so nothing waits behind a Job
    /// that is over.
    pub(crate) async fn needs_ended(&self, job: &Job) {
        let state = if job.status() == JobStatus::CompletedSuccess {
            AttachmentState::Spent
        } else {
            AttachmentState::GivenBack
        };
        let holder = Holder::job(job.id().as_str());
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let ended = ledger::ended(&mut store, &holder, state, &now)
            .and_then(|_| store.settle_held(&holder, Some("branch"), state, &now));
        drop(store);
        self.needs_status_republished_for(job.id()).await;
        if let Err(why) = ended {
            self.said_about_the_ledger(
                job.id(),
                Level::Warn,
                "the Job ended and what it held on the ledger could not be settled",
                Some(&why.to_string()),
            );
        }
    }

    pub(crate) fn said_about_the_ledger(
        &self,
        job: &JobId,
        level: Level,
        said: &'static str,
        cause: Option<&str>,
    ) {
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

/// One claim, written: its path, whether the Job had declared it before, and
/// who is ahead. What the Drone took is recorded where it said.
fn declare(
    store: &mut store::Store,
    holder: &Holder,
    manifest: &str,
    branch: &str,
    claim: &NeedClaim,
    now: &str,
) -> Result<(String, bool, Vec<Need>), store::WriteError> {
    let done = ledger::declare(
        store,
        holder,
        manifest,
        branch,
        &claim.path,
        &claim.what,
        now,
    )?;
    if let Some(took) = &claim.took {
        ledger::took(store, holder, manifest, &claim.path, took, now)?;
    }
    Ok((done.mine.path, done.already, done.ahead))
}
