//! Fleet holding a Job to how the owner works with it (`crate::ways`): the note a change tells a
//! running Job, the `waiting_for` that sends back a question with no options or an ask not worded as a question, and `all_done` held until a
//! picture is shown. A question read out of a reply is not sent back: in its first day 40 of 51 such
//! notices were disputed, since what a reply says is a guess and the item and the call are not.
//!
//! **Each check is here because a note alone is a rule an agent may skip.** What the note says is
//! checked by the call it is about, and the refusal names the item or the call.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{WaitingActKind, WaitingInput};
use store::Holder;

use crate::daemon::Fleet;
use crate::ways;

/// An `all_done` refused for a Job that has shown no picture, where the owner looks. A 409.
pub(super) const PICTURE_OWED: &str = "fleet.picture_owed";
/// A `waiting_for` refused for a question that carries no options, or more than four, where the owner
/// answers by buttons. A 422.
pub(super) const OPTIONS_OWED: &str = "fleet.session_waiting_options";

/// A `waiting_for` refused for an item that asks him to choose or approve and is not worded as a question. A 422.
pub(super) const QUESTION_OWED: &str = "fleet.session_waiting_question";

/// The most options a question may offer.
const MOST_OPTIONS: usize = 4;

/// Whether `item` is a question for him to answer: not an approval, a procedure, or an act that runs,
/// walks, opens a pull request or sends a message.
fn is_a_question(item: &WaitingInput) -> bool {
    !item.approves
        && item.steps.is_empty()
        && item.act.as_ref().is_none_or(|act| act.kind == WaitingActKind::Answer)
}

/// What the agent reads when `item` is a question the owner cannot answer by buttons, or `None` where
/// it is anything else or offers two to four options.
pub(super) fn options_owed(item: &WaitingInput) -> Option<String> {
    let offered = item.options.len();
    (is_a_question(item) && (offered == 0 || offered > MOST_OPTIONS)).then(|| ways::options_refusal(item.id.trim(), offered))
}

/// What the agent reads when `item` asks him to choose or approve and its text is not a question in plain words,
/// which titles the item on his review sheet. `None` for any other item.
pub(super) fn question_owed(item: &WaitingInput) -> Option<String> {
    let answered = item.act.as_ref().is_none_or(|act| act.kind == WaitingActKind::Answer);
    let asks = item.approves || (answered && (!item.options.is_empty() || item.act.is_some()));
    let why = ipc::question_checked(Some(&item.text), &format!("the text of `{}`", item.id.trim())).err()?;
    asks.then_some(why)
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
    /// The note of how the owner works, as his preferences say it now.
    pub(crate) fn ways_note(&self) -> String {
        ways::note(&self.preferences_in_force())
    }

    /// The note the Operator starts with: how he takes things in, without a Job's own acts.
    pub(crate) fn operator_ways_note(&self) -> String {
        ways::operator_note(&self.preferences_in_force())
    }

    /// Tell every session with a running process how he works now, as Fleet's word: a Job `job_note`,
    /// the Operator `operator_note`. **Only one running**: one that is not reads its note as it starts.
    /// News that asks nothing, so a Job's `all_done` stands. A note that did not change is `None`.
    pub(crate) async fn ways_told(self: &Arc<Self>, job_note: Option<&str>, operator_note: Option<&str>) {
        let live = self.store().lock().await.live_session_ids().unwrap_or_default();
        for id in live {
            let (running, operator) = {
                let host = self.hosts().of(&id);
                let state = host.state();
                (state.process.is_some(), state.operator)
            };
            let note = if operator { operator_note } else { job_note };
            if let (true, Some(note)) = (running, note) {
                let _ = self.told_news_by_fleet(&id, note, ipc::Notice::Taught).await;
            }
        }
    }

    /// What `all_done` is told while the owner looks and the Job has shown no picture: `show_sketch`,
    /// `show_window` or `show_file` of an image. A picture withdrawn was shown, so it counts. **A Side
    /// Job with no plan owes none**: it answered a question, and a diagram of an answer is noise.
    pub(super) fn picture_owed_text(&self, store: &store::Store, id: &str) -> Result<Option<String>, Refusal> {
        if !ways::picture_owed(&self.preferences_in_force()) {
            return Ok(None);
        }
        let side = store.hosting(id).map_err(|why| self.ledger_fault(why))?.is_some_and(|hosting| hosting.side_job);
        if side && store.session_plan(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Ok(None);
        }
        let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
        let shown = !super::picture_shown::pictures_shown(id, &held, &[]).is_empty();
        Ok((!shown).then(|| ways::PICTURE_REFUSAL.to_string()))
    }

    /// `waiting_for` refused, where the owner answers by buttons, for the first question with no options
    /// or too many.
    pub(super) fn options_refused(&self, items: &[WaitingInput]) -> Result<(), Refusal> {
        if !ways::buttons(&self.preferences_in_force()) {
            return Ok(());
        }
        match items.iter().find_map(options_owed) {
            Some(why) => Err(self.hosted_refusal(OPTIONS_OWED, &why)),
            None => Ok(()),
        }
    }

    /// `waiting_for` refused for the first item that asks him to choose or approve with no question.
    pub(super) fn questions_refused(&self, items: &[WaitingInput]) -> Result<(), Refusal> {
        match items.iter().find_map(question_owed) {
            Some(why) => Err(self.hosted_refusal(QUESTION_OWED, &why)),
            None => Ok(()),
        }
    }
}
