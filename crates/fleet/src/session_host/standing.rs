//! What a Session is doing, read from what Fleet observed and never from the agent saying so.
//! `docs/concepts/session.md`, *What a Session is doing*.
//!
//! **Nothing here needs the agent to call a tool.** A turn that ended on a message to the person is a
//! reply; `done_since` is a done; a running turn that nothing has been heard from is stuck. The agent's
//! own `waiting_for` and `all_done` only add detail, so a Session that never makes either call still
//! reaches the person. Pure: every fact is passed in, so each rule is read and tested without a Fleet.
//!
//! **A reply that only tells is a standing; a reply that asks is an item.** One that only reports is the
//! unread mark under Idle that Bridge draws. One whose close asks him something ([`Seen::asks`]) is a
//! line in Needs you, unless the agent already asked through a tool. A done asks him whether to end the
//! Session, so it is an item too.
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
    /// A subagent of it is still at work, which a turn that ended to wait on it does not say.
    pub subagents: bool,
    /// The turn is held on a question or a permission nobody has answered. It is waiting on the
    /// person, which is not the same as stuck.
    pub held: bool,
    /// The agent already put its question to the person through a tool: an item of its own, a card, a
    /// permission or a window question is open. Its reply then raises no item of its own.
    pub called: bool,
    /// How the last turn ended while nothing has followed it: when, and the end of what the agent
    /// last said to the person ([`tail`]).
    pub turn_end: Option<(&'a str, &'a str)>,
    /// When the person last wrote to the Session.
    pub spoke: Option<&'a str>,
    /// When the agent said nothing is left, if that still stands.
    pub done: Option<&'a str>,
    /// Items the person dismissed for good.
    pub dismissed: &'a [String],
    /// The sentence of a reply that asks, if one does: the reader `asks_model::SHIPPED` names, or the
    /// rules of `asks::asking_line` until the model is loaded.
    pub asks: &'a dyn Fn(&str) -> Option<String>,
}

fn millis(at: &str) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(at).epoch_millis()
}

/// Whether the last turn left a reply that still stands.
fn replied(seen: &Seen) -> bool {
    if seen.helm || seen.running.is_some() {
        return false;
    }
    let Some((ended, said)) = seen.turn_end else {
        return false;
    };
    if said.trim().is_empty() {
        return false;
    }
    // He wrote after the turn ended: he has answered it, in the Session.
    match (seen.spoke.and_then(millis), millis(ended)) {
        (Some(spoke), Some(ended_at)) => spoke <= ended_at,
        _ => true,
    }
}

/// When the reply ended and the sentence of it that asks the person, if it asks.
fn asking<'a>(seen: &Seen<'a>) -> Option<(&'a str, String)> {
    if !replied(seen) {
        return None;
    }
    let (ended, said) = seen.turn_end?;
    (seen.asks)(said).map(|line| (ended, first_line(&line)))
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
/// `stuck_after` (never while it waits on the person); between turns a subagent at work is working,
/// and a done wins over a reply. A reply that asks is no unread mark: it is an item, or the agent
/// asked through a tool.
pub(crate) fn standing(seen: &Seen, stuck_after: Duration) -> SessionStanding {
    if let Some(quiet) = seen.running {
        return if quiet > stuck_after && !seen.held {
            SessionStanding::Stuck
        } else {
            SessionStanding::Working
        };
    }
    if seen.subagents {
        SessionStanding::Working
    } else if done(seen).is_some() {
        SessionStanding::Done
    } else if replied(seen) && asking(seen).is_none() {
        SessionStanding::Replied
    } else {
        SessionStanding::Idle
    }
}

/// The items Fleet derives from the turn: the done, and a reply that asks.
pub(crate) fn derived(seen: &Seen) -> Vec<WaitingItem> {
    let mut items = Vec::new();
    let done = done(seen);
    if let Some((id, since)) = &done {
        items.push(item(id.clone(), "Done", since, WaitingSource::Done));
    }
    // A done outranks a reply, a subagent at work means the turn is not over for the person, and
    // an agent that asked through a tool has asked once already.
    if done.is_none() && !seen.subagents && !seen.held && !seen.called {
        if let Some((ended, line)) = asking(seen) {
            let id = format!("replied:{ended}");
            if !seen.dismissed.contains(&id) {
                items.push(item(id, &line, ended, WaitingSource::Replied));
            }
        }
    }
    items
}

fn item(id: String, text: &str, since: &str, source: WaitingSource) -> WaitingItem {
    WaitingItem {
        id,
        text: text.to_string(),
        since: Instant::carried(since),
        source,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
    }
}

/// How much of the end of a reply is kept. A reply asks at its close, so this is the part read.
const TAIL: usize = 1500;

/// The end of what the agent said, from the start of a line, for [`Seen::asks`] to read.
pub(crate) fn tail(said: &str) -> String {
    let said = said.trim();
    let count = said.chars().count();
    if count <= TAIL {
        return said.to_string();
    }
    let cut = said.char_indices().nth(count - TAIL).map_or(0, |(at, _)| at);
    let kept = &said[cut..];
    match kept.split_once('\n') {
        Some((_, rest)) if !rest.trim().is_empty() => rest.trim_start().to_string(),
        _ => kept.to_string(),
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
