//! Whether a session's walk window is ready: the page done and waiting on the person, or still being
//! changed. `docs/concepts/session.md`, *Showing a window*.
//!
//! **Kept on the window's artifact as `detail.working = "true"`; absent is ready.** The artifact is
//! per address and the session's record already reaches Bridge whole, so the bar reads it live and a
//! window reopened from the ledger reads it too. **Detected, not declared**: the mod reports the
//! session's own file write (`wrote`) and every window it showed is working; its turn ending
//! (`turn_completed`) or its showing the page again makes them ready; and a note taken in the window
//! is working too, since the session will now change the page.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Sessions;
use ipc::{SessionFact, SessionId, SessionReport};
use store::{AttachmentState, Holder, KeptAttachment, Store};

use crate::daemon::Fleet;

/// The key on a window's artifact: present with this value while the session is changing the page.
pub(crate) const WORKING: &str = "working";

/// The address a note taken in the window names, `On <element>, <where>, at <address>`, or `None`
/// for any other message. Exactly the shape Bridge's note sends (`session-note.ts`).
pub(crate) fn note_address(text: &str) -> Option<&str> {
    let last = text.trim().lines().last()?;
    let (_, address) = last.strip_prefix("On ")?.rsplit_once(", at ")?;
    let web = address.starts_with("http://") || address.starts_with("https://");
    (web && !address.contains(char::is_whitespace)).then_some(address)
}

/// Set every window the session showed to working or ready, and say whether any changed. **Under the
/// caller's lock**, as the other facts are kept.
pub(crate) fn windows_set(
    store: &mut Store,
    holder: &Holder,
    now: &str,
    working: bool,
) -> Result<bool, store::WriteError> {
    let mut changed = false;
    for window in store.attachments_of(holder)? {
        let is_working = window.detail.get(WORKING).map(String::as_str) == Some("true");
        if window.kind != "artifact"
            || window.detail.get("form").map(String::as_str) != Some("window")
            || window.state != AttachmentState::Standing
            || is_working == working
        {
            continue;
        }
        let mut detail = window.detail.clone();
        if working {
            detail.insert(WORKING.to_string(), "true".to_string());
        } else {
            detail.remove(WORKING);
        }
        changed |= store.attach(&KeptAttachment { detail, changed_at: now.to_string().into(), ..window }, false)?;
    }
    Ok(changed)
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
    /// The person took a note in the window: the session will change the page, so its windows are
    /// working. Best effort, as a row is.
    pub(crate) async fn window_noted(&self, session: &str) {
        let harness = {
            let store = self.store().lock().await;
            match store.session(session) {
                Ok(Some(held)) => held.harness,
                _ => return,
            }
        };
        let _ = self
            .report_session(SessionReport {
                harness,
                session_id: SessionId::carried(session),
                fact: SessionFact::Wrote,
            })
            .await;
    }
}
