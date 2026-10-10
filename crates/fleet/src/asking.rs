//! A Judge criterion that refuses, and asks rather than stops the step.
//!
//! `docs/concepts/judge.md`'s asking design: every criterion asks by default,
//! a criterion marked `refuse` — or a Job set to [`WhenRefused::AlwaysRefuse`]
//! — still stops the step exactly as it did before this existed, and
//! `declared_plan_drift` can never reach [`Ruling::Refused`] at all. This
//! module is the other half of that design: what happens once
//! [`crate::judging::looks::JudgeFold`] decides a refusal is one to ask about.
//!
//! **The step holds at the same states a `human_always` review gate already
//! reaches** — `awaiting_human` beneath `awaiting_review` — and the Drone is
//! not stood down, unlike that gate: `crate::dispatch`'s own arm says why.
//! Answering is one of three moves, and none of them times out into a
//! refusal — an unanswered question just holds, exactly as an unanswered
//! review does.
//!
//! [`WhenRefused`]: core_model::WhenRefused

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepId, StepLevelTrigger, StepTarget, Target,
};
use ipc::{JudgeAnswer, JudgeQuestion};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::{Ending, Redirection};
use crate::ruling::Ruling;

/// The wire's [`ipc::WhenRefused`], from the domain's. Beside
/// `crate::permitting::wire_setting` for the same setting's sibling.
pub fn wire_when_refused(when: core_model::WhenRefused) -> ipc::WhenRefused {
    match when {
        core_model::WhenRefused::PerCriterion => ipc::WhenRefused::PerCriterion,
        core_model::WhenRefused::AlwaysAsk => ipc::WhenRefused::AlwaysAsk,
        core_model::WhenRefused::AlwaysRefuse => ipc::WhenRefused::AlwaysRefuse,
    }
}

/// The domain's, from the wire's.
pub fn domain_when_refused(when: ipc::WhenRefused) -> core_model::WhenRefused {
    match when {
        ipc::WhenRefused::PerCriterion => core_model::WhenRefused::PerCriterion,
        ipc::WhenRefused::AlwaysAsk => core_model::WhenRefused::AlwaysAsk,
        ipc::WhenRefused::AlwaysRefuse => core_model::WhenRefused::AlwaysRefuse,
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
    /// Write down the question this ruling opens. **Called once, from
    /// `crate::dispatch::act_on`'s `Questioned` arm, after the step has moved
    /// but before the Job's move to `awaiting_review` is applied and
    /// announced** — so a client that reacts to that announcement by
    /// re-reading the Job always finds the question `get_job` would answer
    /// with already on record.
    pub(crate) async fn asked_the_judge_question(
        &self,
        job: &Job,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        let Ruling::Questioned {
            question,
            question_text,
            ..
        } = ruling
        else {
            return Ok(());
        };
        // **Rule 4 already guarantees all three are `Some`.** A refusal that
        // cited nothing never reached `Judgment` at all —
        // `verification::Brief::read` refuses it before this ruling exists.
        let (Some(expected), Some(produced), Some(consequence)) = (
            question.expected.as_deref(),
            question.produced.as_deref(),
            question.consequence.as_deref(),
        ) else {
            return Ok(());
        };
        let now = self.now();
        self.store()
            .lock()
            .await
            .record_judge_question(
                job.id(),
                step.as_str(),
                &question.criterion_id,
                question_text,
                expected,
                produced,
                consequence,
                &now,
            )
            .map_err(Adrift::Writing)?;
        self.noted_asking(
            job.id(),
            step,
            crate::retro::lines::A_JUDGE_ASKS,
            question.criterion_id.as_str(),
        );
        Ok(())
    }

    /// The question this Job is holding open, for `get_job`. `None` where
    /// nothing is open — every Job most of the time.
    pub(crate) async fn judge_question_of(&self, job_id: &JobId) -> Option<JudgeQuestion> {
        let open = self
            .store()
            .lock()
            .await
            .open_judge_question(job_id)
            .ok()??;
        Some(JudgeQuestion {
            step_id: ipc::StepId::from(&StepId::new(open.step_id)),
            criterion_id: (&open.criterion_id).into(),
            question: open.question,
            expected: open.expected,
            produced: open.produced,
            consequence: open.consequence,
            asked_at: (&open.asked_at).into(),
            brief_path: None,
        })
    }

    /// A person's answer to the question this Job is holding open.
    ///
    /// **Agree sends the step back to a Drone, carrying the Judge's finding**
    /// — the owner's decision of 2 Oct 2026; until then it stopped the Job.
    /// The step stops as it would where the criterion is marked `refuse`,
    /// walked through `running` (`awaiting_review -> escalated` is
    /// `interrupted`'s edge alone, so this takes the same two declared edges
    /// `reviewing::Fleet::loop_is_spent` walks off the same gate), and then
    /// takes `restart_step`'s road with the finding as its note: the Job is
    /// queued and the next Drone opens on it. **No retry budget is read** —
    /// a person's press is not a retry, and `restart_step` never reads one.
    /// [`Ending::Any`], since the Drone a question leaves idle is not one a
    /// person asked to keep. Where the restart is refused, the Job stays
    /// `escalated` with the restart offered, which is what Agree used to do.
    /// **Disagree advances it**: `awaiting_human -> advanced`, the edge a
    /// person approving a review gate walks, and the Job goes on to the next
    /// step or completes.
    ///
    /// **The question clears last, after the move lands** — a move that fails
    /// leaves it open rather than stranding an answered, stopped step with
    /// nothing left to retry.
    ///
    /// **`asked_at`, where sent, is checked against the question genuinely
    /// open now** — the same `Superseded` shape as
    /// `crate::questioning::NotAnswered`. Absent, a peer built before 13.35
    /// trusts whatever is open, as this always did.
    pub async fn answer_judge(
        &self,
        job_id: &JobId,
        answer: JudgeAnswer,
        asked_at: Option<ipc::Instant>,
        note: Option<String>,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        if job.status() != JobStatus::AwaitingReview {
            return Err(Adrift::NotAnswerable {
                job: job_id.clone(),
                because: "this job is not holding a judge question open".to_string(),
            });
        }
        let open = self
            .store()
            .lock()
            .await
            .open_judge_question(job_id)
            .map_err(Adrift::Reading)?
            .ok_or_else(|| Adrift::NotAnswerable {
                job: job_id.clone(),
                because: "this job is not holding a judge question open".to_string(),
            })?;
        if let Some(named) = asked_at {
            if ipc::Instant::from(&open.asked_at) != named {
                return Err(Adrift::NotAnswerable {
                    job: job_id.clone(),
                    because: "the judge question being answered is not the one open now. Read \
                              the job again and answer the one it names"
                        .to_string(),
                });
            }
        }
        let step = StepId::new(open.step_id.clone());
        let job = match answer {
            JudgeAnswer::Agree => {
                let trigger = StepLevelTrigger::of(EscalationTrigger::GateFailure)
                    .expect("gate_failure is a step-level trigger");
                let job = self.move_job(&job, Target::Running, Actor::Human).await?;
                let job = self
                    .move_step_by(&job, &step, StepTarget::Stopped(trigger), Actor::Human)
                    .await?;
                self.move_job(
                    &job,
                    Target::Escalated(EscalationTrigger::GateFailure),
                    Actor::Human,
                )
                .await?;
                let finding = Redirection::saying(&finding(&open, note.as_deref()))
                    .expect("a finding always names its criterion");
                match self
                    .restart_step_ending(job_id, Some(&finding), Ending::Any)
                    .await
                {
                    Ok(job) => job,
                    // The answer landed — the step stopped — so the question
                    // is not left open over a Job that can no longer serve it.
                    Err(refused) => {
                        self.store()
                            .lock()
                            .await
                            .clear_judge_question(job_id)
                            .map_err(Adrift::Writing)?;
                        return Err(refused);
                    }
                }
            }
            JudgeAnswer::DisagreeOnce | JudgeAnswer::DisagreeAlways => {
                let slot = self.slot_for(job_id).await;
                let mut working = slot.lock().await;
                let passed = self.declared_step(&job, &step)?.clone();
                let next = job.workflow().after(&step).cloned();
                let job = self
                    .move_step_by(&job, &step, StepTarget::Advanced, Actor::Human)
                    .await?;
                match next {
                    None => {
                        let told = OutcomeTurn::approved(&passed, None, &self.prompts().wording());
                        self.completed(&job, &told, job_id, &mut working, Actor::Human)
                            .await?
                    }
                    Some(_) => self.move_job(&job, Target::Queued, Actor::Human).await?,
                }
            }
        };
        if answer == JudgeAnswer::DisagreeAlways {
            self.store()
                .lock()
                .await
                .record_tolerance(&open.criterion_id, &self.now(), Actor::Human)
                .map_err(Adrift::Writing)?;
        }
        self.store()
            .lock()
            .await
            .clear_judge_question(job_id)
            .map_err(Adrift::Writing)?;
        self.noted_asking(job_id, &step, answered(answer), open.criterion_id.as_str());
        if let Some(note) = note.as_deref().filter(|note| !note.trim().is_empty()) {
            self.noted_answer_note(job_id, &step, note);
        }
        Ok(job)
    }

    /// This Job's setting, for `get_job`. `per_criterion` where the store will
    /// not say, which is what the next gate would read.
    pub(crate) async fn when_refused_of(&self, job: &JobId) -> ipc::WhenRefused {
        let when = self
            .store()
            .lock()
            .await
            .when_refused(job)
            .unwrap_or_default();
        wire_when_refused(when)
    }

    /// Change how this Job meets a Judge criterion that refuses. **Read by
    /// the next gate**, so nothing respawns.
    pub async fn set_when_refused(
        &self,
        job: &JobId,
        when: core_model::WhenRefused,
    ) -> Result<(), Adrift> {
        self.store()
            .lock()
            .await
            .set_when_refused(job, when)
            .map_err(Adrift::Writing)?;
        if let Some(step) = self
            .load(job)
            .await
            .ok()
            .and_then(|record| record.current_step().map(|step| step.step_id().clone()))
        {
            self.noted_about(
                job,
                &step,
                "a person changed how this job meets a judge refusal",
                "when_refused",
                when.as_wire(),
            );
        }
        Ok(())
    }

    fn noted_asking(&self, job: &JobId, step: &StepId, message: &'static str, criterion: &str) {
        self.noted_about(job, step, message, "criterion_id", criterion);
    }

    fn noted_about(
        &self,
        job: &JobId,
        step: &StepId,
        message: &'static str,
        field: &'static str,
        value: &str,
    ) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            message,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(field, FieldValue::Str(value.to_string()));
        self.noted_in_the_log(job, &envelope);
    }

    fn noted_answer_note(&self, job: &JobId, step: &StepId, note: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person's note on their answer to a judge question",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("note", FieldValue::Str(note.to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}

/// What the next Drone is told where a person agreed with a refusal: the
/// Judge's own three fields, and the person's note where they wrote one.
/// `crossing::Redirected` frames it.
fn finding(open: &store::OpenJudgeQuestion, note: Option<&str>) -> String {
    let mut said = format!(
        "The Judge refused this step on `{}` and a person agreed: {}\nExpected: {}\nFound: \
         {}\nWhat that does: {}",
        open.criterion_id.as_str(),
        open.question,
        open.expected,
        open.produced,
        open.consequence,
    );
    if let Some(note) = note.map(str::trim).filter(|note| !note.is_empty()) {
        said.push_str(&format!("\nTheir note: {note}"));
    }
    said
}

fn answered(answer: JudgeAnswer) -> &'static str {
    match answer {
        JudgeAnswer::Agree => crate::retro::lines::A_PERSON_AGREES,
        JudgeAnswer::DisagreeOnce => crate::retro::lines::A_PERSON_DISAGREES_ONCE,
        JudgeAnswer::DisagreeAlways => crate::retro::lines::A_PERSON_DISAGREES_ALWAYS,
    }
}
