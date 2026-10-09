//! The owner's two acts on a hold: rerun the Command, or skip it. The same two
//! answer a destructive Command that asks first: Run is its Rerun.
//! `docs/concepts/trigger.md`, *A failed Trigger with `block` on* and *A
//! destructive Command asks first*.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use api::Refusal;
use core_model::{
    Actor, AddedKind, Fired, Job, JobId, Level, NotRun, TriggerFiring, TriggerResolution,
    TriggerState, TriggerWhen,
};
use ipc::WireError;
use verification::Exit;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::trigger_hold::Hold;
use crate::trigger_repair::{Subject, Waiting};

const NO_HOLD: &str = "fleet.no_hold";
const NO_HOLD_NAMED: &str = "fleet.no_hold_named";
const HOLD_REPAIRING: &str = "fleet.hold_repairing";
const HOLD_HAS_A_FIX: &str = "fleet.hold_has_a_fix";
const HOLD_NOTHING_TO_RUN: &str = "fleet.hold_nothing_to_run";
const HOLD_NO_WORKTREE: &str = "fleet.hold_no_worktree";
const HOLD_JOB_WORKING: &str = "fleet.hold_job_working";
const HOLD_ALREADY_RUNNING: &str = "fleet.hold_already_running";

enum Named {
    Trigger(String),
    Addition(String),
}

fn named(act: &ipc::HoldAct) -> Option<Named> {
    match (&act.trigger, &act.addition) {
        (Some(trigger), None) => Some(Named::Trigger(trigger.clone())),
        (None, Some(addition)) => Some(Named::Addition(addition.clone())),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Act {
    Rerun,
    Skip,
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
    /// `rerun_trigger`, spawned and awaited as `choose_trigger_fix` is, so a
    /// client that stops waiting does not stop the Command halfway.
    pub(crate) async fn hold_rerun(
        self: std::sync::Arc<Self>,
        job_id: ipc::JobId,
        act: ipc::HoldAct,
    ) -> Result<ipc::HoldSettled, Refusal> {
        let fleet = std::sync::Arc::clone(&self);
        let job = job_id.to_domain();
        let ran =
            tokio::spawn(async move { fleet.hold_acted_on(&job, &act, Act::Rerun).await }).await;
        match ran {
            Ok(settled) => settled,
            Err(why) => Err(Refusal::Fault(WireError::raised(
                HOLD_NO_WORKTREE,
                format!("running the Command stopped: {why}"),
                self.run_id(),
            ))),
        }
    }

    /// `skip_trigger`.
    pub(crate) async fn hold_skip(
        &self,
        job_id: ipc::JobId,
        act: ipc::HoldAct,
    ) -> Result<ipc::HoldSettled, Refusal> {
        self.hold_acted_on(&job_id.to_domain(), &act, Act::Skip)
            .await
    }

    fn not_held(&self, job_id: &JobId, code: &'static str, said: String) -> Refusal {
        Refusal::IllegalMove(
            WireError::raised(code, said, self.run_id()).about_job(ipc::JobId::from(job_id)),
        )
    }

    async fn hold_acted_on(
        &self,
        job_id: &JobId,
        act: &ipc::HoldAct,
        doing: Act,
    ) -> Result<ipc::HoldSettled, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let Some(asked) = named(act) else {
            return Err(Refusal::Unacceptable(
                WireError::raised(
                    NO_HOLD_NAMED,
                    String::from(
                        "name the Trigger or the added step that holds the Job, one of them",
                    ),
                    self.run_id(),
                )
                .about_job(ipc::JobId::from(job_id)),
            ));
        };
        let holds = self
            .holds_and_asks_on(job_id)
            .await
            .map_err(|why| self.refusal(why))?;
        let hold = holds
            .into_iter()
            .rev()
            .find(|hold| match (&asked, hold) {
                (Named::Trigger(name), Hold::Firing { firing, .. }) => &firing.name == name,
                (Named::Addition(id), Hold::Addition(added)) => &added.id == id,
                _ => false,
            })
            .ok_or_else(|| {
                let (Named::Trigger(which) | Named::Addition(which)) = &asked;
                self.not_held(
                    job_id,
                    NO_HOLD,
                    format!("nothing named `{which}` holds this Job"),
                )
            })?;
        let state = hold.state();
        if hold.record().side_run_in_flight(state) {
            return Err(self.not_held(
                job_id,
                HOLD_REPAIRING,
                format!(
                    "`{}` is still running, and the hold goes by itself when it ends well",
                    hold.name()
                ),
            ));
        }
        if matches!(state, TriggerState::Repairing | TriggerState::Rerunning) {
            return Err(self.not_held(
                job_id,
                HOLD_REPAIRING,
                format!(
                    "`{}` is being repaired, and the hold goes by itself when the repair passes",
                    hold.name()
                ),
            ));
        }
        match doing {
            Act::Skip => self.hold_skipped(&job, hold).await,
            Act::Rerun if state == TriggerState::FixReady => Err(self.not_held(
                job_id,
                HOLD_HAS_A_FIX,
                format!(
                    "a fix for `{}` waits on your choice: place it, or skip the Trigger",
                    hold.name()
                ),
            )),
            Act::Rerun => self.hold_rerun_now(&job, hold).await,
        }
    }

    async fn hold_skipped(&self, job: &Job, hold: Hold) -> Result<ipc::HoldSettled, Refusal> {
        let at = self.now();
        // An ask that does not block held nothing to release.
        let first_entry = hold.when() == TriggerWhen::StepStarts && hold.blocks();
        let state = match &hold {
            Hold::Firing { id, firing } => {
                let after = firing.clone().skipped_by_the_owner(at);
                self.store()
                    .lock()
                    .await
                    .settle_hold(*id, &after, first_entry)
                    .map_err(|why| self.refusal(Adrift::Writing(why)))?;
                self.trigger_moved(job, &after);
                self.logged(job.id(), self.hold_line(job, &after, None, true));
                after.state
            }
            Hold::Addition(added) => {
                let fired = added
                    .fired
                    .clone()
                    .unwrap_or_else(|| Fired::running(at.clone()));
                let skipped = Fired {
                    state: TriggerState::Skipped,
                    not_run: Some(NotRun::ByOwner),
                    ended_at: Some(at),
                    ..fired
                };
                self.addition_settled(job, added, &skipped, first_entry)
                    .await
                    .map_err(|why| self.refusal(why))?;
                skipped.state
            }
        };
        let released = self.hold_let_go(job.id(), Actor::Human).await;
        Ok(ipc::HoldSettled {
            state: state.into(),
            released,
        })
    }

    async fn hold_rerun_now(&self, job: &Job, hold: Hold) -> Result<ipc::HoldSettled, Refusal> {
        let job_id = job.id();
        if let Some(side) = self.side_of(job, &hold).await {
            self.side_rerun(job, &hold, &side).await;
            return Ok(ipc::HoldSettled {
                state: TriggerState::Running.into(),
                released: false,
            });
        }
        if self.job_is_working(job_id).await {
            return Err(self.not_held(
                job_id,
                HOLD_JOB_WORKING,
                String::from("a Drone is working in the Job's worktree: rerun when it stops"),
            ));
        }
        let command = self.hold_command(job, &hold).await.ok_or_else(|| {
            self.not_held(
                job_id,
                HOLD_NOTHING_TO_RUN,
                format!(
                    "`{}` no longer names a Command this repository declares, or is not one \
                     Fleet runs",
                    hold.name()
                ),
            )
        })?;
        let worktree: Worktree = match self.worktree_of(job) {
            Ok(Some(tree)) => tree,
            Ok(None) => {
                return Err(self.not_held(
                    job_id,
                    HOLD_NO_WORKTREE,
                    String::from("the Job's worktree is not there to run the Command in"),
                ))
            }
            Err(why) => return Err(self.not_held(job_id, HOLD_NO_WORKTREE, why.to_string())),
        };
        if let Hold::Firing { id, firing } = &hold {
            if firing.state == TriggerState::AwaitingOwner {
                return self
                    .owner_ran(job, *id, firing, &command, Path::new(worktree.path()))
                    .await;
            }
        }
        if let Hold::Firing { firing, .. } = &hold {
            let running = TriggerFiring {
                state: TriggerState::Rerunning,
                ..firing.clone()
            };
            self.trigger_moved(job, &running);
        }
        let attempt = checks_runner::run(
            &command,
            Path::new(worktree.path()),
            self.budget().duration(),
        )
        .await;
        let passed = matches!(attempt.exit, Exit::Code(0));
        let code = match &attempt.exit {
            Exit::Code(code) => Some(*code),
            _ => None,
        };
        let (at, first_entry) = (self.now(), hold.when() == TriggerWhen::StepStarts);
        let state = if passed {
            TriggerState::Passed
        } else {
            TriggerState::Held
        };
        match &hold {
            Hold::Firing { id, firing } => {
                let after = TriggerFiring {
                    state,
                    exit_code: code,
                    ended_at: Some(at),
                    ..firing.clone()
                };
                self.store()
                    .lock()
                    .await
                    .settle_hold(*id, &after, passed && first_entry)
                    .map_err(|why| self.refusal(Adrift::Writing(why)))?;
                self.trigger_moved(job, &after);
                self.logged(job_id, self.hold_line(job, &after, Some(&attempt), true));
            }
            Hold::Addition(added) => {
                let fired = added
                    .fired
                    .clone()
                    .unwrap_or_else(|| Fired::running(at.clone()));
                let after = Fired {
                    state,
                    exit_code: code,
                    ended_at: Some(at),
                    ..fired
                };
                self.addition_settled(job, added, &after, passed && first_entry)
                    .await
                    .map_err(|why| self.refusal(why))?;
            }
        }
        let released = passed && self.hold_let_go(job_id, Actor::Human).await;
        Ok(ipc::HoldSettled {
            state: state.into(),
            released,
        })
    }

    /// The owner's Run on a destructive Command that asked: it runs once, in
    /// the Job's worktree, and ends as a firing does. Passed lets the Job go,
    /// a failure is held where `block` is on, and `repair` queues a repair
    /// Drone. **The store still reads `awaiting_owner` while it runs**, so a
    /// Fleet that stops halfway asks him again and never runs it unasked;
    /// `owner_runs` is what refuses a second Run meanwhile.
    async fn owner_ran(
        &self,
        job: &Job,
        id: i64,
        firing: &TriggerFiring,
        command: &str,
        worktree: &Path,
    ) -> Result<ipc::HoldSettled, Refusal> {
        if !self.owner_runs().lock().expect("not poisoned").insert(id) {
            return Err(self.not_held(
                job.id(),
                HOLD_ALREADY_RUNNING,
                format!("`{}` is already running", firing.name),
            ));
        }
        self.trigger_moved(
            job,
            &TriggerFiring {
                state: TriggerState::Running,
                ..firing.clone()
            },
        );
        let attempt = checks_runner::run(command, worktree, self.budget().duration()).await;
        self.owner_runs().lock().expect("not poisoned").remove(&id);
        let code = match &attempt.exit {
            Exit::Code(code) => Some(*code),
            _ => None,
        };
        let after = firing.clone().ended(code, self.now());
        let passed = after.state == TriggerState::Passed;
        let first_entry = firing.when == TriggerWhen::StepStarts && firing.on_failure.block;
        self.store()
            .lock()
            .await
            .settle_hold(id, &after, passed && first_entry)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        self.trigger_moved(job, &after);
        self.logged(job.id(), self.hold_line(job, &after, Some(&attempt), false));
        if after.state == TriggerState::Repairing {
            self.trigger_repairs()
                .lock()
                .expect("not poisoned")
                .push(Waiting {
                    job: job.id().clone(),
                    subject: Subject::Firing(id),
                    trigger: after.name.clone(),
                    step: after.step.clone(),
                    command: command.to_string(),
                    exit: after.exit_code,
                    stdout: attempt.output.stdout.clone(),
                    stderr: attempt.output.stderr.clone(),
                    record: core_model::RepairRecord::default(),
                    side: None,
                });
        }
        let released = !after.holds_the_job() && self.hold_let_go(job.id(), Actor::Human).await;
        Ok(ipc::HoldSettled {
            state: after.state.into(),
            released,
        })
    }

    /// The Command a hold would run again: the Trigger's as it now reads, or the
    /// added Script's. A skill and a Drone step have none.
    pub(crate) async fn hold_command(&self, job: &Job, hold: &Hold) -> Option<String> {
        match hold {
            Hold::Firing { firing, .. } => self.command_of(job, firing).await,
            Hold::Addition(added) => {
                let AddedKind::Script { command } = &added.kind else {
                    return None;
                };
                let served = self.served_by(job).ok()?;
                let (manifest, _) = self.effective_manifest_in(&served, job).await;
                let asked = TriggerResolution::Command {
                    name: command.clone(),
                    asks_first: false,
                };
                match crate::triggering::decided(&asked, &manifest) {
                    crate::triggering::Comes::Run { command } => Some(command),
                    _ => None,
                }
            }
        }
    }

    fn hold_line(
        &self,
        job: &Job,
        firing: &TriggerFiring,
        attempt: Option<&checks_runner::Attempt>,
        again: bool,
    ) -> core_model::Envelope {
        let said = match &firing.skipped {
            Some(why) => format!("Trigger `{}` {why}", firing.name),
            None => format!(
                "Trigger `{}` run {}by you: {}",
                firing.name,
                if again { "again " } else { "" },
                firing.state.as_wire()
            ),
        };
        let text = |value: &str| core_model::FieldValue::Str(value.to_string());
        let at = firing.ended_at.clone().unwrap_or(firing.started_at.clone());
        let level = if firing.state == TriggerState::Held {
            Level::Warn
        } else {
            Level::Info
        };
        let mut line = self
            .trigger_line_at(job, at, level, &said)
            .with_field("trigger", text(&firing.name))
            .with_field("when", text(firing.when.as_wire()))
            .with_field("step", text(firing.step.as_str()))
            .with_field("source", text(firing.source.as_wire()))
            .with_field("state", text(firing.state.as_wire()));
        if let Some(code) = firing.exit_code {
            line = line.with_field("exit_code", core_model::FieldValue::Str(code.to_string()));
        }
        if let Some(attempt) = attempt {
            line = line
                .with_field("stdout", text(&attempt.output.stdout))
                .with_field("stderr", text(&attempt.output.stderr));
        }
        line
    }
}
