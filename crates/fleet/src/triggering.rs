//! What a Job's Triggers come to at each moment, and where Fleet fires them.
//! `docs/concepts/trigger.md`.
//!
//! Frozen once at approval by [`freeze`], decided at each moment by [`plan`], and
//! recorded and never obeyed: a firing's exit reaches no gate, step or Job.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use config::Manifest;
use core_model::{
    Component, Envelope, FieldValue, FrozenTrigger, FrozenWorkflow, Job, Level, StepId, Timestamp,
    TriggerFiring, TriggerResolution, TriggerSkipped, TriggerState, TriggerWhen, WorkflowId,
};
use verification::Exit;

use crate::daemon::Fleet;
use crate::side_run::Side;
use crate::trigger_repair::{Subject, Waiting};

/// Every Trigger of `resolved` that applies to this workflow, bound to each
/// step it fires on. A `pr_opened` one is bound to the delivering step, and a
/// workflow with none freezes none.
pub fn freeze(
    resolved: &config::ResolvedTriggers,
    workflow: &WorkflowId,
    frozen: &FrozenWorkflow,
) -> Vec<FrozenTrigger> {
    let bound = |when, step: &StepId| {
        resolved
            .applying(workflow, when, step)
            .map(|one| FrozenTrigger {
                name: one.trigger().name().to_string(),
                when,
                step: step.clone(),
                source: one.source(),
                resolution: one.resolution().clone(),
                on_failure: one.trigger().on_failure(),
            })
            .collect::<Vec<_>>()
    };
    let mut out = Vec::new();
    for when in [TriggerWhen::StepStarts, TriggerWhen::StepPasses] {
        for step in frozen.steps() {
            out.extend(bound(when, step.id()));
        }
    }
    if let Some(delivering) = frozen.delivering_step() {
        out.extend(bound(TriggerWhen::PrOpened, delivering.id()));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Comes {
    Run { command: String },
    Skip(TriggerSkipped),
    AskTheOwner,
    /// A skill, which a side Drone runs on a branch of its own.
    Side(Side),
}

/// One frozen Trigger, decided: to run, skipped, or held for the owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    trigger: FrozenTrigger,
    comes: Comes,
}

/// The frozen Triggers for `when` on `step`, each decided against `manifest`.
///
/// The Command is looked up again here, so one the repository stopped declaring
/// since the approval is skipped, and one that became `destructive` is held.
pub fn plan(
    frozen: &[FrozenTrigger],
    when: TriggerWhen,
    step: &StepId,
    manifest: &Manifest,
) -> Vec<Planned> {
    frozen
        .iter()
        .filter(|one| one.when == when && &one.step == step)
        .map(|one| Planned {
            trigger: one.clone(),
            comes: decided(&one.resolution, manifest),
        })
        .collect()
}

pub(crate) fn decided(resolution: &TriggerResolution, manifest: &Manifest) -> Comes {
    match resolution {
        TriggerResolution::Skipped(why) => Comes::Skip(why.clone()),
        TriggerResolution::Skill { name } => Comes::Side(Side::Skill(name.clone())),
        TriggerResolution::Command { name, asks_first } => match manifest.command(name) {
            None => Comes::Skip(TriggerSkipped::NotInThisRepo {
                command: name.clone(),
            }),
            Some(command) if *asks_first || command.is_destructive() => Comes::AskTheOwner,
            Some(command) => Comes::Run {
                command: command.run().to_string(),
            },
        },
    }
}

impl Planned {
    pub fn trigger(&self) -> &FrozenTrigger {
        &self.trigger
    }

    /// This Trigger with `block` off, where nothing is left to hold: it fires
    /// and fails as a Trigger that does not block does.
    pub(crate) fn cannot_hold(mut self) -> Planned {
        self.trigger.on_failure.block = false;
        self
    }

    /// The command line to run, or `None` where nothing is to run.
    pub fn to_run(&self) -> Option<&str> {
        match &self.comes {
            Comes::Run { command } => Some(command),
            Comes::Skip(_) | Comes::AskTheOwner | Comes::Side(_) => None,
        }
    }

    /// The record as it opens: running, already skipped, or held for the owner.
    pub fn opened(&self, at: Timestamp) -> TriggerFiring {
        match &self.comes {
            Comes::Run { .. } | Comes::Side(_) => TriggerFiring::running(&self.trigger, at),
            Comes::Skip(why) => TriggerFiring::skipped(&self.trigger, why.clone(), at),
            Comes::AskTheOwner => TriggerFiring::awaiting_the_owner(&self.trigger, at),
        }
    }

    /// The opened record, ended. Zero is the only pass; a signal, a timeout and
    /// a program that never started are failures with no code.
    pub fn ended(&self, opened: TriggerFiring, exit: &Exit, at: Timestamp) -> TriggerFiring {
        let code = match exit {
            Exit::Code(code) => Some(*code),
            Exit::Signalled { .. } | Exit::TimedOut { .. } | Exit::NeverRan(_) => None,
        };
        opened.ended(code, at)
    }
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
    /// Freeze the Triggers this Job runs onto it, replacing any held.
    ///
    /// Held, never raised: a Trigger that cannot be frozen does not run, and
    /// refusing an approval for it would put a person's act behind a
    /// convenience. The Job's log says what went wrong.
    pub(crate) async fn freeze_triggers(&self, job: &Job) {
        let Ok(served) = self.served_by(job) else {
            return;
        };
        let written = self
            .locating()
            .triggers(Path::new(served.root()), served.manifest().base());
        let resolved = config::TriggerCatalogue::of(written).resolve(served.manifest());
        for left_out in resolved.left_out() {
            self.logged(
                job.id(),
                self.trigger_line(job, Level::Warn, &left_out.to_string()),
            );
        }
        let frozen = freeze(&resolved, job.workflow_id(), job.workflow());
        let kept = self.store().lock().await.freeze_triggers(job.id(), &frozen);
        if let Err(why) = kept {
            let said = format!("the Job's Triggers could not be kept, so none will run: {why}");
            self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
        }
    }

    /// Fire what runs at this moment: the Job's frozen Triggers, then the steps
    /// added to it. **Fails nothing**: every refusal in here is a line in the
    /// Job's log and the Job carries on.
    pub(crate) async fn fire_triggers(
        &self,
        job: &Job,
        when: TriggerWhen,
        step: &StepId,
        worktree: &Worktree,
    ) {
        // A hold at this step's start that was let go is not fired again when
        // the Job goes on to it: that would undo a skip, and rerun a pass.
        if when == TriggerWhen::StepStarts {
            let passed = self
                .store()
                .lock()
                .await
                .take_released_hold(job.id(), when, step);
            if passed.unwrap_or(false) {
                return;
            }
        }
        Box::pin(self.fire_frozen_triggers(job, when, step, worktree)).await;
        Box::pin(self.fire_additions(job, when, step, worktree)).await;
    }

    async fn fire_frozen_triggers(
        &self,
        job: &Job,
        when: TriggerWhen,
        step: &StepId,
        worktree: &Worktree,
    ) {
        let frozen = match self.store().lock().await.frozen_triggers(job.id()) {
            Ok(frozen) => frozen,
            Err(why) => {
                let said = format!("the Job's Triggers could not be read, so none ran: {why}");
                self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
                return;
            }
        };
        let Ok(served) = self.served_by(job) else {
            return;
        };
        let holds = core_model::can_hold(job.workflow(), when, step);
        for one in plan(&frozen, when, step, served.manifest()) {
            let one = if holds { one } else { one.cannot_hold() };
            Box::pin(self.fired(job, &one, worktree)).await;
        }
    }

    async fn fired(&self, job: &Job, one: &Planned, worktree: &Worktree) {
        let opened = one.opened(self.now());
        let kept = self.store().lock().await.open_firing(job.id(), &opened);
        let firing = match kept {
            Ok(id) => Some(id),
            Err(why) => {
                let said = format!("a Trigger's firing could not be recorded: {why}");
                self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
                None
            }
        };
        self.trigger_moved(job, &opened);
        if let Comes::Side(side) = &one.comes {
            self.logged(job.id(), self.firing_line(job, &opened, None));
            if let Some(firing) = firing {
                let (name, step) = (opened.name.clone(), opened.step.clone());
                let subject = Subject::Firing(firing);
                self.side_queued(job, subject, name, opened.when, step, side)
                    .await;
            }
            return;
        }
        let Some(command) = one.to_run() else {
            self.logged(job.id(), self.firing_line(job, &opened, None));
            return;
        };
        let budget = self.budget().duration();
        let attempt = checks_runner::run(command, Path::new(worktree.path()), budget).await;
        let ended = one.ended(opened, &attempt.exit, self.now());
        if let Some(id) = firing {
            if let Err(why) = self.store().lock().await.settle_firing(id, &ended) {
                let said = format!("a Trigger's ending could not be recorded: {why}");
                self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
            }
        }
        self.logged(job.id(), self.firing_line(job, &ended, Some(&attempt)));
        self.trigger_moved(job, &ended);
        // **Queued and not waited for**: the Job's step and status are where
        // they were, and a repair Drone is put on by `repair_next`.
        if let (TriggerState::Repairing, Some(firing), Some(command)) =
            (ended.state, firing, one.to_run())
        {
            self.trigger_repairs()
                .lock()
                .expect("not poisoned")
                .push(Waiting {
                    job: job.id().clone(),
                    subject: Subject::Firing(firing),
                    trigger: ended.name.clone(),
                    step: ended.step.clone(),
                    command: command.to_string(),
                    exit: ended.exit_code,
                    stdout: attempt.output.stdout.clone(),
                    stderr: attempt.output.stderr.clone(),
                    record: core_model::RepairRecord::default(),
                    side: None,
                });
        }
    }

    /// `job.trigger_changed`, the row whole. One per state a firing reaches.
    pub(crate) fn trigger_moved(&self, job: &Job, firing: &TriggerFiring) {
        self.publish(ipc::Event::JobTriggerChanged(ipc::JobTriggerChanged {
            job_id: ipc::JobId::from(job.id()),
            trigger: firing.into(),
            at: (&self.now()).into(),
        }));
    }

    pub(crate) fn trigger_line(&self, job: &Job, level: Level, said: &str) -> Envelope {
        self.trigger_line_at(job, self.now(), level, said)
    }

    pub(crate) fn trigger_line_at(
        &self,
        job: &Job,
        at: Timestamp,
        level: Level,
        said: &str,
    ) -> Envelope {
        Envelope::new(
            at,
            level,
            Component::Fleet,
            self.run().clone(),
            said.to_string(),
        )
        .in_job(job.id().as_ulid().clone())
    }

    /// The Job's log line for one firing. The output rides along as evidence and
    /// is never read: a person reads it.
    fn firing_line(
        &self,
        job: &Job,
        firing: &TriggerFiring,
        attempt: Option<&checks_runner::Attempt>,
    ) -> Envelope {
        let level = match firing.state {
            TriggerState::Failed | TriggerState::Repairing | TriggerState::Held => Level::Warn,
            _ => Level::Info,
        };
        let said = match (&firing.skipped, firing.state) {
            (Some(why), _) => format!("Trigger `{}` {why}", firing.name),
            (None, TriggerState::AwaitingOwner) => format!(
                "Trigger `{}` is on a destructive Command and waits on you before it runs",
                firing.name
            ),
            (None, state) => format!("Trigger `{}` {}", firing.name, state.as_wire()),
        };
        let text = |value: &str| FieldValue::Str(value.to_string());
        // Stamped with the firing's own end, which is what `JobTrigger::log_at`
        // points at: the line is found by the instant the record already holds.
        let at = firing.ended_at.clone().unwrap_or(firing.started_at.clone());
        let mut line = self
            .trigger_line_at(job, at, level, &said)
            .with_field("trigger", text(&firing.name))
            .with_field("when", text(firing.when.as_wire()))
            .with_field("step", text(firing.step.as_str()))
            .with_field("source", text(firing.source.as_wire()))
            .with_field("state", text(firing.state.as_wire()));
        if let Some(code) = firing.exit_code {
            line = line.with_field("exit_code", FieldValue::Str(code.to_string()));
        }
        if let Some(attempt) = attempt {
            line = line
                .with_field("stdout", text(&attempt.output.stdout))
                .with_field("stderr", text(&attempt.output.stderr));
        }
        line
    }
}
