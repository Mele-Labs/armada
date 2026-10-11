//! Whether a session's walk is open, and the one thing that depends on it: a question the agent
//! asks meanwhile is refused rather than held. `docs/concepts/session.md`, *Showing a window*.
//!
//! **Read off the thread, not off the waiting list.** The `walk:` item settles at the person's
//! first message after the window opened, and a note taken in the window is such a message, so
//! the walk would read as over at his first note.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::SessionRow;

use crate::daemon::Fleet;

/// What the agent reads when its question is refused during a walk. **A sentence may follow it**
/// naming a way to ask about the page itself.
pub(crate) const WALK_OPEN_ASK: &str = "Refused: the owner is walking a page you showed him, and a question held open here would keep his notes and his Approve from reaching you until he answered it. Put the question in your waiting_for list, where he answers it when he is ready, and end your turn so what he sends reaches you.";

/// What Bridge's Approve sends the session, ahead of the address.
pub(crate) const APPROVED: &str = "Approved: ";

/// The address of the walk open on this thread: the newest `window` row, unless it has ended.
pub(crate) fn open_walk(rows: &[SessionRow]) -> Option<&str> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Window { url, ended_at, .. } => Some(ended_at.is_none().then_some(url.as_str())),
        _ => None,
    })?
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
    /// The address of the session's open walk. **A thread that will not read is no walk**: the
    /// question is then put to the person, as before walks were read.
    pub(crate) async fn walk_open(&self, session: &str) -> Option<String> {
        let rows = self.rows_of(session).await.ok()?;
        open_walk(&rows).map(str::to_string)
    }

    /// End the newest walk on `url` that is still open: its row gets `ended_at` and is published
    /// again. Where none is open, nothing happens.
    pub(crate) async fn walk_ended(&self, session: &str, url: &str) {
        let Ok(rows) = self.rows_of(session).await else {
            return;
        };
        let open = rows.into_iter().rev().find_map(|row| match row {
            SessionRow::Window { id, at, title, url: shown, ended_at: None } if shown == url => {
                Some(SessionRow::Window { id, at, title, url: shown, ended_at: Some(self.instant()) })
            }
            _ => None,
        });
        if let Some(ended) = open {
            self.row_put(session, ended).await;
        }
    }
}
