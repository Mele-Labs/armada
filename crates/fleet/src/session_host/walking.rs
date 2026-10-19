//! Whether a session's walk is open, and the one thing that depends on it: a question the agent
//! asks meanwhile is refused rather than held. `docs/concepts/session.md`, *Showing a window*.
//!
//! **Read off the thread, not off the waiting list.** The `walk:` item settles at the person's
//! first message after the window opened, and a note taken in the window is such a message, so
//! the walk would read as over at his first note.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, SessionRow, SessionVoice};

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

/// The windows the Job showed that still stand: one artifact per page.
pub(crate) fn walk_windows(attachments: &[ipc::Attachment]) -> impl Iterator<Item = &ipc::Attachment> {
    attachments.iter().filter(|one| {
        one.kind == "artifact"
            && one.detail.get("form").map(String::as_str) == Some("window")
            && one.state == ipc::AttachmentState::Standing
    })
}

/// Whether the Job is changing the page now (`readiness.rs`).
pub(crate) fn is_working(window: &ipc::Attachment) -> bool {
    window.detail.get(super::readiness::WORKING).map(String::as_str) == Some("true")
}

/// Whether the page was changed after it was shown and has not been shown again. **Ready does not
/// clear it**; only showing the page again does (`readiness.rs`).
pub(crate) fn is_changed(window: &ipc::Attachment) -> bool {
    window.detail.get(super::readiness::CHANGED).map(String::as_str) == Some("true")
}

/// When the newest window row showing `walk` went out, or `None` for a page never shown.
pub(crate) fn walk_shown_at<'a>(rows: &'a [SessionRow], walk: &str) -> Option<&'a Instant> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Window { at, url, variations, .. } if url == walk || variations.iter().any(|take| take.url == walk) => Some(at),
        _ => None,
    })
}

/// When the person approved the walk on `walk`, or `None` while he has not. **Only an Approve at or
/// after the newest window row showing it counts**, so a page shown again must be approved again. A
/// take's Approve is the window's, as it is for the walk item. A page never shown is not approved.
pub(crate) fn walk_approval<'a>(rows: &'a [SessionRow], walk: &str) -> Option<&'a Instant> {
    let (shown, url, takes) = rows.iter().rev().find_map(|row| match row {
        SessionRow::Window { at, url, variations, .. }
            if url == walk || variations.iter().any(|take| take.url == walk) =>
        {
            Some((at, url, variations))
        }
        _ => None,
    })?;
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { at, from: SessionVoice::You, text, .. } if at >= shown => {
                let address = approved_address(text)?;
                (address == walk || address == url || takes.iter().any(|take| take.url == address)).then_some(at)
            }
            _ => None,
        })
        .max()
}

/// One page wherever it is served: the address without its port or a trailing slash. A dev server
/// started again often takes another port, and what it serves there is the same walk.
fn page_of(address: &str) -> String {
    let (scheme, rest) = address.split_once("://").unwrap_or(("", address));
    let (authority, path) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|one| one.is_ascii_digit()) => host,
        _ => authority,
    };
    format!("{scheme}://{host}{path}").trim_end_matches('/').to_string()
}

/// Whether a newer window replaced the one on `walk`: a later `window` row on another address shows
/// one of its pages, page or take, compared by [`page_of`]. **Only the same page replaces**: a newer
/// window of another page leaves this one waiting until his Approve.
pub(crate) fn replaced(rows: &[SessionRow], walk: &str) -> bool {
    let pages = |url: &str, takes: &[ipc::WindowTake]| -> Vec<String> {
        std::iter::once(url).chain(takes.iter().map(|take| take.url.as_str())).map(page_of).collect()
    };
    let Some(at) = rows.iter().rposition(|row| matches!(row, SessionRow::Window { url, .. } if url == walk)) else {
        return false;
    };
    let SessionRow::Window { url, variations, .. } = &rows[at] else {
        return false;
    };
    let mine = pages(url, variations);
    rows[at + 1..].iter().any(|row| {
        matches!(row, SessionRow::Window { url, variations, .. }
            if url != walk && pages(url, variations).iter().any(|page| mine.contains(page)))
    })
}

/// The window on `walk`, page or take: the address it was shown on, from its newest row.
pub(crate) fn window_of<'a>(rows: &'a [SessionRow], walk: &str) -> Option<&'a str> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Window { url, variations, .. } if url == walk || variations.iter().any(|take| take.url == walk) => {
            Some(url.as_str())
        }
        _ => None,
    })
}

/// Whether what Bridge said of a window is that nothing answered on its address: Chromium's own
/// error (`ERR_CONNECTION_REFUSED`) or a code it gave no name, as `capture/failed.ts` words a failed
/// load. A page that answered broken (`HTTP 500`, the error overlay) is still to be walked once fixed.
pub(crate) fn could_not_load(why: &str) -> bool {
    let why = why.trim();
    why.starts_with("ERR_") || why.starts_with("load failed")
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
