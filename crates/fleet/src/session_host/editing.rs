//! A message the person sent, edited: `edit_session_message`. Since 10 Nov 2026.
//!
//! **Where Fleet still holds the turn, the words change in place**: the agent has not been sent them,
//! so it reads only the new ones and the row is not marked. **Anywhere else it is a correction**:
//! what is written to the agent's input is not taken back, so the row keeps the new words marked
//! `edited_at` and the agent is sent a turn of Fleet's saying what changed. That turn is no row of the
//! thread, as a Resume's line is none: the edited bubble is what the person sees.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{EditSessionMessage, SessionRecord, SessionRow, SessionVoice};

use super::start_card::{first_turn_of, job_turn};
use crate::daemon::Fleet;

/// An edit of a row the thread does not hold, or that is not a message. A 422.
const SESSION_ROW_UNKNOWN: &str = "fleet.session_row_unknown";
/// An edit of a message that is not the person's own. A 422.
const SESSION_ROW_NOT_HIS: &str = "fleet.session_row_not_his";

/// What the agent reads of an edit it was sent the words of already.
fn correction(before: &str, now: &str) -> String {
    format!(
        "The owner edited a message they sent earlier.\n\nBefore:\n{}\n\nNow:\n{}\n\nAct on the new version where it changes anything.",
        quoted(before),
        quoted(now),
    )
}

/// `words` with each line marked as a quotation.
fn quoted(words: &str) -> String {
    words.lines().map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n")
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
    /// `edit_session_message`. **Refused with nothing changed**, and a no-op that succeeds where the
    /// words are the row's own.
    pub(crate) async fn session_message_edited(
        self: &Arc<Self>,
        edit: EditSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        let id = edit.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                super::serving::SESSION_COMMAND_REFUSED,
                &format!("job {id} runs in a terminal, and only a hosted job's messages are edited from here"),
            ));
        }
        let (session, _) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let words = edit.text.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(super::serving::MESSAGE_EMPTY, "an edited message needs words"));
        }
        let unknown = || self.hosted_refusal(SESSION_ROW_UNKNOWN, &format!("job {id} has no message {}", edit.row_id));
        let row = self.rows_of(&id).await?.into_iter().find(|row| row.id() == edit.row_id).ok_or_else(unknown)?;
        let (from, before) = match &row {
            SessionRow::Message { from, text, .. } => (from.clone(), text.clone()),
            _ => return Err(unknown()),
        };
        if from != SessionVoice::You {
            return Err(self.hosted_refusal(
                SESSION_ROW_NOT_HIS,
                &format!("message {} was not the owner's, and only the owner's words are edited", edit.row_id),
            ));
        }
        if before == words {
            return self.published_hosted(&id).await;
        }
        if self.held_turn_changed(&id, &edit.row_id, &words).await? {
            self.edited_row_put(&id, row, &words, false).await;
        } else {
            self.edited_row_put(&id, row, &words, true).await;
            let held = job_turn(&words, correction(&before, &words));
            let runtime = self.hosts().of(&id);
            // Behind the turns held for a proposal, as any later turn is.
            let held = {
                let mut state = runtime.state();
                match state.proposal.holds_back() {
                    true => {
                        state.proposal.held.push_back(held);
                        None
                    }
                    false => Some(held),
                }
            };
            if let Some(held) = held {
                self.delivered_in_order(&id, held).await;
            }
        }
        self.published_hosted(&id).await
    }

    /// The row put again with `words`. **A correction marks it**; a change in place does not.
    async fn edited_row_put(&self, id: &str, mut row: SessionRow, words: &str, marked: bool) {
        let at = self.instant();
        let runtime = self.hosts().of(id);
        let still_queued = {
            let mut state = runtime.state();
            state.activity.writing.person_edited(row.id(), words);
            state.call_rows.edited(row.id(), words, &at)
        };
        if let SessionRow::Message { text, edited_at, queued, .. } = &mut row {
            *text = words.to_string();
            if marked {
                *edited_at = Some(at);
            }
            // The held row in `call_rows` is the one put again when the agent reads it, so it is the one that says.
            *queued = *queued && still_queued;
        }
        self.row_put(id, row).await;
    }

    /// Replace the words of a turn Fleet holds for `row`: behind a proposal's reading, after a process
    /// that would not start, or for the Resume of a paused Job. `false` where no turn is held for it.
    /// **What Fleet added to the words goes**, as it does where the person edits the first turn.
    async fn held_turn_changed(&self, id: &str, row: &str, words: &str) -> Result<bool, Refusal> {
        let mut changed = false;
        {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            let state = &mut *state;
            let failed = state.start_failed.iter_mut().flat_map(|failed| failed.held.iter_mut());
            for held in state.proposal.held.iter_mut().chain(failed) {
                if held.row.as_deref() == Some(row) {
                    held.text = words.to_string();
                    held.instead = None;
                    changed = true;
                }
            }
        }
        if changed {
            return Ok(true);
        }
        let kept = self
            .store()
            .lock()
            .await
            .amend_held_later_turns(id, |body| {
                let mut turn = first_turn_of(body)?;
                if turn.row.as_deref() != Some(row) {
                    return None;
                }
                turn.text = words.to_string();
                turn.instead = None;
                ipc::encode(&turn).ok()
            })
            .map_err(|why| self.ledger_fault(why))?;
        Ok(kept > 0)
    }
}
