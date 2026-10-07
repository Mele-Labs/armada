//! A Job's needs: what it says it needs on a file, and the order its landing
//! takes among everyone else's. `#1059`, `docs/capabilities/needs.md`.
//!
//! **A need is a row on the session ledger** (`ledger`), held by the Job as it
//! holds its slot and its branch (`ledgering`). A session that declares through
//! the intake, or a terminal through `armada need` (`served`), is a holder of the
//! same table, so a Job and a session stand in ONE order and `armada need`'s
//! answer, the peer turn and the merge refusal all read it.
//!
//! | Act | What it does to a need |
//! |---|---|
//! | `declare_scope`, `record_plan`, `add_task` carrying `needs` | Declares it, and tells this Job's Drone who is ahead |
//! | A press to merge, under `forge` and under `push` alike | Refused while a need ahead stands, naming what it waits behind, and refused where the branch changes a watched path (`adapters::undeclared`) with no need declared |
//! | The Job reaching a terminal status | Spends it where it landed, gives it back where it was dropped (`dispatch.rs`, `record`) |
//!
//! **Under `merge_by: forge` it is still Fleet that holds**, because it is
//! Fleet that asks the forge to merge. A person pressing the forge's own button
//! goes around it; nothing here sees that press. **Nothing expires by time**: a
//! need that stalls is given back by a person, `armada need --release`.

mod converting;
pub(crate) mod ledger;
mod served;

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, NotMerged, Vcs, WorkProduct};
use adapters::undeclared::undeclared;
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
    }

    /// Refuse the merge while a need ahead of this Job's stands, naming what it
    /// waits behind. **Read at the press, never frozen**, so a need given back
    /// frees the Job on its next press without anything being told.
    ///
    /// **And refused outright where the Job changes a watched path it never
    /// declared a need on**, `adapters::undeclared`: the rule `armada land`
    /// holds a session to, so a Job is held to the same one. A diff git cannot
    /// read is a line in the log and not a refusal, as a ledger that cannot be
    /// read is: neither says anything about the Job.
    pub(crate) async fn held_behind_needs(&self, job: &Job) -> Result<(), Adrift> {
        let Some(branch) = job.branch().map(|branch| branch.as_str().to_string()) else {
            return Ok(());
        };
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
        let declared: Vec<String> = all
            .iter()
            .filter(|need| need.holder == holder || need.held_by == branch)
            .map(|need| need.path.clone())
            .collect();
        if let Some(said) = self
            .took_an_undeclared_number(job, &branch, &declared)
            .await
        {
            return Err(Adrift::NotMerged {
                job: job.id().clone(),
                why: NotMerged::WaitingBehind { said },
            });
        }
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

    /// What to run, where this Job's branch changes a watched path with no need
    /// declared for it.
    async fn took_an_undeclared_number(
        &self,
        job: &Job,
        branch: &str,
        declared: &[String],
    ) -> Option<String> {
        let served = self.served_by(job).ok()?;
        let root = served.root().to_string();
        let base = self
            .vcs()
            .base_commit(&root, served.manifest().base())
            .ok()??;
        let (branch, declared) = (branch.to_string(), declared.to_vec());
        let read = tokio::task::spawn_blocking(move || {
            undeclared(Path::new(&root), &base, &branch, &declared)
        })
        .await
        .unwrap_or(Ok(None));
        match read {
            Ok(said) => said,
            Err(why) => {
                self.said_about_the_ledger(
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
        if let Err(why) = ended {
            drop(store);
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
