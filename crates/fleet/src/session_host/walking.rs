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
pub(crate) const WALK_OPEN_ASK: &str = "Refused: the owner is walking a page you showed him, and a question held open here would keep his notes and his Approve from reaching you until he answered it. Your question is kept, whole, in his Needs you list, where he answers it after the walk, and his answer reaches you as his message. Do not add it to waiting_for. End your turn so what he sends reaches you.";

/// The sentence that follows [`WALK_OPEN_ASK`]: a question about the page has a way in that holds
/// nothing back. `docs/concepts/session.md`, *Asking in the window*.
pub(crate) const WALK_ASK_THE_PAGE: &str = "If the question is about the page you showed him, ask it in the window instead: call ask_in_window with the page's url and a CSS selector for the place.";

/// The whole refusal of a question asked during a walk.
pub(crate) fn walk_open_refusal() -> String {
    format!("{WALK_OPEN_ASK} {WALK_ASK_THE_PAGE}")
}

/// What Bridge's Approve sends the session, ahead of the address. **Of several takes it names the
/// one approved between**: `Approved B, Needs you first: <address>`.
const APPROVED: &str = "Approved";

/// The address an Approve message approves, or `None` where the text is anything else. Exactly the
/// two shapes Bridge sends, so a person's own sentence that begins with the word is not one.
pub(crate) fn approved_address(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix(APPROVED)?;
    if let Some(address) = rest.strip_prefix(": ") {
        return Some(address.trim());
    }
    let take = rest.strip_prefix(' ')?;
    let mut letters = take.chars();
    (letters.next()?.is_ascii_uppercase() && letters.as_str().starts_with(", ")).then_some(())?;
    take.rsplit_once(": ").map(|(_, address)| address.trim())
}

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

    /// End the newest walk on `url` that is still open, whether `url` is its page or one of its
    /// takes: its row gets `ended_at` and is published again. Where none is open, nothing happens.
    pub(crate) async fn walk_ended(&self, session: &str, url: &str) {
        let Ok(rows) = self.rows_of(session).await else {
            return;
        };
        let open = rows.into_iter().rev().find_map(|row| match row {
            SessionRow::Window { id, at, title, url: shown, variations, steps, origins, ended_at: None }
                if shown == url || variations.iter().any(|take| take.url == url) =>
            {
                Some(SessionRow::Window { id, at, title, url: shown, variations, steps, origins, ended_at: Some(self.instant()) })
            }
            _ => None,
        });
        if let Some(ended) = open {
            self.row_put(session, ended).await;
        }
    }
}
