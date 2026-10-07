//! The owner's choice of where a failed Trigger's held fix goes, and the
//! delivery each choice makes. `crate::trigger_repair::Delivery` decides which;
//! this does it. **Fleet never chooses**: the firing stays `fix_ready` until
//! `choose_trigger_fix` is called, and stays there when the choice cannot be
//! carried out, so he can make another.

use std::path::Path;

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
}

impl std::fmt::Display for FixNotChosen {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixNotChosen::NoSuchJob(why) => write!(out, "{why}"),
            FixNotChosen::NothingWaiting { trigger } => {
                write!(out, "no fix for `{trigger}` is waiting on a choice")
            }
            FixNotChosen::Refused(said) => write!(out, "{said}"),
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
                    return Err(refused(String::from(
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
        let state = chosen.map_err(refused)?;
        let waiting = Waiting {
            job: job_id.clone(),
            firing: firing_id,
            trigger: trigger.to_string(),
            step: firing.step.clone(),
            command,
            exit: firing.exit_code,
            stdout: String::new(),
            stderr: String::new(),
        };
        self.repair_ended(&job, &waiting, state, &record, "the fix is placed")
            .await;
        Ok(FixChosen {
            state,
            pull_request: record.pull_request,
        })
    }

    /// The command line the firing's Trigger runs now: asked of the frozen set
    /// and the Job's Manifest again, as a firing is.
    async fn command_of(&self, job: &Job, firing: &TriggerFiring) -> Option<String> {
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
    ) -> Result<TriggerState, String> {
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
            Opened::NothingPushed => Err(String::from(
                "nothing reached a remote to open a pull request from",
            )),
            Opened::NoTool { why } => Err(format!(
                "nothing on this machine can open a pull request: {why}"
            )),
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
    ) -> Result<TriggerState, String> {
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
                    SlotLeased::Full => return Err(String::from(
                        "every worktree slot is in use, so the Job's branch could not be reached",
                    )),
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
    ) -> Result<TriggerState, String> {
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
                return Err(format!(
                    "the Job's branch moved and the fix conflicts in {}, so nothing was merged",
                    files.join(", ")
                ))
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
}
