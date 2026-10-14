//! What a Session is doing, read from what Fleet observed and never from the agent saying so.
//! `docs/concepts/session.md`, *What a Session is doing*.
//!
//! **Nothing here needs the agent to call a tool.** A turn that ended on a message to the person is a
//! reply; `done_since` is a done; a running turn that nothing has been heard from is stuck. The agent's
//! own `waiting_for` and `all_done` only add detail, so a Session that never makes either call still
//! reaches the person. Pure: every fact is passed in, so each rule is read and tested without a Fleet.
//!
//! **The stuck threshold is its own** ([`STUCK_AFTER`]), not `settings.session-quiet-timeout`, which
//! is how long an idle process is kept and says nothing about a turn that went quiet.

use std::time::Duration;

use ipc::{Instant, SessionStanding, WaitingItem, WaitingSource};

/// A running turn that nothing has been heard from for this long is stuck. Ten minutes, and nothing
/// measured it; carried as a constant until a user setting reads it.
pub(crate) const STUCK_AFTER: Duration = Duration::from_secs(10 * 60);

/// The facts a Session's standing is read from.
pub(crate) struct Seen<'a> {
    /// A Helm Session: its replies are the dock's, not calls.
    pub helm: bool,
    /// A turn is running: how long since anything was heard from it. `None` between turns.
    pub running: Option<Duration>,
    /// The turn is held on a question or a permission nobody has answered. It is waiting on the
    /// person, which is not the same as stuck.
    pub held: bool,
    /// How the last turn ended while nothing has followed it: when, and the first line of what the
    /// agent last said to the person.
    pub turn_end: Option<(&'a str, &'a str)>,
    /// When the person last wrote to the Session.
    pub spoke: Option<&'a str>,
    /// When the agent said nothing is left, if that still stands.
    pub done: Option<&'a str>,
    /// Items the person dismissed for good.
    pub dismissed: &'a [String],
}

fn millis(at: &str) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(at).epoch_millis()
}

/// The reply the last turn left, if it still stands: `(item id, what was said, since)`.
fn reply<'a>(seen: &Seen<'a>) -> Option<(String, &'a str, &'a str)> {
    if seen.helm || seen.running.is_some() {
        return None;
    }
    let (ended, said) = seen.turn_end?;
    if said.trim().is_empty() {
        return None;
    }
    // He wrote after the turn ended: he has answered it, in the Session.
    if let (Some(spoke), Some(ended_at)) = (seen.spoke.and_then(millis), millis(ended)) {
        if spoke > ended_at {
            return None;
        }
    }
    let id = format!("replied:{ended}");
    (!seen.dismissed.contains(&id)).then_some((id, said, ended))
}

/// The done the agent stated, if it still stands: `(item id, since)`.
fn done<'a>(seen: &Seen<'a>) -> Option<(String, &'a str)> {
    if seen.helm || seen.running.is_some() {
        return None;
    }
    let since = seen.done?;
    let id = format!("done:{since}");
    (!seen.dismissed.contains(&id)).then_some((id, since))
}

/// How the Session stands. A running turn is working, or stuck once nothing has been heard from it for
/// `stuck_after` (never while it waits on the person); between turns a done wins over a reply.
pub(crate) fn standing(seen: &Seen, stuck_after: Duration) -> SessionStanding {
    if let Some(quiet) = seen.running {
        return if quiet > stuck_after && !seen.held {
            SessionStanding::Stuck
        } else {
            SessionStanding::Working
        };
    }
    if done(seen).is_some() {
        SessionStanding::Done
    } else if reply(seen).is_some() {
        SessionStanding::Replied
    } else {
        SessionStanding::Idle
    }
}

/// The items Fleet derives from the turn: the done, and the reply. **The reply is left off where
/// something else is already owed** (`others`): the agent said what it needs in that item, and a
/// second card for the same turn is noise.
pub(crate) fn derived(seen: &Seen, others: bool) -> Vec<WaitingItem> {
    let item = |id: String, text: &str, since: &str, source| WaitingItem {
        id,
        text: text.to_string(),
        since: Instant::carried(since),
        source,
        act: None,
        options: Vec::new(),
        place: None,
    };
    if let Some((id, since)) = done(seen) {
        return vec![item(id, "Done", since, WaitingSource::Done)];
    }
    match (others, reply(seen)) {
        (false, Some((id, said, since))) => vec![item(id, said, since, WaitingSource::Replied)],
        _ => Vec::new(),
    }
}

/// The first line of what the agent said, as the item's text: trimmed, and no longer than a line of a
/// card can hold.
pub(crate) fn first_line(said: &str) -> String {
    let line = said.lines().map(str::trim).find(|one| !one.is_empty()).unwrap_or("");
    if line.chars().count() <= 200 {
        return line.to_string();
    }
    let cut: String = line.chars().take(199).collect();
    format!("{}…", cut.trim_end())
}
