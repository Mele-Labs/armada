//! A Drone asking several decisions in one ask, and the answers coming back.
//! The single question is [`crate::questioning`]'s, and this is its second shape.
//!
//! **One ask, one answer, no queue.** The decisions are asked together, stored
//! together on the Job and answered together, so a person is never holding half
//! an interview and a Drone is never handed half of one. An answer missing a
//! decision is refused by naming it, because a Drone that opened on two of
//! three answers would guess the third.
//!
//! **Any step may ask this way, not only the Plan step.** The owner's ask was
//! the Plan interview, but Fleet reads no step's role off its id, and
//! `records_plan` would leave out the `feature` workflow's own Plan step, whose
//! product is a note. The cost of allowing it elsewhere is a person answering
//! several things in one sitting, which is what they would do anyway.
use core_model::{Actor, Decision, DroneQuestion, JobId};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::questioning::{NotAnswered, Told};

/// [`Decided`]'s frame as it ships, `prompts.crossingDecided`'s default.
pub(crate) const DECIDED: &str =
    "WHAT A PERSON DECIDED\n\nThe Drone before you on this part asked a person several things at \
     once, and they answered each:\n\n{answers}\n\nIt stood down to wait for the answers, and \
     its work so far is on the branch you are in. Carry on this part from there and from the \
     answers. Every question here is answered and closed — do not ask any of them again.";

/// One decision and the answer to it, as the brief states it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decided {
    question: String,
    label: String,
    consequence: String,
}

/// Every decision on an answered interview, or `None` while any is open.
pub(crate) fn decided(asked: &DroneQuestion) -> Option<Vec<Decided>> {
    asked
        .decisions
        .iter()
        .map(|decision| {
            let chosen = decision.chosen()?;
            Some(Decided {
                question: decision.question.clone(),
                label: chosen.label.clone(),
                consequence: chosen.consequence.clone(),
            })
        })
        .collect()
}

/// The block, in the words of `prompts.crossingDecided`.
pub(crate) fn text(decided: &[Decided], prompts: &crate::prompts::Prompts) -> String {
    let answers = decided
        .iter()
        .enumerate()
        .map(|(at, one)| {
            format!(
                "{}. \"{}\"\n   They answered: {}\n   Which that Drone said means: {}",
                at + 1,
                one.question,
                one.label,
                one.consequence
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    prompts.fill(
        config::settings::PROMPT_CROSSING_DECIDED,
        &[("answers", &answers)],
    )
}

/// The picks checked against the decisions, as `(decision id, label)` in the
/// order the decisions were asked. **Every refusal names what is wrong**, and
/// none keeps anything.
fn checked(
    asked: &DroneQuestion,
    picks: &[ipc::ChosenDecision],
) -> Result<Vec<(String, String)>, NotAnswered> {
    let mut kept: Vec<(String, String)> = Vec::with_capacity(picks.len());
    for pick in picks {
        let Some(decision) = asked.decision(&pick.decision_id) else {
            return Err(NotAnswered::NoSuchDecision {
                decision: pick.decision_id.clone(),
            });
        };
        if kept.iter().any(|(id, _)| *id == decision.id) {
            return Err(NotAnswered::DecidedTwice {
                decision: decision.id.clone(),
            });
        }
        let Some(chosen) = decision.offering(&pick.chose) else {
            return Err(NotAnswered::NotOfferedFor {
                decision: decision.id.clone(),
                chose: pick.chose.clone(),
                offered: owned(decision),
            });
        };
        kept.push((decision.id.clone(), chosen.label.clone()));
    }
    let open: Vec<String> = asked
        .decisions
        .iter()
        .filter(|decision| !kept.iter().any(|(id, _)| *id == decision.id))
        .map(|decision| decision.id.clone())
        .collect();
    if !open.is_empty() {
        return Err(NotAnswered::Unanswered { decisions: open });
    }
    Ok(kept)
}

fn owned(decision: &Decision) -> Vec<String> {
    decision.labels().iter().map(|label| label.to_string()).collect()
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
    /// Keep a person's answers to every decision of an interview on the Job.
    ///
    /// Everything [`answer_question`](Fleet::answer_question) does and does not
    /// do, for a question that is several decisions: it starts nothing, the
    /// actor is human, and the answered event is the same one. **All of the
    /// picks or none of them** — a refused answer leaves the interview as it
    /// was, so a person can send the whole of it again.
    pub async fn answer_decisions(
        &self,
        job_id: &JobId,
        question_id: &str,
        picks: &[ipc::ChosenDecision],
    ) -> Result<Told, NotAnswered> {
        let nothing = || NotAnswered::NothingIsAsking {
            job: job_id.clone(),
        };
        let job = self.load(job_id).await.map_err(|_| nothing())?;
        let Some(asked) = job.question() else {
            return Err(nothing());
        };
        if asked.id.as_str() != question_id {
            return Err(NotAnswered::Superseded {
                job: job_id.clone(),
            });
        }
        if !job.waiting_on_an_answer() {
            return Err(nothing());
        }
        if asked.decisions.is_empty() {
            return Err(NotAnswered::NotAnInterview {
                job: job_id.clone(),
            });
        }
        let kept = checked(asked, picks)?;
        let step = asked.step.clone();
        let now = self.now();
        self.store()
            .lock()
            .await
            .record_question(&job.question_decided(&kept, now.clone()))
            .map_err(NotAnswered::NotKept)?;
        let chose = kept
            .iter()
            .map(|(_, label)| label.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        self.noted_answered(job_id, &step, question_id, Some(&chose), None);
        self.events().publish(ipc::Event::JobAsking(ipc::JobAsking {
            job_id: ipc::JobId::from(job_id),
            step_id: ipc::StepId::from(&step),
            asking: None,
            actor: Actor::Human.into(),
            at: (&now).into(),
        }));
        Ok(Told {
            job: job_id.clone(),
            step,
            chose: Some(chose),
        })
    }
}
