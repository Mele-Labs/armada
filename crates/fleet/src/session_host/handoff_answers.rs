//! The owner's handoff card and his answers on it that start nothing, kept on the Job's ledger, and
//! what the Context rule does with them and with the Large context setting.
//! `docs/concepts/session/a-handed-off-job.md`.
//!
//! **One card, one press.** The Context rule tells the Job to send a brief with `handoff_brief`, and
//! that brief raises a `handoff` row Fleet owns, waiting as `handoff:<row>`. His press settles it, and
//! the Job is told nothing. **Read off the row Fleet holds, never off the agent's word.** The Context
//! rule (`usage_following.rs`) reads the ledger back: a Job's own answer wins over the setting for
//! that Job.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use ipc::{AskedQuestion, HandoffBrief, HandoffPick, QuestionAnswer, SessionAskState, SessionRecord, SessionRow, SessionVoice, WaitingOption};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

/// On the Job's ledger once the owner pressed [`ipc::HANDOFF_QUIET`]: `spent`, a fact that stays.
pub(crate) const QUIET: &str = "handoff_quiet";
/// On the Job's ledger once the owner pressed a "Remind at" figure: `standing` until the Job's
/// context passes it and it is told again. The target is the figure in tokens.
pub(crate) const REMIND_AT: &str = "handoff_remind_at";
/// On the Job's ledger once the Context rule told it to send a brief: `standing` until a brief raises
/// the card, `spent` from then. `handoff_brief` is refused without a standing one.
pub(crate) const ASKED: &str = "handoff_asked";

/// The id a handoff card's waiting item goes by, from its row's.
pub(crate) const HANDOFF_PREFIX: &str = "handoff:";
/// What the card asks, the waiting item's text.
pub(crate) const CARD_ASKS: &str = "Carry on in a new Job?";

/// A `handoff_brief` that places no Job Fleet knows.
const HANDOFF_UNNAMED: &str = "fleet.handoff_unnamed";
/// A `handoff_brief` from a Job the Context rule did not ask, or one whose card is raised already.
const HANDOFF_NOT_ASKED: &str = "fleet.handoff_not_asked";
/// A `handoff_brief` with no words in it.
const HANDOFF_BRIEF_EMPTY: &str = "fleet.handoff_brief_empty";

/// The card's options, in the order a `choice` indexes them: carry on, stay, don't ask again, then
/// a "Remind at" for each figure.
pub(crate) fn card_options(remind_at: &[u64]) -> Vec<WaitingOption> {
    let option = |label: String| WaitingOption { label, description: None, picture: None };
    [ipc::HANDOFF_YES, ipc::HANDOFF_STAY, ipc::HANDOFF_QUIET]
        .into_iter()
        .map(String::from)
        .chain(remind_at.iter().map(|tokens| format!("{} {}", ipc::HANDOFF_REMIND, ipc::handoff_figure(*tokens))))
        .map(option)
        .collect()
}

/// What a `choice` on the card picks. `None` for an index past its options.
pub(crate) fn pick_of(choice: u32, remind_at: &[u64]) -> Option<HandoffPick> {
    match choice {
        0 => Some(HandoffPick::CarryOn),
        1 => Some(HandoffPick::Stay),
        2 => Some(HandoffPick::Quiet),
        more => remind_at.get(more as usize - 3).map(|tokens| HandoffPick::RemindAt { tokens: *tokens }),
    }
}

/// How Bridge opens the person's answer to a question kept for a walk, a line a question
/// (`lapsedAnswerText`, `packages/screens/src/sessions-wire.ts`).
pub(crate) const ANSWERED_LATER: &str = "Answering your earlier question";

/// What the person chose on the kept `questions`, read off their message: each label one the
/// question offered, so words of their own choose nothing.
pub(crate) fn answered_later(questions: &[AskedQuestion], text: &str) -> Vec<QuestionAnswer> {
    questions
        .iter()
        .filter_map(|asked| {
            let opening = format!("{ANSWERED_LATER} \"{}\": ", asked.question);
            let line = text.lines().find_map(|line| line.strip_prefix(opening.as_str()))?;
            Some(QuestionAnswer { question: asked.question.clone(), chosen: super::asking::labels_in(line, &asked.options) })
        })
        .collect()
}

/// The questions of every ask that nobody answered in time, that was skipped, or that was kept
/// (for a walk, or because nobody answered inside the hold), oldest first. A kept one outlasts what
/// the person says; a skipped one is cleared by anything he says after it; a message answering them
/// clears all. A question asked again later is left to the later ask. Bridge's `lapsedOf`
/// (`packages/screens/src/sessions-wire.ts`) is the same rule: keep the two in step.
pub(crate) fn lapsed_question(rows: &[SessionRow]) -> Option<Vec<AskedQuestion>> {
    let mut said = false;
    let mut later = BTreeSet::new();
    let mut lapsed: Vec<AskedQuestion> = Vec::new();
    for row in rows.iter().rev() {
        match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => {
                if text.starts_with(ANSWERED_LATER) {
                    break;
                }
                said = true;
            }
            SessionRow::Ask { ask, state, .. } if !ask.questions.is_empty() => {
                let kept = *state == SessionAskState::Kept
                    || (!said && matches!(state, SessionAskState::Unanswered | SessionAskState::Refused));
                for asked in &ask.questions {
                    if kept && !later.contains(&asked.question) {
                        lapsed.push(asked.clone());
                    }
                    later.insert(asked.question.clone());
                }
            }
            _ => {}
        }
    }
    lapsed.reverse();
    (!lapsed.is_empty()).then_some(lapsed)
}

/// What the Context rule does for one Job whose context is over the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextAct {
    /// Tell the Job to send its brief for the card. `reminded` where it is the reminder the owner asked for, which
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

    /// The Job was told to send its brief: `handoff_brief` takes one from it now.
    pub(crate) async fn handoff_asked(&self, session: &str) {
        self.handoff_answer_kept(session, ASKED, session, AttachmentState::Standing).await;
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
    /// `handoff_brief`: the Job sends the brief the Context rule asked it for, and the card is raised
    /// with its context, the limit and the figures. **One brief a telling**: the `handoff_asked` row is
    /// spent in the same write that finds it standing, so a second brief is refused.
    pub(crate) async fn handoff_brief_sent(&self, caller: Option<api::Caller>, sent: HandoffBrief) -> Result<SessionRecord, Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| sent.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(HANDOFF_UNNAMED, "a job to send the brief for"))?;
        let brief: String = sent.brief.trim().chars().take(super::replacing::BRIEF_CHARS).collect();
        let brief = brief.trim_end().to_string();
        if brief.is_empty() {
            return Err(self.hosted_refusal(HANDOFF_BRIEF_EMPTY, "a brief needs its words: where the work is, what is open, what you hold"));
        }
        let limits = crate::usage_rules::Limits::of(&self.machine_settings().now());
        let tokens = {
            let mut store = self.store().lock().await;
            if store.session(&id).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(self.hosted_refusal(HANDOFF_UNNAMED, "a job Fleet knows"));
            }
            let now = self.now().as_str().to_string();
            let asked = store.settle_held(&Holder::session(&id), Some(ASKED), AttachmentState::Spent, &now).map_err(|why| self.ledger_fault(why))?;
            if !asked {
                return Err(self.hosted_refusal(
                    HANDOFF_NOT_ASKED,
                    "this job was not asked for a brief. Send one only when Fleet tells you your context is past the limit, once for each telling",
                ));
            }
            let turns = store.turns_for_rules(&id, limits.main_turns_needed()).unwrap_or_default();
            crate::usage_rules::context_now(&turns).unwrap_or_default()
        };
        let row = SessionRow::Handoff {
            id: self.row_id(&id),
            at: self.instant(),
            tokens,
            limit: limits.context_tokens.unwrap_or_default(),
            brief,
            remind_at: ipc::handoff_remind_ladder(tokens).to_vec(),
            answer: None,
        };
        self.row_put(&id, row).await;
        self.published_hosted(&id).await
    }

    /// The figures handoff card `row` of Job `session` offers, which its `choice` indexes past the first three.
    pub(crate) async fn handoff_figures(&self, session: &str, row: &str) -> Vec<u64> {
        let rows = self.rows_of(session).await.unwrap_or_default();
        rows.into_iter()
            .find_map(|one| match one {
                SessionRow::Handoff { id, remind_at, .. } if id == row => Some(remind_at),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// His press on the handoff card `row` of Job `session`. **Only Carry on starts anything**, and the
    /// Job is told nothing whatever he pressed. The row is kept answered, so the item leaves his list.
    pub(crate) async fn handoff_answered(self: &Arc<Self>, session: &str, row: &str, pick: HandoffPick) -> Result<SessionRecord, Refusal> {
        let found = self.rows_of(session).await?.into_iter().find_map(|one| match one {
            SessionRow::Handoff { id, at, tokens, limit, brief, remind_at, answer: None } if id == row => {
                Some((brief.clone(), SessionRow::Handoff { id, at, tokens, limit, brief, remind_at, answer: Some(pick) }))
            }
            _ => None,
        });
        let Some((brief, answered)) = found else { return Err(self.settled_already()) };
        self.row_put(session, answered).await;
        match pick {
            HandoffPick::CarryOn if !self.handing_on_already(session).await => self.handed_on_apart(session, Some(brief)),
            HandoffPick::CarryOn | HandoffPick::Stay => {}
            HandoffPick::Quiet => self.handoff_quieted(session).await,
            HandoffPick::RemindAt { tokens } => self.handoff_remind_kept(session, tokens).await,
        }
        self.published_hosted(session).await
    }
}
