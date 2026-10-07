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
enum Comes {
    Run { command: String },
    Skip(TriggerSkipped),
    AskTheOwner,
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

fn decided(resolution: &TriggerResolution, manifest: &Manifest) -> Comes {
    match resolution {
        TriggerResolution::Skipped(why) => Comes::Skip(why.clone()),
        TriggerResolution::Skill { name } => Comes::Skip(TriggerSkipped::SkillNotRun {
            skill: name.clone(),
        }),
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

    /// The command line to run, or `None` where nothing is to run.
    pub fn to_run(&self) -> Option<&str> {
        match &self.comes {
            Comes::Run { command } => Some(command),
            Comes::Skip(_) | Comes::AskTheOwner => None,
        }
    }

    /// The record as it opens: running, already skipped, or held for the owner.
    pub fn opened(&self, at: Timestamp) -> TriggerFiring {
        match &self.comes {
            Comes::Run { .. } => TriggerFiring::running(&self.trigger, at),
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

    /// Fire the Job's frozen Triggers for `when` on `step`, in its worktree, and
    /// record each. **Fails nothing**: every refusal in here is a line in the
    /// Job's log and the Job carries on.
    pub(crate) async fn fire_triggers(
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
        for one in plan(&frozen, when, step, served.manifest()) {
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
    }

    fn trigger_line(&self, job: &Job, level: Level, said: &str) -> Envelope {
        Envelope::new(
            self.now(),
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
            TriggerState::Failed => Level::Warn,
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
        let mut line = self
            .trigger_line(job, level, &said)
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
