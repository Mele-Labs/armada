//! The owner's answers on a Job's handoff card that start nothing, kept on the Job's ledger, and what
//! the Context rule does with them and with the Large context setting.
//! `docs/concepts/session/a-handed-off-job.md`.
//!
//! **Read off the card Fleet holds, never off the agent's word.** The asking door calls in here with
//! what the owner pressed. The Context rule (`usage_following.rs`) reads the ledger back: a Job's own
//! answer wins over the setting for that Job.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{AskedQuestion, QuestionAnswer, SessionAskState, SessionRow, SessionVoice};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

/// On the Job's ledger once the owner chose [`ipc::HANDOFF_QUIET`]: `spent`, a fact that stays.
pub(crate) const QUIET: &str = "handoff_quiet";
/// On the Job's ledger once the owner chose a figure after [`ipc::HANDOFF_REMIND`]: `standing` until
/// the Job's context passes it and it is told again. The target is the figure in tokens.
pub(crate) const REMIND_AT: &str = "handoff_remind_at";

/// Which of the handoff answers the owner pressed on a card, in any question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandoffAnswer {
    Yes,
    Quiet,
    Remind,
    Other,
}

impl HandoffAnswer {
    pub(crate) fn of(answers: &[QuestionAnswer]) -> HandoffAnswer {
        let chose = |label: &str| answers.iter().any(|one| one.chosen.iter().any(|chosen| chosen == label));
        if chose(ipc::HANDOFF_YES) {
            HandoffAnswer::Yes
        } else if chose(ipc::HANDOFF_QUIET) {
            HandoffAnswer::Quiet
        } else if chose(ipc::HANDOFF_REMIND) {
            HandoffAnswer::Remind
        } else {
            HandoffAnswer::Other
        }
    }
}

/// How Bridge opens the person's answer to a question kept for a walk, a line a question
/// (`lapsedAnswerText`, `packages/screens/src/sessions-wire.ts`).
const ANSWERED_LATER: &str = "Answering your earlier question";

/// What the person chose on the kept `questions`, read off their message: each label one the
/// question offered, so words of their own choose nothing.
fn answered_later(questions: &[AskedQuestion], text: &str) -> Vec<QuestionAnswer> {
    questions
        .iter()
        .filter_map(|asked| {
            let opening = format!("{ANSWERED_LATER} \"{}\": ", asked.question);
            let line = text.lines().find_map(|line| line.strip_prefix(opening.as_str()))?;
            // Joined with ", ", which a label may hold itself.
            let chosen = asked
                .options
                .iter()
                .map(|option| option.label.as_str())
                .filter(|label| {
                    line == *label
                        || line.starts_with(&format!("{label}, "))
                        || line.ends_with(&format!(", {label}"))
                        || line.contains(&format!(", {label}, "))
                })
                .map(String::from)
                .collect();
            Some(QuestionAnswer { question: asked.question.clone(), chosen })
        })
        .collect()
}

/// The last ask with questions that nobody answered in time, that was skipped, or that was kept for a
/// walk, while nothing the person said and no newer question came after it. A kept one outlasts what
/// he says; a message answering it clears it. Bridge's `lapsedOf` (`packages/screens/src/sessions-wire.ts`)
/// is the same rule: keep the two in step.
pub(crate) fn lapsed_question(rows: &[SessionRow]) -> Option<Vec<AskedQuestion>> {
    let mut said = false;
    for row in rows.iter().rev() {
        match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => {
                if text.starts_with(ANSWERED_LATER) {
                    return None;
                }
                said = true;
            }
            SessionRow::Ask { ask, state, .. } if !ask.questions.is_empty() => {
                let lapsed = *state == SessionAskState::Kept
                    || (!said && matches!(state, SessionAskState::Unanswered | SessionAskState::Refused));
                return lapsed.then(|| ask.questions.clone());
            }
            _ => {}
        }
    }
    None
}

/// What the Context rule does for one Job whose context is over the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextAct {
    /// Tell the Job to ask on its card. `reminded` where it is the reminder the owner asked for, which
    /// is told even where the rule told it already and has not seen it fall under.
    Tell { reminded: bool },
    /// Carry on in a new Job with no card: the Large context setting's standing consent.
    Hand,
    Nothing,
}

/// The Context rule's act for a Job at `tokens`, by its own answers first and then the setting.
/// `due` is whether the rule's own watch would tell it now.
pub(crate) fn context_act(quiet: bool, remind_at: Option<u64>, setting: &str, tokens: f64, due: bool) -> ContextAct {
    if quiet {
        return ContextAct::Nothing;
    }
    if let Some(figure) = remind_at {
        return match tokens > figure as f64 {
            true => ContextAct::Tell { reminded: true },
            false => ContextAct::Nothing,
        };
    }
    match (due, setting) {
        (false, _) => ContextAct::Nothing,
        (true, keys::HANDOFF_NEVER) => ContextAct::Nothing,
        (true, keys::HANDOFF_AUTOMATICALLY) => ContextAct::Hand,
        (true, _) => ContextAct::Tell { reminded: false },
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
    /// The Context rule's act for Job `session` at `tokens`.
    pub(crate) async fn context_act_of(&self, session: &str, tokens: f64, due: bool) -> ContextAct {
        let setting: String = self.machine_settings().now().get(keys::PLAN_CONTEXT_HANDOFF);
        let held = self.store().lock().await.attachments_of(&Holder::session(session)).unwrap_or_default();
        let quiet = held.iter().any(|one| one.kind == QUIET && one.state == AttachmentState::Spent);
        let remind_at = held
            .iter()
            .find(|one| one.kind == REMIND_AT && one.state == AttachmentState::Standing)
            .and_then(|one| one.target.parse::<u64>().ok());
        context_act(quiet, remind_at, &setting, tokens, due)
    }

    /// The owner chose not to be asked again on this Job.
    pub(crate) async fn handoff_quieted(&self, session: &str) {
        self.handoff_answer_kept(session, QUIET, session, AttachmentState::Spent).await;
    }

    /// The owner chose to be asked again once the Job's context passes `tokens`. An earlier figure is
    /// given back.
    pub(crate) async fn handoff_remind_kept(&self, session: &str, tokens: u64) {
        self.handoff_answer_kept(session, REMIND_AT, &tokens.to_string(), AttachmentState::Standing).await;
    }

    /// The reminder was told: the figure is spent, and the rule's watch takes over again.
    pub(crate) async fn handoff_reminded(&self, session: &str) {
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.settle_held(&Holder::session(session), Some(REMIND_AT), AttachmentState::Spent, &now);
    }

    async fn handoff_answer_kept(&self, session: &str, kind: &str, target: &str, state: AttachmentState) {
        let Ok((row, _)) = self.session_and_hosting(session).await else { return };
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.attach(
            &KeptAttachment {
                holder: Holder::session(session),
                kind: kind.to_string(),
                manifest_id: row.manifest_id.unwrap_or_default(),
                target: target.to_string(),
                state,
                detail: Default::default(),
                since: now.clone(),
                changed_at: now,
            },
            true,
        );
        let _ = self.published_hosted(session).await;
    }

    /// The person answered a question kept for a walk, or one nobody answered in time, with a message, from Needs you: its handoff
    /// answer is run as the card's is in the permission door. Read before the message's row is put.
    /// `true` where the Job hands on, so the message is not sent to the process the hand ends.
    pub(crate) async fn handoff_answered_later(self: &Arc<Self>, session: &str, text: &str) -> bool {
        if !text.starts_with(ANSWERED_LATER) {
            return false;
        }
        let Ok(rows) = self.rows_of(session).await else { return false };
        let Some(questions) = lapsed_question(&rows) else { return false };
        match HandoffAnswer::of(&answered_later(&questions, text)) {
            HandoffAnswer::Yes if !self.handing_on_already(session).await => {
                self.handed_on_apart(session, Self::handoff_brief(&questions));
                true
            }
            HandoffAnswer::Quiet => {
                self.handoff_quieted(session).await;
                false
            }
            _ => false,
        }
    }

    /// Whether Job `session` has ended, is handing on already, or has handed on.
    pub(crate) async fn handing_on_already(&self, session: &str) -> bool {
        let ended = self.session_and_hosting(session).await.map_or(true, |(row, _)| row.state == store::SessionState::Ended);
        let held = self.store().lock().await.attachments_of(&Holder::session(session)).unwrap_or_default();
        ended
            || held.iter().any(|one| {
                (one.kind == super::replacing::STARTING && one.state == AttachmentState::Standing)
                    || (one.kind == super::replacing::REPLACED_BY && one.state == AttachmentState::Spent)
            })
    }
}
