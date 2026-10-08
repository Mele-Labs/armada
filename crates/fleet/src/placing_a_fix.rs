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
use core_model::{FixChoice, Job, JobId, RepairRecord, TriggerState};
use verification::Exit;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repairing::holder_of;
use crate::trigger_hold::Hold;
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

/// Which fix: a Trigger's, or an added step's.
pub(crate) enum Named<'a> {
    Trigger(&'a str),
    Addition(&'a str),
}

impl Named<'_> {
    fn said(&self) -> &str {
        match self {
            Named::Trigger(which) | Named::Addition(which) => which,
        }
    }
}

const NO_FIX_WAITING: &str = "fleet.no_fix_waiting";
const FIX_NOT_PLACED: &str = "fleet.fix_not_placed";
const FIX_CONFLICTS: &str = "fleet.fix_conflicts";
const FIX_WAITING: &str = "fleet.fix_waiting";
const NO_FIX_NAMED: &str = "fleet.no_fix_named";

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
        self.choose_fix(job_id, Named::Trigger(trigger), choice)
            .await
    }

    /// [`choose_trigger_fix`](Fleet::choose_trigger_fix) for a step added to
    /// the Job, named by its id.
    pub async fn choose_addition_fix(
        &self,
        job_id: &JobId,
        addition: &str,
        choice: FixChoice,
    ) -> Result<FixChosen, FixNotChosen> {
        self.choose_fix(job_id, Named::Addition(addition), choice)
            .await
    }

    /// The latest firing of a Trigger, or the added step, whose fix waits.
    pub(crate) async fn fix_waiting_on(
        &self,
        job_id: &JobId,
        named: &Named<'_>,
    ) -> Result<Option<Hold>, FixNotChosen> {
        let store = self.store().lock().await;
        Ok(match named {
            Named::Trigger(trigger) => store
                .firings_with_ids(job_id)
                .map_err(|why| FixNotChosen::NoSuchJob(Adrift::Reading(why)))?
                .into_iter()
                .rev()
                .find(|(_, one)| one.name == *trigger && one.state == TriggerState::FixReady)
                .map(|(id, firing)| Hold::Firing { id, firing }),
            Named::Addition(addition) => store
                .job_additions(job_id)
                .map_err(|why| FixNotChosen::NoSuchJob(Adrift::Reading(why)))?
                .into_iter()
                .find(|one| {
                    one.id == *addition
                        && one
                            .fired
                            .as_ref()
                            .is_some_and(|fired| fired.state == TriggerState::FixReady)
                })
                .map(Hold::Addition),
        })
    }

    pub(crate) async fn choose_fix(
        &self,
        job_id: &JobId,
        named: Named<'_>,
        choice: FixChoice,
    ) -> Result<FixChosen, FixNotChosen> {
        let job = self.load(job_id).await.map_err(FixNotChosen::NoSuchJob)?;
        let Some(hold) = self.fix_waiting_on(job_id, &named).await? else {
            return Err(FixNotChosen::NothingWaiting {
                trigger: named.said().to_string(),
            });
        };
        let (subject, trigger) = (hold.subject(), hold.name());
        let refused = |said: String| FixNotChosen::Refused(said);
        let served = self
            .served_by(&job)
            .map_err(|why| refused(why.to_string()))?;
        let repair_branch = hold
            .record()
            .branch
            .clone()
            .ok_or_else(|| refused(String::from("the repair branch was not recorded")))?;
        // A Skill or Drone run has no Command to run again: its fix is the answer.
        let side = self.side_of(&job, &hold).await.is_some();
        let command = match self.hold_command(&job, &hold).await {
            Some(command) => Some(command),
            None if side => None,
            None => {
                return Err(refused(format!(
                    "`{trigger}` no longer names a Command to run"
                )))
            }
        };
        let pool = crate::leasing::pool_of(&served);
        let holder = holder_of(job_id, &subject);
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
        let mut record = hold.record().clone();
        record.choice = Some(choice);
        // A Drone is working where the merge would land, so the choice is kept
        // and placed once its step settles.
        if choice == FixChoice::ThisBranch && self.job_is_working(job_id).await {
            let kept = RepairRecord {
                settled_at: Some(self.now()),
                ..record
            };
            self.repair_settled(&job, &subject, TriggerState::FixReady, &kept, false)
                .await
                .map_err(refused)?;
            let _ = self.vcs().park_slot(&pool, repair_slot, &holder);
            self.repair_moved(&job, &subject).await;
            return Ok(FixChosen {
                state: TriggerState::FixReady,
                pull_request: None,
            });
        }
        let chosen = match Placing::of(choice) {
            Placing::AsAPullRequest => {
                self.fix_opened_as_a_pull_request(&job, &trigger, side, &repair_tree, &mut record)
                    .await
            }
            Placing::OntoTheJobsBranch => {
                self.fix_merged_onto_the_job(
                    &job,
                    &hold,
                    command.as_deref(),
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
            subject,
            trigger,
            step: hold.step().clone(),
            command: command.unwrap_or_default(),
            exit: hold.exit_code(),
            stdout: String::new(),
            stderr: String::new(),
            record: record.clone(),
            side: None,
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
        let said = |code: &'static str, said: String| {
            Refusal::IllegalMove(WireError::raised(code, said, self.run_id()))
        };
        // Exactly one of the two names the fix, as a hold act's do.
        if choose.trigger.is_some() == choose.addition.is_some() {
            return Err(Refusal::Unacceptable(
                WireError::raised(
                    NO_FIX_NAMED,
                    String::from(
                        "name the Trigger or the added step whose fix is waiting, one of them",
                    ),
                    self.run_id(),
                )
                .about_job(job_id.clone()),
            ));
        }
        let placed = tokio::spawn(async move {
            let choice = choose.choice.into();
            match (&choose.trigger, &choose.addition) {
                (Some(trigger), _) => fleet.choose_trigger_fix(&job, trigger, choice).await,
                (None, Some(addition)) => fleet.choose_addition_fix(&job, addition, choice).await,
                (None, None) => unreachable!("one of the two was checked"),
            }
        })
        .await;
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
    pub(crate) async fn command_of(
        &self,
        job: &Job,
        firing: &core_model::TriggerFiring,
    ) -> Option<String> {
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
        trigger: &str,
        side: bool,
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
        let review = match side {
            true => Review::assembled(
                format!("`{trigger}` on {}", job.title().as_str()),
                format!("What `{trigger}` changed on a branch cut from this Job's."),
            ),
            false => Review::assembled(
                format!("Repair `{trigger}` on {}", job.title().as_str()),
                format!("`{trigger}` failed on this Job's branch, and this makes its Command pass."),
            ),
        };
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
        hold: &Hold,
        command: Option<&str>,
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
                let holder = format!("{}-onto", holder_of(job.id(), &hold.subject()));
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
            .merged_and_run(job, hold, command, repair_tree, &job_tree, record)
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
        hold: &Hold,
        command: Option<&str>,
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
        // Merged and pushed is all a Skill or Drone run has to settle.
        let Some(command) = command else {
            return Ok(TriggerState::Passed);
        };
        let waiting = Waiting {
            job: job.id().clone(),
            subject: hold.subject(),
            trigger: hold.name(),
            step: hold.step().clone(),
            command: command.to_string(),
            exit: hold.exit_code(),
            stdout: String::new(),
            stderr: String::new(),
            record: record.clone(),
            side: None,
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
    pub(crate) async fn job_is_working(&self, job: &JobId) -> bool {
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
        let chosen: Vec<(JobId, Hold)> = {
            let store = self.store().lock().await;
            let firings = store.chosen_fixes().unwrap_or_default();
            let additions = store.chosen_addition_fixes().unwrap_or_default();
            firings
                .into_iter()
                .map(|(job, id, firing)| (job, Hold::Firing { id, firing }))
                .chain(
                    additions
                        .into_iter()
                        .map(|(job, added)| (job, Hold::Addition(added))),
                )
                .collect()
        };
        for (job_id, hold) in chosen {
            let Some(choice) = hold.record().choice else {
                continue;
            };
            if self.job_is_working(&job_id).await {
                continue;
            }
            let name = hold.name();
            let named = match &hold {
                Hold::Firing { .. } => Named::Trigger(&name),
                Hold::Addition(added) => Named::Addition(&added.id),
            };
            let Err(why) = self.choose_fix(&job_id, named, choice).await else {
                continue;
            };
            if matches!(why, FixNotChosen::Waiting(_)) {
                continue;
            }
            let record = RepairRecord {
                choice: None,
                settled_at: Some(self.now()),
                ..hold.record().clone()
            };
            let Ok(job) = self.load(&job_id).await else {
                continue;
            };
            let _ = self
                .repair_settled(
                    &job,
                    &hold.subject(),
                    TriggerState::FixReady,
                    &record,
                    false,
                )
                .await;
            self.repair_moved(&job, &hold.subject()).await;
            let said = format!(
                "the fix for `{name}` could not be placed and waits on your choice again: {why}"
            );
            self.logged(
                job.id(),
                self.trigger_line(&job, core_model::Level::Warn, &said),
            );
        }
    }
}
