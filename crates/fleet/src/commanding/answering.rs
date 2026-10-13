//! A person's answers to what a Drone or the Judge asked. Each is one line in `commanding`'s
//! `Commands` impl, which cannot be split across files and reached the 1200-line rule.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobId, JobSummary};

use crate::budget::budgeted_for;
use crate::daemon::Fleet;
use crate::timeline::{command_answer_says, judge_answer_says, question_answer_says};

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
    /// A person's answer to the question a Drone asked before it stood down.
    /// **Nothing moves** — the Job is `queued` before and after, and admission
    /// takes it in its turn. The summary comes back because a caller folds the
    /// row rather than re-reading the board. The refusals are
    /// `crate::questioning::NotAnswered`'s.
    pub(super) async fn question_answered(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        let (asked_of, says) = (job_id.clone(), question_answer_says(&answer));
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let question = answer.question_id.as_str();
                // A label or a mode answers a single question and picks answer
                // an interview. Anything else goes the interview's way, where
                // the missing picks are refused by name.
                if answer.decisions.is_empty() && (answer.chose.is_some() || answer.mode.is_some())
                {
                    Fleet::answer_question(
                        &fleet,
                        &id,
                        question,
                        answer.chose.as_deref(),
                        answer.mode,
                    )
                    .await
                } else {
                    Fleet::answer_decisions(&fleet, &id, question, &answer.decisions).await
                }
                .map_err(|why| why.about(&id))?;
                fleet.load(&id).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.job_answered(&asked_of, says);
        self.summarised(&job).await
    }

    /// A person's answer to a command a Drone was refused or is waiting on.
    /// The refusals are `crate::permitting::NotPermitted`'s.
    ///
    /// **The note and the rule are both read here and nowhere below**, by the
    /// one constructor that decides which of the three answers this is:
    /// `crate::permitting::Answered` has no field for a note on an allow or a
    /// rule on anything but Always allow, so either sent where it does not
    /// belong is dropped at this line rather than carried down and ignored
    /// later. An empty or blank note is the same request as none —
    /// `Note::saying`.
    pub(super) async fn command_answered(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::AnswerCommand,
    ) -> Result<JobSummary, Refusal> {
        let answered = crate::permitting::Answered::naming(
            answer.answer,
            answer.note.as_deref(),
            answer.rule.as_deref(),
        )
        .kept_in(answer.scope);
        let (asked_of, says) = (job_id.clone(), command_answer_says(answer.answer));
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                Fleet::answer_command(&fleet, &id, &answer.call, answered, answer.mode)
                    .await
                    .map_err(|why| why.about(&id))?;
                fleet.load(&id).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.job_answered(&asked_of, says);
        self.summarised(&job).await
    }

    /// A person's answer to the question a Judge refusal opened.
    /// `crate::asking::answer_judge`'s refusals.
    pub(super) async fn judge_answered(
        self: Arc<Self>,
        job_id: JobId,
        answered: ipc::JudgeAnswered,
    ) -> Result<JobSummary, Refusal> {
        let (asked_of, says) = (job_id.clone(), judge_answer_says(answered.answer));
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                Fleet::answer_judge(
                    &fleet,
                    &id,
                    answered.answer,
                    answered.asked_at,
                    answered.note,
                    answered.mode,
                )
                .await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.job_answered(&asked_of, says);
        self.summarised(&job).await
    }
}
