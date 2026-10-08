//! Steps added to one Job: placed at approval, added to a Job underway, removed
//! before they fire, and fired at their moment. `docs/concepts/trigger.md`.
//!
//! **Beside the frozen workflow and never in it**: nothing here writes a step
//! row or moves the Job's status.
//!
//! **A Script fires through the Trigger path**: `fire_triggers` calls
//! [`Fleet::fire_additions`] at the same three moments, and the Command is
//! looked up and run as a Trigger's is. The record differs: the latest state
//! lives on the addition's own row, because an addition has no level.
//!
//! **A Skill is recorded skipped, as a skill Trigger is, and so is a Drone
//! step.** A step a Drone works needs a gate, and the frozen workflow's step
//! rows are the only one Fleet has.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use api::Refusal;
use core_model::{
    AddedKind, AddedStep, Behind, Component, Envelope, FieldValue, Fired, Job, JobId, Kept, Level,
    NotRun, OnTriggerFailure, Placed, StepId, Timestamp, TriggerResolution, TriggerSkipped,
    TriggerState, TriggerWhen,
};
use ipc::{WireError, WireValue};
use store::{Edited, NewAddition, Removal};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::side_run::Side;
use crate::trigger_repair::{Subject, Waiting};
use crate::triggering::Comes;

/// Nothing to run, or a place the workflow does not have. A 422.
const UNACCEPTABLE_ADDITION: &str = "fleet.unacceptable_addition";
/// A gap behind the current step, or a Job that is over. A 409.
const ADDED_STEP_BEHIND: &str = "fleet.added_step_behind";
/// A Job nobody has approved: the approval carries the step. A 409.
const ADDED_STEP_BEFORE_APPROVAL: &str = "fleet.added_step_before_approval";
/// A removal of one whose moment has come. A 409.
const ADDED_STEP_FIRED: &str = "fleet.added_step_fired";
/// The Job holds no such addition. A 422.
const NO_SUCH_ADDITION: &str = "fleet.no_such_addition";

/// An addition as the wire sends it, read: text that names something, in a
/// place the workflow has. Whether the place is still ahead is
/// [`Fleet::step_added`]'s question, since at the gate everything is.
pub fn new_addition(
    add: &ipc::AddStep,
    workflow: &core_model::FrozenWorkflow,
) -> Result<NewAddition, String> {
    let kind = AddedKind::from(&add.runs);
    if kind.text().trim().is_empty() {
        return Err(String::from("an added step names nothing to run"));
    }
    let when: TriggerWhen = add.when.into();
    let step = StepId::new(add.step.as_str());
    core_model::placeable(workflow, when, &step).map_err(|why| why.to_string())?;
    let on_failure = match kind {
        // A Drone already fixes its own failures.
        AddedKind::Skill { .. } | AddedKind::Drone { .. } => OnTriggerFailure {
            block: add.block,
            repair: false,
        },
        AddedKind::Script { .. } => OnTriggerFailure {
            block: add.block,
            repair: add.repair,
        },
    };
    Ok(NewAddition {
        kind,
        when,
        step,
        on_failure,
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
    /// `add_job_step`. **A gap whose moment has come is a 409
    /// `fleet.added_step_behind`**, and nothing is written.
    pub(crate) async fn step_added(
        &self,
        job_id: &JobId,
        add: &ipc::AddStep,
    ) -> Result<ipc::AddedStep, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let new = new_addition(add, job.workflow())
            .map_err(|said| self.addition_unacceptable(job_id, said))?;
        if let Some(behind) = core_model::behind(&job, new.when, &new.step) {
            return Err(self.addition_behind(job_id, &new, behind));
        }
        let at = self.now();
        let added = self
            .store()
            .lock()
            .await
            .add_job_step(job_id, &new, Placed::WhileRunning, &at)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        let said = format!(
            "Step `{}` added to this Job {}",
            added.kind.text(),
            where_it_fires(&added)
        );
        self.logged(
            job_id,
            self.addition_line(&job, at, Level::Info, &said, &added),
        );
        self.addition_moved(&job, &added, false);
        Ok(ipc::AddedStep::from(&added))
    }

    /// `edit_job_step`, **only before the step fires.** The answer is the row
    /// as it now stands.
    pub(crate) async fn step_edited(
        &self,
        job_id: &JobId,
        edit: &ipc::EditAddedStep,
    ) -> Result<ipc::AddedStep, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let edited = self
            .store()
            .lock()
            .await
            .edit_job_step(job_id, &edit.id, edit.block, edit.repair)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        match edited {
            Edited::Changed => {}
            Edited::Fired => return Err(Refusal::IllegalMove(
                WireError::raised(
                    ADDED_STEP_FIRED,
                    format!(
                        "`{}` has fired, so it is part of what this Job did and stays as it was",
                        edit.id
                    ),
                    self.run_id(),
                )
                .about_job(ipc::JobId::from(job_id))
                .with_field("id", WireValue::Str(edit.id.clone())),
            )),
            Edited::NoSuch => return Err(self.no_such_addition(job_id, &edit.id)),
        }
        let held = self
            .store()
            .lock()
            .await
            .job_additions(job_id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let added = held
            .iter()
            .find(|one| one.id == edit.id)
            .ok_or_else(|| self.no_such_addition(job_id, &edit.id))?;
        let said = format!(
            "Step `{}` now {} the Job on a failure and {} repair",
            added.kind.text(),
            if added.on_failure.block {
                "holds"
            } else {
                "does not hold"
            },
            if added.on_failure.repair {
                "gets a"
            } else {
                "gets no"
            },
        );
        self.logged(
            job_id,
            self.addition_line(&job, self.now(), Level::Info, &said, added),
        );
        self.addition_moved(&job, added, false);
        Ok(ipc::AddedStep::from(added))
    }

    /// `remove_job_step`, **only before the step fires.** The row stays.
    pub(crate) async fn step_removed(
        &self,
        job_id: &JobId,
        id: &str,
    ) -> Result<ipc::AddedStepRemoved, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let held = self
            .store()
            .lock()
            .await
            .job_additions(job_id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let at = self.now();
        let removal = self
            .store()
            .lock()
            .await
            .remove_job_step(job_id, id, &at)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        match (removal, held.iter().find(|one| one.id == id)) {
            (Removal::Removed, Some(added)) => {
                let said = format!(
                    "Step `{}` taken off this Job before it fired",
                    added.kind.text()
                );
                self.logged(
                    job_id,
                    self.addition_line(&job, at, Level::Info, &said, added),
                );
                self.addition_moved(&job, added, true);
                Ok(ipc::AddedStepRemoved { id: id.to_string() })
            }
            (Removal::Fired, _) => Err(Refusal::IllegalMove(
                WireError::raised(
                    ADDED_STEP_FIRED,
                    format!("`{id}` has fired, so it is part of what this Job did and stays"),
                    self.run_id(),
                )
                .about_job(ipc::JobId::from(job_id))
                .with_field("id", WireValue::Str(id.to_string())),
            )),
            _ => Err(self.no_such_addition(job_id, id)),
        }
    }

    /// What a `save_trigger` that keeps an addition must find: the Job holds it
    /// and it is one. **Asked before the file is written**, so a
    /// refusal leaves nothing behind.
    pub(crate) async fn addition_to_keep(
        &self,
        from: &ipc::KeptFrom,
    ) -> Result<(Job, AddedStep), Refusal> {
        let job_id = from.job_id.to_domain();
        let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
        let held = self
            .store()
            .lock()
            .await
            .job_additions(&job_id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let added = held
            .into_iter()
            .find(|one| one.id == from.addition_id)
            .ok_or_else(|| self.no_such_addition(&job_id, &from.addition_id))?;
        Ok((job, added))
    }

    /// Say where an addition was kept, once the Trigger it became is written.
    pub(crate) async fn addition_kept(
        &self,
        job: &Job,
        added: &AddedStep,
        scope: ipc::TriggerScope,
    ) {
        let kept = Kept::from(scope);
        let wrote = self
            .store()
            .lock()
            .await
            .set_addition_kept(job.id(), &added.id, kept);
        let now = self.now();
        if let Err(why) = wrote {
            let said = format!("where an added step was kept could not be recorded: {why}");
            self.logged(
                job.id(),
                self.addition_line(job, now, Level::Warn, &said, added),
            );
            return;
        }
        let said = format!(
            "Step `{}` kept for every Job, {}",
            added.kind.text(),
            kept.as_wire()
        );
        self.logged(
            job.id(),
            self.addition_line(job, now, Level::Info, &said, added),
        );
        let moved = AddedStep {
            kept: Some(kept),
            ..added.clone()
        };
        self.addition_moved(job, &moved, false);
    }

    /// The Job's added steps as the wire serves them.
    pub(crate) async fn additions_served(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<ipc::AddedStep>, Adrift> {
        let held = self
            .store()
            .lock()
            .await
            .job_additions(job_id)
            .map_err(Adrift::Reading)?;
        Ok(held.iter().map(ipc::AddedStep::from).collect())
    }

    /// Fire the Job's added steps for `when` on `step`, in its worktree, and
    /// record each. **Fails nothing**, as [`Fleet::fire_triggers`] does.
    pub(crate) async fn fire_additions(
        &self,
        job: &Job,
        when: TriggerWhen,
        step: &StepId,
        worktree: &Worktree,
    ) {
        let held = match self.store().lock().await.job_additions(job.id()) {
            Ok(held) => held,
            Err(why) => {
                let said = format!("the Job's added steps could not be read, so none ran: {why}");
                self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
                return;
            }
        };
        let Ok(served) = self.served_by(job) else {
            return;
        };
        for added in held
            .iter()
            .filter(|one| one.when == when && &one.step == step)
        {
            Box::pin(self.addition_fired(job, added, served.manifest(), worktree)).await;
        }
    }

    async fn addition_fired(
        &self,
        job: &Job,
        added: &AddedStep,
        manifest: &config::Manifest,
        worktree: &Worktree,
    ) {
        // A Skill and a Drone step run on a side Drone, as a repair does.
        if let Some(side) = Side::of_addition(&added.kind) {
            let opened = Fired::running(self.now());
            self.addition_recorded(job, added, &opened).await;
            self.logged(job.id(), self.firing_of_addition(job, added, &opened, None));
            let subject = Subject::Addition(added.id.clone());
            let name = added.kind.text().to_string();
            self.side_queued(job, subject, name, added.when, added.step.clone(), &side)
                .await;
            return;
        }
        let AddedKind::Script { command } = &added.kind else {
            return;
        };
        let asked = TriggerResolution::Command {
            name: command.clone(),
            asks_first: false,
        };
        let comes = crate::triggering::decided(&asked, manifest);
        let opened = match &comes {
            Comes::Run { .. } => Fired::running(self.now()),
            Comes::AskTheOwner { .. } => Fired::awaiting_the_owner(self.now()),
            Comes::Skip(TriggerSkipped::NotInThisRepo { command }) => Fired::skipped(
                NotRun::NotInThisRepo {
                    command: command.clone(),
                },
                self.now(),
            ),
            Comes::Skip(TriggerSkipped::ByOwner) => Fired::skipped(NotRun::ByOwner, self.now()),
            // A Command resolves to none of these.
            Comes::Skip(TriggerSkipped::SkillNotRun { .. }) | Comes::Side(_) => return,
        };
        self.addition_recorded(job, added, &opened).await;
        let Comes::Run { command } = comes else {
            self.logged(job.id(), self.firing_of_addition(job, added, &opened, None));
            return;
        };
        let budget = self.budget().duration();
        let attempt = checks_runner::run(&command, Path::new(worktree.path()), budget).await;
        let code = match &attempt.exit {
            verification::Exit::Code(code) => Some(*code),
            _ => None,
        };
        let holds =
            added.on_failure.block && core_model::can_hold(job.workflow(), added.when, &added.step);
        let ended = opened.ended(code, holds, added.on_failure.repair, self.now());
        self.addition_recorded(job, added, &ended).await;
        self.logged(
            job.id(),
            self.firing_of_addition(job, added, &ended, Some(&attempt)),
        );
        // **Queued and not waited for**, as a Trigger's is: the Job's step and
        // status are where they were, and `repair_next` puts a Drone on it.
        if ended.state == TriggerState::Repairing {
            self.trigger_repairs()
                .lock()
                .expect("not poisoned")
                .push(Waiting {
                    job: job.id().clone(),
                    subject: Subject::Addition(added.id.clone()),
                    trigger: added.kind.text().to_string(),
                    step: added.step.clone(),
                    command,
                    exit: code,
                    stdout: attempt.output.stdout.clone(),
                    stderr: attempt.output.stderr.clone(),
                    record: core_model::RepairRecord::default(),
                    side: None,
                });
        }
    }

    /// Keep one firing's state and tell whoever is watching.
    pub(crate) async fn addition_recorded(&self, job: &Job, added: &AddedStep, fired: &Fired) {
        let kept = self
            .store()
            .lock()
            .await
            .set_addition_fired(job.id(), &added.id, fired);
        if let Err(why) = kept {
            let said = format!("how an added step went could not be recorded: {why}");
            let line = self.addition_line(job, self.now(), Level::Warn, &said, added);
            self.logged(job.id(), line);
        }
        let moved = AddedStep {
            fired: Some(fired.clone()),
            ..added.clone()
        };
        self.addition_moved(job, &moved, false);
    }

    /// An added step that held its Job was let go or run again: keep how it
    /// stands, say so in the log and tell whoever is watching.
    pub(crate) async fn addition_settled(
        &self,
        job: &Job,
        added: &AddedStep,
        fired: &Fired,
        released: bool,
    ) -> Result<(), Adrift> {
        self.store()
            .lock()
            .await
            .settle_addition_hold(job.id(), &added.id, fired, released)
            .map_err(Adrift::Writing)?;
        self.logged(job.id(), self.firing_of_addition(job, added, fired, None));
        let moved = AddedStep {
            fired: Some(fired.clone()),
            ..added.clone()
        };
        self.addition_moved(job, &moved, false);
        Ok(())
    }

    /// One log line for each step placed at the approval press.
    pub(crate) fn additions_noted(&self, job: &Job, placed: &[AddedStep]) {
        for added in placed {
            let said = format!(
                "Step `{}` added to this Job {}, at approval",
                added.kind.text(),
                where_it_fires(added)
            );
            let line = self.addition_line(job, self.now(), Level::Info, &said, added);
            self.logged(job.id(), line);
        }
    }

    /// `job.addition_changed`, the row whole.
    pub(crate) fn addition_moved(&self, job: &Job, added: &AddedStep, removed: bool) {
        self.publish(ipc::Event::JobAdditionChanged(ipc::JobAdditionChanged {
            job_id: ipc::JobId::from(job.id()),
            addition: added.into(),
            removed,
            at: (&self.now()).into(),
        }));
    }

    fn addition_unacceptable(&self, job_id: &JobId, said: String) -> Refusal {
        Refusal::Unacceptable(
            WireError::raised(UNACCEPTABLE_ADDITION, said, self.run_id())
                .about_job(ipc::JobId::from(job_id)),
        )
    }

    fn no_such_addition(&self, job_id: &JobId, id: &str) -> Refusal {
        Refusal::Unacceptable(
            WireError::raised(
                NO_SUCH_ADDITION,
                format!("this Job holds no added step `{id}`"),
                self.run_id(),
            )
            .about_job(ipc::JobId::from(job_id))
            .with_field("id", WireValue::Str(id.to_string())),
        )
    }

    fn addition_behind(&self, job_id: &JobId, new: &NewAddition, behind: Behind) -> Refusal {
        let step = new.step.as_str();
        let (code, said) = match behind {
            Behind::NotApproved => (
                ADDED_STEP_BEFORE_APPROVAL,
                String::from(
                    "this Job is not approved, so a step added to it goes with the approval",
                ),
            ),
            Behind::Ended => (
                ADDED_STEP_BEHIND,
                String::from("this Job is over, so there is nothing left to add a step to"),
            ),
            Behind::Started => (
                ADDED_STEP_BEHIND,
                format!("step `{step}` has already started, so a step cannot go before it"),
            ),
            Behind::Passed => (
                ADDED_STEP_BEHIND,
                format!("the Job is past step `{step}`, so a step cannot go after it"),
            ),
        };
        Refusal::IllegalMove(
            WireError::raised(code, said, self.run_id())
                .about_job(ipc::JobId::from(job_id))
                .with_field("when", WireValue::Str(new.when.as_wire().to_string()))
                .with_field("step", WireValue::Str(step.to_string()))
                .with_field("reason", WireValue::Str(behind.as_wire().to_string())),
        )
    }

    fn addition_line(
        &self,
        job: &Job,
        at: Timestamp,
        level: Level,
        said: &str,
        added: &AddedStep,
    ) -> Envelope {
        let text = |value: &str| FieldValue::Str(value.to_string());
        Envelope::new(
            at,
            level,
            Component::Fleet,
            self.run().clone(),
            said.to_string(),
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("addition", text(&added.id))
        .with_field("kind", text(added.kind.as_wire()))
        .with_field("when", text(added.when.as_wire()))
        .with_field("step", text(added.step.as_str()))
    }

    /// The Job's log line for one firing, stamped with the firing's own end so
    /// `AddedStep.log_at` finds it. What a Script printed is evidence, never read.
    fn firing_of_addition(
        &self,
        job: &Job,
        added: &AddedStep,
        fired: &Fired,
        attempt: Option<&checks_runner::Attempt>,
    ) -> Envelope {
        let level = match fired.state {
            TriggerState::Failed | TriggerState::Repairing | TriggerState::Held => Level::Warn,
            _ => Level::Info,
        };
        let name = added.kind.text();
        let said = match (&fired.not_run, fired.state) {
            (Some(why), _) => format!("Added step `{name}` {why}"),
            (None, TriggerState::AwaitingOwner) => format!(
                "Added step `{name}` is on a destructive Command and waits on you before it runs"
            ),
            (None, state) => format!("Added step `{name}` {}", state.as_wire()),
        };
        let at = fired.ended_at.clone().unwrap_or(fired.started_at.clone());
        let mut line = self
            .addition_line(job, at, level, &said, added)
            .with_field("state", FieldValue::Str(fired.state.as_wire().to_string()));
        if let Some(code) = fired.exit_code {
            line = line.with_field("exit_code", FieldValue::Str(code.to_string()));
        }
        if let Some(attempt) = attempt {
            line = line
                .with_field("stdout", FieldValue::Str(attempt.output.stdout.clone()))
                .with_field("stderr", FieldValue::Str(attempt.output.stderr.clone()));
        }
        line
    }
}

fn where_it_fires(added: &AddedStep) -> String {
    match added.when {
        TriggerWhen::StepStarts => format!("before `{}`", added.step.as_str()),
        TriggerWhen::StepPasses => format!("after `{}`", added.step.as_str()),
        TriggerWhen::PrOpened => String::from("after the pull request opens"),
    }
}
