//! Fleet holding a Job to how the owner works with it (`crate::ways`): the note a change tells a
//! running Job, the `waiting_for` that sends back a question with no options, `all_done` held until a
//! picture is shown, and a question read out of a reply sent to be asked again with options.
//!
//! **Each check is here because a note alone is a rule an agent may skip.** What the note says is
//! checked by the call it is about, and the refusal names the item or the call.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRow, SessionVoice, WaitingActKind, WaitingInput};
use store::Holder;

use crate::daemon::Fleet;
use crate::ways;

/// An `all_done` refused for a Job that has shown no picture, where the owner looks. A 409.
pub(super) const PICTURE_OWED: &str = "fleet.picture_owed";
/// A `waiting_for` refused for a question that carries no options, or more than four, where the owner
/// answers by buttons. A 422.
pub(super) const OPTIONS_OWED: &str = "fleet.session_waiting_options";

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

    /// Tell every Job with a running process `note`, as Fleet's word. **Only a Job running**: one
    /// that is not reads the note as it starts. News that asks nothing, so a Job's `all_done` stands.
    pub(crate) async fn ways_told(self: &Arc<Self>, note: &str) {
        let live = self.store().lock().await.live_session_ids().unwrap_or_default();
        for id in live {
            let running = self.hosts().of(&id).state().process.is_some();
            if running {
                let _ = self.told_news_by_fleet(&id, note, ipc::Notice::Taught).await;
            }
        }
    }

    /// What `all_done` is told while the owner looks and the Job has shown no picture: `show_sketch`,
    /// `show_window` or `show_file` of an image. A picture withdrawn was shown, so it counts.
    pub(super) fn picture_owed_text(&self, store: &store::Store, id: &str) -> Result<Option<String>, Refusal> {
        if !ways::picture_owed(&self.preferences_in_force()) {
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

    /// A reply ended on `said`. Where the owner answers by buttons and it asks him something in words,
    /// the Job is told once to ask it again with options. **Not when the Job asked through a tool**
    /// (a question card open, a list kept), and **not for the reply to this very notice**, so a Job
    /// that still asks in words is not told again until he has written.
    pub(crate) async fn reply_asked_again(self: &Arc<Self>, id: &str, said: &str) {
        if !ways::buttons(&self.preferences_in_force()) || super::asks::asking_line(said).is_none() {
            return;
        }
        if self.hosts().of(id).state().asked.is_some() {
            return;
        }
        if self.store().lock().await.waiting_for(id).is_ok_and(|kept| !kept.is_empty()) {
            return;
        }
        let Ok(rows) = self.rows_of(id).await else { return };
        let last_word = rows.iter().rev().find_map(|row| match row {
            SessionRow::Message { from, notice, .. } if *from != SessionVoice::Agent => Some(notice.as_deref()),
            _ => None,
        });
        if last_word.flatten() == Some(ipc::Notice::ReplyClaim.word()) {
            return;
        }
        let _ = self.told_by_fleet(id, ways::ASK_AGAIN, ipc::Notice::ReplyClaim).await;
    }
}
