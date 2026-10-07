//! The owner's choice of where a failed Trigger's held fix goes, and the
//! delivery each choice makes. `crate::trigger_repair::Delivery` decides which;
//! this does it. **Fleet never chooses**: the firing stays `fix_ready` until
//! `choose_trigger_fix` is called, and stays there when the choice cannot be
//! carried out, so he can make another.
//!
//! **A choice that has to wait, waits.** `this_branch` while a Drone is working
//! on the Job's branch is kept on the firing and placed when that step
//! settles, by [`chosen_fixes_retried`](Fleet::chosen_fixes_retried). A full
//! pool is waited out the same way. A merge that conflicts is not: the choice
//! is cleared and the fix is back with the owner, who is told.

use std::path::Path;

use api::Refusal;
use ipc::WireError;

use adapter_traits::{
    AgentHarness, BranchMerged, Delivery, Opened, Review, SlotLeased, Vcs, WorkProduct, Worktree,
};
use core_model::{FixChoice, Job, JobId, RepairRecord, TriggerFiring, TriggerState};
use verification::Exit;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repairing::holder_of;
use crate::trigger_repair::{Delivery as Placing, Waiting};

/// A fix the owner chose a place for that Fleet could not put there. The firing
/// is still `fix_ready`, so he can choose again.
#[derive(Debug)]
pub enum FixNotChosen {
    NoSuchJob(Adrift),
    /// No firing of this Trigger holds a fix.
    NothingWaiting {
        trigger: String,
    },
    /// Said in the words a person reads.
    Refused(String),
    /// Every slot is held. The fix stays held and a later try reaches it.
    Waiting(String),
    /// The fix does not merge onto the Job's branch as it now stands.
    Conflicts(String),
}

impl From<String> for FixNotChosen {
    fn from(said: String) -> FixNotChosen {
        FixNotChosen::Refused(said)
    }
}

impl std::fmt::Display for FixNotChosen {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixNotChosen::NoSuchJob(why) => write!(out, "{why}"),
            FixNotChosen::NothingWaiting { trigger } => {
                write!(out, "no fix for `{trigger}` is waiting on a choice")
            }
            FixNotChosen::Refused(said)
            | FixNotChosen::Waiting(said)
            | FixNotChosen::Conflicts(said) => write!(out, "{said}"),
        }
    }
}

impl std::error::Error for FixNotChosen {}

/// What choosing came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixChosen {
    pub state: TriggerState,
    /// The pull request opened, for [`FixChoice::NewPr`].
    pub pull_request: Option<String>,
}

const NO_FIX_WAITING: &str = "fleet.no_fix_waiting";
const FIX_NOT_PLACED: &str = "fleet.fix_not_placed";
const FIX_CONFLICTS: &str = "fleet.fix_conflicts";
const FIX_WAITING: &str = "fleet.fix_waiting";

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
    /// The owner's choice of where a held fix goes. **Fleet never chooses**:
    /// the firing stays `fix_ready` until this is called, and stays there if
    /// the choice cannot be carried out, so he can make another.
    pub async fn choose_trigger_fix(
        &self,
        job_id: &JobId,
        trigger: &str,
        choice: FixChoice,
    ) -> Result<FixChosen, FixNotChosen> {
        let job = self.load(job_id).await.map_err(FixNotChosen::NoSuchJob)?;
        let firings = self
            .store()
            .lock()
            .await
            .firings_with_ids(job_id)
            .map_err(|why| FixNotChosen::NoSuchJob(Adrift::Reading(why)))?;
        let Some((firing_id, firing)) = firings
            .into_iter()
            .rev()
            .find(|(_, one)| one.name == trigger && one.state == TriggerState::FixReady)
        else {
            return Err(FixNotChosen::NothingWaiting {
                trigger: trigger.to_string(),
            });
        };
        let refused = |said: String| FixNotChosen::Refused(said);
        let served = self
            .served_by(&job)
            .map_err(|why| refused(why.to_string()))?;
        let repair_branch = firing
            .repair
            .branch
            .clone()
            .ok_or_else(|| refused(String::from("the repair branch was not recorded")))?;
        let command = self
            .command_of(&job, &firing)
            .await
            .ok_or_else(|| refused(format!("`{trigger}` no longer names a Command to run")))?;
        let pool = crate::leasing::pool_of(&served);
        let holder = holder_of(job_id, firing_id);
        let (repair_slot, repair_tree) =
            match self
                .vcs()
                .lease_existing_slot(&pool, &repair_branch, &holder)
            {
                Ok(SlotLeased::Took { slot, worktree, .. }) => {
                    (slot, self.based(&served, worktree))
                }
                Ok(SlotLeased::Full) => {
                    return Err(FixNotChosen::Waiting(String::from(
                        "every worktree slot is in use, so the fix could not be reached yet",
                    )))
                }
                Err(why) => {
                    return Err(refused(format!(
                        "the repair branch could not be reached: {why}"
                    )))
                }
            };
        let mut record = firing.repair.clone();
        record.choice = Some(choice);
        // A Drone is working where the merge would land, so the choice is kept
        // and placed once its step settles.
        if choice == FixChoice::ThisBranch && self.job_is_working(job_id).await {
            let kept = RepairRecord {
                settled_at: Some(self.now()),
                ..record
            };
            self.store()
                .lock()
                .await
                .settle_repair(firing_id, TriggerState::FixReady, &kept, None)
                .map_err(|why| refused(why.to_string()))?;
            let _ = self.vcs().park_slot(&pool, repair_slot, &holder);
            self.repair_moved(&job, firing_id).await;
            return Ok(FixChosen {
                state: TriggerState::FixReady,
                pull_request: None,
            });
        }
        let chosen = match Placing::of(choice) {
            Placing::AsAPullRequest => {
                self.fix_opened_as_a_pull_request(&job, &firing, &repair_tree, &mut record)
                    .await
            }
            Placing::OntoTheJobsBranch => {
                self.fix_merged_onto_the_job(
                    &job,
                    firing_id,
                    &firing,
                    &command,
                    &repair_tree,
                    &pool,
                    &mut record,
                )
                .await
            }
        };
        // Whichever way it went, the repair slot is given back: the fix is on
        // its branch, and a refusal leaves it `fix_ready` to be reached again.
        let _ = self.vcs().park_slot(&pool, repair_slot, &holder);
        let state = chosen?;
        let waiting = Waiting {
            job: job_id.clone(),
            firing: firing_id,
            trigger: trigger.to_string(),
            step: firing.step.clone(),
            command,
            exit: firing.exit_code,
            stdout: String::new(),
            stderr: String::new(),
            record: record.clone(),
        };
        self.repair_ended(&job, &waiting, state, &record, "the fix is placed")
            .await;
        Ok(FixChosen {
            state,
            pull_request: record.pull_request,
        })
    }

    /// [`choose_trigger_fix`](Fleet::choose_trigger_fix) as the wire asks for
    /// it. **Spawned and awaited**, as `show_again` is, so a client that stops
    /// waiting leaves a merge it began to finish.
    pub(crate) async fn fix_chosen(
        self: std::sync::Arc<Self>,
        job_id: ipc::JobId,
        choose: ipc::ChooseTriggerFix,
    ) -> Result<ipc::TriggerFixChosen, Refusal> {
        let fleet = std::sync::Arc::clone(&self);
        let job = job_id.to_domain();
        let placed = tokio::spawn(async move {
            fleet
                .choose_trigger_fix(&job, &choose.trigger, choose.choice.into())
                .await
        })
        .await;
        let said = |code: &'static str, said: String| {
            Refusal::IllegalMove(WireError::raised(code, said, self.run_id()))
        };
        match placed {
            Err(why) => Err(Refusal::Fault(WireError::raised(
                "fleet.fix_not_placed",
                format!("placing the fix stopped: {why}"),
                self.run_id(),
            ))),
            Ok(Ok(chosen)) => Ok(ipc::TriggerFixChosen {
                state: chosen.state.into(),
                pull_request: chosen
                    .pull_request
                    .as_deref()
                    .map(ipc::TriggerPullRequest::at),
            }),
            Ok(Err(FixNotChosen::NoSuchJob(why))) => Err(self.refusal(why)),
            Ok(Err(why @ FixNotChosen::NothingWaiting { .. })) => {
                Err(said(NO_FIX_WAITING, why.to_string()))
            }
            Ok(Err(why @ FixNotChosen::Refused(_))) => Err(said(FIX_NOT_PLACED, why.to_string())),
            Ok(Err(why @ FixNotChosen::Waiting(_))) => Err(said(FIX_WAITING, why.to_string())),
            Ok(Err(why @ FixNotChosen::Conflicts(_))) => Err(said(FIX_CONFLICTS, why.to_string())),
        }
    }

    /// The command line the firing's Trigger runs now: asked of the frozen set
    /// and the Job's Manifest again, as a firing is.
    pub(crate) async fn command_of(&self, job: &Job, firing: &TriggerFiring) -> Option<String> {
        let frozen = self.store().lock().await.frozen_triggers(job.id()).ok()?;
        let served = self.served_by(job).ok()?;
        let (manifest, _) = self.effective_manifest_in(&served, job).await;
        crate::triggering::plan(&frozen, firing.when, &firing.step, &manifest)
            .into_iter()
            .find(|one| one.trigger().name == firing.name)
            .and_then(|one| one.to_run().map(str::to_string))
    }

    /// Push the repair branch and open a pull request from it against the Job's
    /// target. The Command passed on it before the fix was held.
    async fn fix_opened_as_a_pull_request(
        &self,
        job: &Job,
        firing: &TriggerFiring,
        repair_tree: &Worktree,
        record: &mut RepairRecord,
    ) -> Result<TriggerState, FixNotChosen> {
        let base = self
            .the_base(job.id(), repair_tree)
            .await
            .map_err(|why| why.to_string())?
            .ok_or_else(|| {
                String::from("the repository names no base to open a pull request against")
            })?;
        self.vcs()
            .push(repair_tree)
            .map_err(|why| format!("the repair branch did not go out: {}", why.said))?;
        let review = Review::assembled(
            format!("Repair `{}` on {}", firing.name, job.title().as_str()),
            format!(
                "The Trigger `{}` failed on this Job's branch, and this makes its Command pass.",
                firing.name
            ),
        );
        match self
            .vcs()
            .open_for_review(repair_tree, &base, &review)
            .map_err(|why| format!("the pull request was not opened: {}", why.said))?
        {
            Opened::PullRequest { url } | Opened::AlreadyOpen { url } => {
                record.pull_request = Some(url);
                Ok(TriggerState::Passed)
            }
            Opened::NothingPushed => Err(FixNotChosen::Refused(String::from(
                "nothing reached a remote to open a pull request from",
            ))),
            Opened::NoTool { why } => Err(FixNotChosen::Refused(format!(
                "nothing on this machine can open a pull request: {why}"
            ))),
        }
    }

    /// Merge the repair branch onto the Job's, push it, and run the Command on
    /// the Job's branch, which is what settles the Trigger.
    #[allow(clippy::too_many_arguments)]
    async fn fix_merged_onto_the_job(
        &self,
        job: &Job,
        firing_id: i64,
        firing: &TriggerFiring,
        command: &str,
        repair_tree: &Worktree,
        pool: &adapter_traits::SlotPool,
        record: &mut RepairRecord,
    ) -> Result<TriggerState, FixNotChosen> {
        let (job_tree, leased) = match self.worktree_of(job).map_err(|why| why.to_string())? {
            Some(held) => (held, None),
            None => {
                let from = job
                    .branch()
                    .ok_or_else(|| String::from("the Job has no branch"))?;
                let holder = format!("{}-onto", holder_of(job.id(), firing_id));
                let served = self.served_by(job).map_err(|why| why.to_string())?;
                match self
                    .vcs()
                    .lease_existing_slot(pool, from.as_str(), &holder)
                    .map_err(|why| format!("the Job's branch could not be reached: {why}"))?
                {
                    SlotLeased::Took { slot, worktree, .. } => {
                        (self.based(&served, worktree), Some((slot, holder)))
                    }
                    SlotLeased::Full => return Err(FixNotChosen::Waiting(String::from(
                        "every worktree slot is in use, so the Job's branch could not be reached",
                    ))),
                }
            }
        };
        let placed = self
            .merged_and_run(
                job,
                firing_id,
                firing,
                command,
                repair_tree,
                &job_tree,
                record,
            )
            .await;
        if let Some((slot, holder)) = leased {
            let _ = self.vcs().park_slot(pool, slot, &holder);
        }
        placed
    }

    #[allow(clippy::too_many_arguments)]
    async fn merged_and_run(
        &self,
        job: &Job,
        firing_id: i64,
        firing: &TriggerFiring,
        command: &str,
        repair_tree: &Worktree,
        job_tree: &Worktree,
        record: &mut RepairRecord,
    ) -> Result<TriggerState, FixNotChosen> {
        let branch = record
            .branch
            .clone()
            .ok_or_else(|| String::from("the repair branch was not recorded"))?;
        match self
            .vcs()
            .merge_branch(job_tree, &branch)
            .map_err(|why| format!("the fix could not be merged: {}", why.said))?
        {
            BranchMerged::Merged => {}
            BranchMerged::PutBack { files } => {
                return Err(FixNotChosen::Conflicts(format!(
                    "the Job's branch moved and the fix conflicts in {}, so nothing was merged",
                    files.join(", ")
                )))
            }
        }
        self.vcs()
            .push(job_tree)
            .map_err(|why| format!("the Job's branch did not go out: {}", why.said))?;
        let _ = repair_tree;
        let waiting = Waiting {
            job: job.id().clone(),
            firing: firing_id,
            trigger: firing.name.clone(),
            step: firing.step.clone(),
            command: command.to_string(),
            exit: firing.exit_code,
            stdout: String::new(),
            stderr: String::new(),
            record: record.clone(),
        };
        self.repair_kept(job, &waiting, TriggerState::Rerunning, record, false)
            .await;
        let attempt = checks_runner::run(
            command,
            Path::new(job_tree.path()),
            self.budget().duration(),
        )
        .await;
        Ok(match attempt.exit {
            Exit::Code(0) => TriggerState::Passed,
            _ => TriggerState::Failed,
        })
    }

    /// Whether a Drone is working on this Job now, which is whether a merge
    /// onto its branch would land under it.
    async fn job_is_working(&self, job: &JobId) -> bool {
        match self.slot_of(job).await {
            Some(slot) => slot.lock().await.as_ref().is_some_and(|at| at.is(job)),
            None => false,
        }
    }

    /// Place the fixes the owner chose that had to wait, now their Job has no
    /// Drone working. A pool still full waits again. A fix that no longer
    /// merges has its choice cleared and goes back to him, listed on the Job's
    /// alerts with the reason in the Job's log.
    pub async fn chosen_fixes_retried(&self) {
        let chosen = self.store().lock().await.chosen_fixes().unwrap_or_default();
        for (job_id, firing_id, firing) in chosen {
            let Some(choice) = firing.repair.choice else {
                continue;
            };
            if self.job_is_working(&job_id).await {
                continue;
            }
            let Err(why) = self.choose_trigger_fix(&job_id, &firing.name, choice).await else {
                continue;
            };
            if matches!(why, FixNotChosen::Waiting(_)) {
                continue;
            }
            let record = RepairRecord {
                choice: None,
                settled_at: Some(self.now()),
                ..firing.repair.clone()
            };
            let _ = self.store().lock().await.settle_repair(
                firing_id,
                TriggerState::FixReady,
                &record,
                None,
            );
            if let Ok(job) = self.load(&job_id).await {
                self.repair_moved(&job, firing_id).await;
                let said = format!(
                    "the fix for `{}` could not be placed and waits on your choice again: {why}",
                    firing.name
                );
                self.logged(
                    job.id(),
                    self.trigger_line(&job, core_model::Level::Warn, &said),
                );
            }
        }
    }
}
