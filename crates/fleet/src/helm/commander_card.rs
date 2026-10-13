//! The card a Job's Commander puts to the owner, and his answer. `#2166`;
//! `docs/concepts/commander.md`, *The card*.
//!
//! **A Drone's question, one subject over.** It is kept in `commander_cards`
//! and waits however long he takes. Nothing holds a call open for it, so Fleet
//! restarting loses nothing and silence is never a refusal. **While one is out
//! the Job counts as waiting on him** (`JobSummary::asking`), and that is the
//! whole of what it moves: a running Job keeps running, and an escalated one
//! stays escalated with the card beside it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Job, JobId};
use ipc::{
    AnswerCommanderCard, AskTheOwner, CardBy, CardId, CardOption, CommanderCard,
    CommanderCardAnswered, CommanderCardChanged, Instant, WireError,
};
use store::{CardAnswered, CardRaised, CardRaiser, NewCard, StoredCard};

use crate::daemon::Fleet;

/// The Job has a card out already.
const CARD_OUT: &str = "fleet.commander_card_out";
/// The card named is not the one out.
const CARD_NOT_OUT: &str = "fleet.commander_card_not_out";
/// The card did not offer that option.
const CARD_NO_OPTION: &str = "fleet.commander_card_no_option";
/// The Job is over, so nobody is waiting on an answer for it.
const JOB_OVER: &str = "fleet.commander_job_over";
/// The store would not read or write the card.
const CARD_UNREADABLE: &str = "fleet.commander_card_unreadable";

/// A card as the wire carries it. An option's id is its one-based place.
pub(crate) fn wired(card: &StoredCard) -> CommanderCard {
    CommanderCard {
        card_id: CardId::carried(&card.card_id),
        job_id: ipc::JobId::from(&card.job_id),
        by: match card.raised_by {
            CardRaiser::Commander => CardBy::Commander,
            CardRaiser::Fleet => CardBy::Fleet,
        },
        raised_at: Instant::from(&card.raised_at),
        text: card.text.clone(),
        reach: card.reach.clone(),
        question: card.question.clone(),
        options: card
            .options
            .iter()
            .enumerate()
            .map(|(at, label)| CardOption {
                id: (at + 1).to_string(),
                label: label.clone(),
            })
            .collect(),
    }
}

/// The words a Commander is woken with when the owner answers: the question,
/// and the label he picked.
fn answered_in_words(card: &StoredCard, label: &str) -> String {
    format!("The owner answered your card, \"{}\": {label}", card.question)
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
    fn card_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(
            CARD_UNREADABLE,
            format!("a Commander's card could not be read or written: {why}"),
            self.run_id(),
        ))
    }

    /// `ask_the_owner`: the Commander's card, kept and put out.
    pub(super) async fn commander_card_raised(
        self: Arc<Self>,
        job: api::Resolved,
        asked: AskTheOwner,
    ) -> Result<CommanderCard, Refusal> {
        let record = self
            .load(&job.id().to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let stored = self
            .card_put_out(
                &record,
                CardRaiser::Commander,
                asked.text.as_str(),
                asked.reach.as_str(),
                asked.question.as_str(),
                asked.options.labels(),
            )
            .await?;
        Ok(wired(&stored))
    }

    /// Put a card out for a Job, **or say which one is out already**. The one
    /// writer, for the Commander's own card and the one Fleet raises where the
    /// loop bound stopped it (`crate::commanding_wakes`).
    pub(crate) async fn card_put_out(
        &self,
        record: &Job,
        by: CardRaiser,
        text: &str,
        reach: &str,
        question: &str,
        options: &[String],
    ) -> Result<StoredCard, Refusal> {
        let about = ipc::JobId::from(record.id());
        if record.status().is_terminal() {
            return Err(Refusal::IllegalMove(
                WireError::raised(
                    JOB_OVER,
                    "this Job is over, so nobody is waiting on an answer for it",
                    self.run_id(),
                )
                .about_job(about),
            ));
        }
        let card = NewCard {
            card_id: self.mint().ulid().as_str().to_string(),
            raised_by: by,
            raised_at: self.now(),
            text: text.to_string(),
            reach: reach.to_string(),
            question: question.to_string(),
            options: options.to_vec(),
        };
        let raised = self
            .store()
            .lock()
            .await
            .raise_commander_card(record.id(), &card)
            .map_err(|why| self.card_fault(why))?;
        match raised {
            CardRaised::AlreadyOut(out) => Err(Refusal::IllegalMove(
                WireError::raised(
                    CARD_OUT,
                    format!(
                        "a card is out already, asking \"{}\". Wait for the owner's answer; \
                         Fleet wakes you with it",
                        out.question
                    ),
                    self.run_id(),
                )
                .about_job(about),
            )),
            CardRaised::Out => {
                let stored = StoredCard {
                    card_id: card.card_id,
                    job_id: record.id().clone(),
                    raised_by: by,
                    raised_at: card.raised_at,
                    text: card.text,
                    reach: card.reach,
                    question: card.question,
                    options: card.options,
                    chose: None,
                    answered_at: None,
                };
                self.noted_setting(
                    record.id(),
                    "the Job's Commander put a card out to the owner",
                    &[
                        ("card", stored.card_id.clone()),
                        ("by", by.as_wire().to_string()),
                        ("question", stored.question.clone()),
                    ],
                )
                .await;
                self.publish(ipc::Event::CommanderCard(CommanderCardChanged {
                    job_id: about,
                    card: Some(wired(&stored)),
                }));
                self.sleep_carded(record, &stored).await;
                Ok(stored)
            }
        }
    }

    /// `answer_commander_card`: his pick, kept, and the Commander woken with it.
    pub(super) async fn commander_card_answered(
        self: Arc<Self>,
        job: api::Resolved,
        picked: AnswerCommanderCard,
    ) -> Result<CommanderCardAnswered, Refusal> {
        let id = job.id().to_domain();
        let recorded = self
            .store()
            .lock()
            .await
            .answer_commander_card(&id, picked.card_id.as_str(), &picked.option_id, &self.now())
            .map_err(|why| self.card_fault(why))?;
        let card = match recorded {
            CardAnswered::Recorded(card) => card,
            CardAnswered::NotOut => {
                return Err(Refusal::IllegalMove(
                    WireError::raised(
                        CARD_NOT_OUT,
                        "that card is not the one out: it was answered, or a newer one replaced it",
                        self.run_id(),
                    )
                    .about_job(job.id()),
                ))
            }
            CardAnswered::NoSuchOption(card) => {
                return Err(Refusal::IllegalMove(
                    WireError::raised(
                        CARD_NO_OPTION,
                        format!(
                            "the card asking \"{}\" offered no option {}",
                            card.question, picked.option_id
                        ),
                        self.run_id(),
                    )
                    .about_job(job.id()),
                ))
            }
        };
        self.publish(ipc::Event::CommanderCard(CommanderCardChanged {
            job_id: job.id(),
            card: None,
        }));
        let label = card.label_of(&picked.option_id).unwrap_or_default();
        let reason = answered_in_words(&card, label);
        let woke = Arc::clone(&self).wake_commander(&id, &reason).await.is_ok();
        Ok(CommanderCardAnswered {
            job_id: job.id(),
            card_id: picked.card_id,
            option_id: picked.option_id,
            woke,
        })
    }

    /// The card this Job has out, as the wire carries it. **None for a Job that
    /// is over**, which nobody is waiting on, and where the store will not say.
    pub(crate) async fn card_out_of(&self, job: &Job) -> Option<CommanderCard> {
        if job.status().is_terminal() {
            return None;
        }
        let out = self.store().lock().await.open_commander_card(job.id());
        out.ok().flatten().map(|card| wired(&card))
    }

    /// Whether any Job in `jobs` has a card out, by Job: one read for a whole
    /// board.
    pub(crate) async fn jobs_with_a_card_out(&self) -> std::collections::BTreeSet<JobId> {
        self.store()
            .lock()
            .await
            .open_commander_cards()
            .map(|cards| cards.into_iter().map(|card| card.job_id).collect())
            .unwrap_or_default()
    }
}
