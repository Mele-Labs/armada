//! Waking a Job's Commander when the Job is in trouble, once per fact. `#2167`;
//! `docs/concepts/commander.md`, *When it wakes*, has the troubles, where
//! each is read, and the loop bound.
//!
//! **Typed values only, never a log string.** Four troubles are fields of the
//! turn that found them. An environment failure happens while a Job is
//! admitted and spend is written when a Drone ends, so neither has a turn to sit
//! on: each leaves a note ([`Stirred`]) for the next turn to carry. A repair
//! Drone's end is made where it ends, by `crate::job_repair`.
//!
//! A wake is recorded first and made after, so neither a 250 ms tick nor a
//! restart wakes twice. The same trouble on the same step three times with the
//! Job exactly as it stood is a loop, and the fourth wake is not made.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level, StepId,
    Target,
};
use store::{CommanderProblem, CommanderWake, Moved};

use crate::allowance::{Nearly, Overspent};
use crate::converging::Stage;
use crate::daemon::Fleet;
use crate::drone::Aftermath;
use crate::gate::Ruling;
use crate::grouping::{group_end, GroupEnd};
use crate::silence::Vigil;
use crate::turning::{Turned, Worked};

/// Wakes on one problem, with the Job not moving between them, before Fleet
/// stops waking. `helm/commander_brief.rs` says the same to the Commander.
pub(crate) const WAKES_PER_PROBLEM: u32 = 3;

/// What happened outside a turn that the next turn carries. **Drained, not
/// read**, like `Fleet::delivered`. In memory: a restart loses it, and the
/// Job's next spend or escalation makes the note again.
#[derive(Default)]
pub(crate) struct Stirred {
    broken: Vec<(JobId, EscalationTrigger)>,
    spent: BTreeSet<JobId>,
    /// Repairs the last Fleet had out, found at boot. The rows stay in the store
    /// until the wake for each is made, so a Fleet that dies first finds them.
    lost: Vec<store::RepairOut>,
}

/// The escalations that mean no Drone could run: the worktree or the
/// environment is broken.
fn environmental(trigger: &EscalationTrigger) -> bool {
    matches!(
        trigger,
        EscalationTrigger::NoWorktree
            | EscalationTrigger::NotConfigurable
            | EscalationTrigger::WouldNotStart
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::NotPrepared
    )
}

/// The trouble a Job is in. A `None` step is the Job's current step, read when
/// the wake is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Trouble {
    /// A Drone's run ended, or it stopped answering, with nothing handed in.
    NothingHandedIn {
        step: Option<StepId>,
        found: EscalationTrigger,
        ended: bool,
    },
    /// A group's Check was red on the last run its retries allow.
    GroupRed { step: Option<StepId> },
    /// The Judge refused, or has a question for the owner.
    Refused { step: Option<StepId>, asked: bool },
    /// Work seen outside the step's declared scope.
    Drifted { step: StepId },
    /// The step was not converging, and the Drone never answered the instruction
    /// to report. **Not the instruction itself**: the Drone is given its chance
    /// first, and a wake before it answers would race it.
    Churning { step: StepId },
    /// The Job could not get a worktree, a configured Drone or a running one.
    Broken { found: EscalationTrigger },
    /// A cap is `NEARLY_PERCENT` spent.
    CapNearlySpent(Nearly),
    /// A repair Drone it sent ended with nothing handed in. `#2170`.
    RepairHandedNothingIn { step: StepId },
}

impl Trouble {
    /// The name the store keeps a wake under.
    fn key(&self) -> &'static str {
        match self {
            Trouble::NothingHandedIn { .. } => "nothing_handed_in",
            Trouble::GroupRed { .. } => "group_red",
            Trouble::Refused { .. } => "judge_refused",
            Trouble::Drifted { .. } => "drifted",
            Trouble::Churning { .. } => "churning",
            Trouble::Broken { .. } => "broken",
            Trouble::RepairHandedNothingIn { .. } => "repair_handed_nothing_in",
            Trouble::CapNearlySpent(Nearly {
                cap: Overspent::Cost,
                ..
            }) => "cost_cap_nearly_spent",
            Trouble::CapNearlySpent(Nearly {
                cap: Overspent::Turns,
                ..
            }) => "turn_cap_nearly_spent",
        }
    }

    /// `Some(None)` is the Job's current step, and `None` a trouble that
    /// belongs to the Job and no step.
    fn step(&self) -> Option<Option<&StepId>> {
        match self {
            Trouble::NothingHandedIn { step, .. }
            | Trouble::GroupRed { step }
            | Trouble::Refused { step, .. } => Some(step.as_ref()),
            Trouble::Drifted { step }
            | Trouble::Churning { step }
            | Trouble::RepairHandedNothingIn { step } => Some(Some(step)),
            Trouble::Broken { .. } | Trouble::CapNearlySpent(_) => None,
        }
    }

    /// The reason in a line, for the thread's `woke` row, in the words the
    /// Job's own status uses where it has one (`enum-verbs.toml`).
    fn words(&self, step: &str) -> String {
        match self {
            Trouble::NothingHandedIn {
                found, ended: true, ..
            } => format!(
                "The Drone on step {step} ended with nothing handed in: {}",
                verb(found)
            ),
            Trouble::NothingHandedIn { found, .. } => format!(
                "The Drone on step {step} stopped answering with nothing handed in: {}",
                verb(found)
            ),
            Trouble::GroupRed { .. } => {
                format!("Step {step} failed its Checks with its retries spent")
            }
            Trouble::Refused { asked: false, .. } => format!("The Judge refused step {step}"),
            Trouble::Refused { asked: true, .. } => {
                format!("The Judge has a question about step {step}")
            }
            Trouble::Drifted { .. } => {
                format!("The Drone on step {step} is working outside the plan")
            }
            Trouble::Churning { .. } => {
                format!("The Drone on step {step} went quiet when told to report")
            }
            Trouble::Broken { found } => format!("No Drone could run: {}", verb(found)),
            Trouble::RepairHandedNothingIn { .. } => {
                format!("The repair Drone on step {step} ended with nothing handed in")
            }
            Trouble::CapNearlySpent(Nearly {
                cap: Overspent::Cost,
                used,
                of,
            }) => format!(
                "The cost cap is nearly spent: {} of {}",
                dollars(*used),
                dollars(*of)
            ),
            Trouble::CapNearlySpent(Nearly {
                cap: Overspent::Turns,
                used,
                of,
            }) => format!("The turn cap is nearly spent: {used} of {of} turns"),
        }
    }
}

/// An escalation trigger in the words `enum-verbs.toml` gives it.
fn verb(trigger: &EscalationTrigger) -> &'static str {
    match trigger {
        EscalationTrigger::Silent
        | EscalationTrigger::Stalled
        | EscalationTrigger::HatchUnbidden => "stalled",
        EscalationTrigger::BlockedByPolicy => "blocked by policy",
        EscalationTrigger::Interrupted => "interrupted",
        EscalationTrigger::Unheard => "nothing is reading this Drone",
        EscalationTrigger::NoWorktree => "the worktree was unusable",
        EscalationTrigger::NotConfigurable => "the Drone could not be configured",
        EscalationTrigger::WouldNotStart => "the Drone would not start",
        EscalationTrigger::ResourceExhausted => "ran out of resources",
        EscalationTrigger::NotPrepared => "a required command did not succeed",
        _ => "stopped",
    }
}

fn dollars(micros: u64) -> String {
    format!("${:.2}", micros as f64 / 1_000_000.0)
}

/// Every trouble one Drone's turn found, read off the typed values on it.
pub(crate) fn troubles(worked: &Worked) -> Vec<Trouble> {
    let mut found = Vec::new();
    if let Some(Aftermath::JobMoves(Target::Escalated(trigger))) = &worked.after {
        if matches!(
            trigger,
            EscalationTrigger::Silent
                | EscalationTrigger::Stalled
                | EscalationTrigger::BlockedByPolicy
                | EscalationTrigger::Interrupted
        ) {
            found.push(Trouble::NothingHandedIn {
                step: None,
                found: trigger.clone(),
                ended: true,
            });
        }
    }
    if let Some(quiet) = &worked.quiet {
        match &quiet.said {
            Vigil::AtRest { found: trigger } => found.push(Trouble::NothingHandedIn {
                step: Some(quiet.step.clone()),
                found: trigger.clone(),
                ended: true,
            }),
            Vigil::Escalated { found: trigger, .. } => found.push(Trouble::NothingHandedIn {
                step: Some(quiet.step.clone()),
                found: trigger.clone(),
                ended: false,
            }),
            _ => {}
        }
    }
    if let Some(ruling) = &worked.ruled {
        if group_end(ruling) == GroupEnd::Failed {
            found.push(Trouble::GroupRed { step: None });
        }
        match ruling {
            Ruling::Refused { .. } => found.push(Trouble::Refused {
                step: None,
                asked: false,
            }),
            Ruling::Questioned { .. } => found.push(Trouble::Refused {
                step: None,
                asked: true,
            }),
            _ => {}
        }
    }
    if let Some(drifting) = &worked.drifting {
        found.push(Trouble::Drifted {
            step: drifting.step.clone(),
        });
    }
    if let Some(wandering) = &worked.wandering {
        if let Stage::Escalated { .. } = &wandering.stage {
            found.push(Trouble::Churning {
                step: wandering.step.clone(),
            });
        }
    }
    found
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
    /// A Job was moved onto this trigger. Kept only if it is an environment one.
    pub(crate) fn stirred_by_escalation(&self, job: &JobId, trigger: &EscalationTrigger) {
        if environmental(trigger) {
            self.stirrings()
                .lock()
                .expect("not poisoned")
                .broken
                .push((job.clone(), trigger.clone()));
        }
    }

    /// A Drone's spend was just written for this Job.
    pub(crate) fn stirred_by_spend(&self, job: &JobId) {
        self.stirrings()
            .lock()
            .expect("not poisoned")
            .spent
            .insert(job.clone());
    }

    /// A repair Drone the last Fleet sent, found at boot with its Job still held.
    pub(crate) fn stirred_by_a_lost_repair(&self, repair: store::RepairOut) {
        self.stirrings().lock().expect("not poisoned").lost.push(repair);
    }

    fn take_lost_repairs(&self) -> Vec<store::RepairOut> {
        std::mem::take(&mut self.stirrings().lock().expect("not poisoned").lost)
    }

    /// What was noted since the last turn, for this turn to carry.
    pub(crate) fn take_stirred(&self) -> (Vec<(JobId, EscalationTrigger)>, Vec<JobId>) {
        let mut stirred = self.stirrings().lock().expect("not poisoned");
        (
            std::mem::take(&mut stirred.broken),
            std::mem::take(&mut stirred.spent).into_iter().collect(),
        )
    }

    /// Wake the Commander of every Job this turn found in trouble. **Awaited by
    /// `keep_turning` beside `probed` and `walked`**; a turn with no trouble in
    /// it returns without reading the store.
    pub(crate) async fn woken(self: &Arc<Self>, turned: &Turned) {
        for lost in self.take_lost_repairs() {
            self.repair_lost(lost).await;
        }
        let mut found: Vec<(JobId, Trouble)> = Vec::new();
        for worked in &turned.each {
            for trouble in troubles(worked) {
                found.push((worked.job.clone(), trouble));
            }
        }
        for (job, trigger) in &turned.broken {
            let trouble = Trouble::Broken {
                found: trigger.clone(),
            };
            found.push((job.clone(), trouble));
        }
        for job in &turned.spent {
            for near in self.caps_nearly_spent(job).await {
                found.push((job.clone(), Trouble::CapNearlySpent(near)));
            }
        }
        for (job, trouble) in found {
            self.woken_for(&job, trouble).await;
        }
    }

    async fn caps_nearly_spent(&self, job: &JobId) -> Vec<Nearly> {
        let Ok(record) = self.load(job).await else {
            return Vec::new();
        };
        if record.status().is_terminal() {
            return Vec::new();
        }
        let Ok(spent) = self.spend_of(job).await else {
            return Vec::new();
        };
        self.allowance_for(&record).nearly_spent_by(&spent)
    }

    /// Claim the wake and make it, or say why it was not made.
    pub(crate) async fn woken_for(self: &Arc<Self>, job: &JobId, trouble: Trouble) {
        let Ok(record) = self.load(job).await else {
            return;
        };
        if record.status().is_terminal() {
            return;
        }
        let step = match trouble.step() {
            Some(Some(named)) => Some(named.clone()),
            Some(None) => record.current_step_id().cloned(),
            None => None,
        };
        let step_named = step.as_ref().map(StepId::as_str).unwrap_or_default().to_string();
        let noted = self.claimed(&record, &trouble, step.as_ref(), &step_named).await;
        match noted {
            Ok(CommanderWake::Already) => {}
            Ok(CommanderWake::New { run }) if run > WAKES_PER_PROBLEM => {
                let words = trouble.words(&step_named);
                self.noted_not_woken(job, &words, run);
                self.card_at_the_bound(&record, &words, WAKES_PER_PROBLEM).await;
            }
            Ok(CommanderWake::New { .. }) => {
                // A Commander that cannot start has said why in the Job's log,
                // and the Job stays with the owner as it did before.
                let _ = Arc::clone(self)
                    .wake_commander(job, &trouble.words(&step_named))
                    .await;
            }
            Err(why) => self.noted_wake_unrecorded(job, trouble.key(), why),
        }
    }

    /// Write the wake down: its attempt is the step's run, the Job's
    /// escalations so far where there is no step, or the cap in force.
    async fn claimed(
        &self,
        record: &Job,
        trouble: &Trouble,
        step: Option<&StepId>,
        step_named: &str,
    ) -> Result<CommanderWake, String> {
        let job = record.id();
        let mut store = self.store().lock().await;
        let attempt = match (trouble, step) {
            (Trouble::CapNearlySpent(near), _) => near.of,
            // A repair does not enter the step, so its attempt does not move:
            // each repair that hands nothing in is the next of its own.
            (Trouble::RepairHandedNothingIn { .. }, Some(step)) => {
                store
                    .commander_wakes_noted(job, trouble.key(), step.as_str())
                    .map_err(|why| why.to_string())?
                    + 1
            }
            (_, Some(step)) => u64::from(
                store
                    .step_attempt(job, step)
                    .map_err(|why| why.to_string())?
                    .number(),
            ),
            (_, None) => store
                .events_for(job)
                .map_err(|why| why.to_string())?
                .iter()
                .filter(|event| {
                    matches!(
                        event.moved(),
                        Moved::Job {
                            to: JobStatus::Escalated,
                            ..
                        }
                    )
                })
                .count() as u64,
        };
        let plan = store.work_plan(job).ok().flatten();
        let problem = CommanderProblem {
            trigger: trouble.key().to_string(),
            step: step_named.to_string(),
            attempt,
        };
        store
            .note_commander_wake(job, &problem, &standing(record, plan.as_ref()))
            .map_err(|why| why.to_string())
    }

    /// The loop bound. Nothing wakes the Commander again, and the log says so
    /// here; `card_at_the_bound` puts the card to the owner.
    fn noted_not_woken(&self, job: &JobId, problem: &str, run: u32) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the Job's Commander was woken three times for one problem and the Job has not moved, so it is not woken again",
        )
        .in_job(job.as_ulid().clone())
        .with_field("problem", FieldValue::Str(problem.to_string()))
        .with_field("run", FieldValue::Str(run.to_string()));
        self.noted_in_the_log(job, &envelope);
    }

    fn noted_wake_unrecorded(&self, job: &JobId, trouble: &str, why: String) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the Job's Commander was not woken because the wake could not be written down",
        )
        .in_job(job.as_ulid().clone())
        .with_field("trouble", FieldValue::Str(trouble.to_string()))
        .with_field("why", FieldValue::Str(why));
        self.noted_in_the_log(job, &envelope);
    }
}

/// The Job as the loop bound compares it between wakes: its status, each
/// step's state, and how many tasks are in each state.
fn standing(job: &Job, plan: Option<&core_model::WorkPlan>) -> String {
    let steps: Vec<String> = job
        .steps()
        .iter()
        .map(|step| format!("{}={}", step.step_id().as_str(), step.state().as_wire()))
        .collect();
    let tasks = plan.map(|plan| plan.counts()).unwrap_or_default();
    format!(
        "{}|{}|done:{},working:{},open:{},dropped:{},handed_in:{},failed:{}",
        job.status().as_wire(),
        steps.join(","),
        tasks.done,
        tasks.working,
        tasks.open,
        tasks.dropped,
        tasks.handed_in,
        tasks.failed,
    )
}
