//! A Trigger or an added step that blocks and fails holds its Job.
//! `docs/concepts/trigger.md`, *A failed Trigger with `block` on*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, AddedStep, EscalationTrigger, Job, JobId, JobStatus, Level, RepairRecord, ResolvedStep,
    StepId, Target, Timestamp, TransitionReason, TriggerFiring, TriggerState, TriggerWhen,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::policy::HeldBecause;
use crate::ruling::Ruling;
use crate::trigger_repair::Subject;

/// One thing holding a Job: a firing of a Trigger, or an added step.
#[derive(Debug, Clone)]
pub(crate) enum Hold {
    Firing { id: i64, firing: TriggerFiring },
    Addition(AddedStep),
}

impl Hold {
    pub(crate) fn name(&self) -> String {
        match self {
            Hold::Firing { firing, .. } => firing.name.clone(),
            Hold::Addition(added) => added.kind.text().to_string(),
        }
    }

    pub(crate) fn when(&self) -> TriggerWhen {
        match self {
            Hold::Firing { firing, .. } => firing.when,
            Hold::Addition(added) => added.when,
        }
    }

    pub(crate) fn step(&self) -> &StepId {
        match self {
            Hold::Firing { firing, .. } => &firing.step,
            Hold::Addition(added) => &added.step,
        }
    }

    pub(crate) fn subject(&self) -> Subject {
        match self {
            Hold::Firing { id, .. } => Subject::Firing(*id),
            Hold::Addition(added) => Subject::Addition(added.id.clone()),
        }
    }

    pub(crate) fn exit_code(&self) -> Option<i32> {
        match self {
            Hold::Firing { firing, .. } => firing.exit_code,
            Hold::Addition(added) => added.fired.as_ref().and_then(|fired| fired.exit_code),
        }
    }

    pub(crate) fn record(&self) -> &RepairRecord {
        match self {
            Hold::Firing { firing, .. } => &firing.repair,
            Hold::Addition(added) => &added.repair,
        }
    }

    /// When it last settled, which is how long it has waited on a person.
    pub(crate) fn since(&self) -> Option<&Timestamp> {
        let ended = match self {
            Hold::Firing { firing, .. } => firing.ended_at.as_ref(),
            Hold::Addition(added) => added
                .fired
                .as_ref()
                .and_then(|fired| fired.ended_at.as_ref()),
        };
        self.record().settled_at.as_ref().or(ended)
    }

    pub(crate) fn state(&self) -> TriggerState {
        match self {
            Hold::Firing { firing, .. } => firing.state,
            Hold::Addition(added) => added
                .fired
                .as_ref()
                .map_or(TriggerState::Held, |fired| fired.state),
        }
    }
}

/// Why a Job is on the alerts for a hold.
pub fn held_said(trigger: &str) -> String {
    format!("`{trigger}` failed and holds the Job")
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
    /// The latest firing of each blocking Trigger whose failure is unsettled,
    /// and each added step that is held.
    pub(crate) async fn holds_on(&self, job_id: &JobId) -> Result<Vec<Hold>, Adrift> {
        let job = self.load(job_id).await?;
        let store = self.store().lock().await;
        let mut out: Vec<Hold> = store
            .holding_firings(job_id)
            .map_err(Adrift::Reading)?
            .into_iter()
            .map(|(id, firing)| Hold::Firing { id, firing })
            .collect();
        out.extend(
            store
                .job_additions(job_id)
                .map_err(Adrift::Reading)?
                .into_iter()
                .filter(|added| added.holds_the_job(job.workflow()))
                .map(Hold::Addition),
        );
        Ok(out)
    }

    /// Every hold but a `pr_opened` one, whose Drone is the one working to the
    /// gate it holds.
    async fn held_before_a_drone(&self, job_id: &JobId) -> Result<Option<Hold>, Adrift> {
        Ok(self
            .holds_on(job_id)
            .await?
            .into_iter()
            .find(|hold| hold.when() != TriggerWhen::PrOpened))
    }

    /// Asked twice by `put_a_drone_on`, before the step's own Triggers fire and
    /// after. Stops the Job on `trigger_held` and raises [`Adrift::TriggerHolds`]
    /// where something holds it, as `no_worktree` does: nothing was spawned.
    pub(crate) async fn stopped_for_a_hold(&self, job: &Job) -> Result<(), Adrift> {
        let Some(hold) = self.held_before_a_drone(job.id()).await? else {
            return Ok(());
        };
        self.stopped_before_a_drone(job, EscalationTrigger::TriggerHeld)
            .await?;
        let place = match hold.when() {
            TriggerWhen::StepStarts => format!("before `{}`", hold.step().as_str()),
            TriggerWhen::StepPasses => format!("after `{}`", hold.step().as_str()),
            TriggerWhen::PrOpened => String::from("after the pull request opened"),
        };
        let said = format!("Trigger `{}` failed and holds the Job {place}", hold.name());
        self.logged(job.id(), self.trigger_line(job, Level::Warn, &said));
        Err(Adrift::TriggerHolds {
            job: job.id().clone(),
            trigger: hold.name(),
        })
    }

    /// Refuses an approval or a merge at a gate a hold stands in front of.
    pub(crate) async fn refused_while_held(&self, job_id: &JobId) -> Result<(), Adrift> {
        match self.holds_on(job_id).await?.into_iter().next() {
            None => Ok(()),
            Some(hold) => Err(Adrift::TriggerHolds {
                job: job_id.clone(),
                trigger: hold.name(),
            }),
        }
    }

    /// For the merge sweep, which asks again on its next rotation.
    pub(crate) async fn is_held(&self, job_id: &JobId) -> bool {
        self.holds_on(job_id)
            .await
            .is_ok_and(|holds| !holds.is_empty())
    }

    /// A delivering step's advance becomes a held review while a `pr_opened`
    /// hold stands in front of it, so the approval is the owner's and refused
    /// until the hold is let go.
    pub(crate) async fn guarded_against_a_held_delivery(
        &self,
        job_id: &JobId,
        step: &ResolvedStep,
        ruling: Ruling,
    ) -> Result<Ruling, Adrift> {
        if !step.delivers() {
            return Ok(ruling);
        }
        let held = self
            .holds_on(job_id)
            .await?
            .iter()
            .any(|hold| hold.when() == TriggerWhen::PrOpened);
        if !held {
            return Ok(ruling);
        }
        Ok(match ruling {
            Ruling::Advanced {
                checks,
                output,
                judged,
                cleared,
                policies,
                ..
            }
            | Ruling::Finished {
                checks,
                output,
                judged,
                cleared,
                policies,
                ..
            } => Ruling::HeldForReview {
                checks,
                output,
                judged,
                cleared,
                held: HeldBecause::ATriggerHoldsIt,
                policies,
            },
            other => other,
        })
    }

    /// A hold was let go. Where nothing else holds the Job and it is `escalated`
    /// on `trigger_held`, it goes back in the queue and admission puts a Drone
    /// on the step it stopped before. `true` where nothing holds it now.
    pub(crate) async fn hold_let_go(&self, job_id: &JobId, by: Actor) -> bool {
        let Ok(held) = self.holds_on(job_id).await else {
            return false;
        };
        if !held.is_empty() {
            return false;
        }
        let Ok(job) = self.load(job_id).await else {
            return true;
        };
        let stopped_here = job.status() == JobStatus::Escalated
            && matches!(
                self.last_reason(job_id).await,
                Ok(Some(TransitionReason::Escalation(
                    EscalationTrigger::TriggerHeld
                )))
            );
        if stopped_here {
            if let Err(why) = self.move_job(&job, Target::Queued, by).await {
                let said = format!("the Job could not be put back in the queue: {why}");
                self.logged(job_id, self.trigger_line(&job, Level::Warn, &said));
            }
        }
        true
    }

    /// What a Job's row carries on its bell, most pressing first: a hold, a fix
    /// waiting on a choice, a failure nobody could repair. Nothing once it is over.
    pub(crate) fn alert_on_row(&self, store: &store::Store, job: &Job) -> Option<ipc::JobAlert> {
        let over = job.status().is_terminal();
        let (mut held, mut fix, mut failed) = (None, None, None);
        for (_, firing) in store.alerting_firings(job.id()).unwrap_or_default() {
            if over && firing.state == TriggerState::Held {
                continue;
            }
            let slot = match firing.state {
                TriggerState::Held => &mut held,
                TriggerState::FixReady => &mut fix,
                _ => &mut failed,
            };
            slot.get_or_insert((firing.name.clone(), firing.when, firing.step.clone()));
        }
        // An added step is read by the same rule: a hold, a fix with no
        // choice, a failure after a repair was tried.
        for added in store.job_additions(job.id()).unwrap_or_default() {
            let Some(fired) = &added.fired else {
                continue;
            };
            let slot = match fired.state {
                TriggerState::Held if !over && added.holds_the_job(job.workflow()) => &mut held,
                TriggerState::FixReady if added.repair.choice.is_none() => &mut fix,
                TriggerState::Failed if added.repair.tries > 0 => &mut failed,
                _ => continue,
            };
            slot.get_or_insert((
                added.kind.text().to_string(),
                added.when,
                added.step.clone(),
            ));
        }
        let (kind, (trigger, when, step)) = match (held, fix, failed) {
            (Some(one), _, _) => (ipc::JobAlertKind::Held, one),
            (None, Some(one), _) => (ipc::JobAlertKind::FixReady, one),
            (None, None, Some(one)) => (ipc::JobAlertKind::Failed, one),
            (None, None, None) => return None,
        };
        Some(ipc::JobAlert {
            kind,
            trigger,
            when: when.into(),
            step: ipc::StepId::from(&step),
        })
    }
}
