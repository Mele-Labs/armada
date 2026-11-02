//! Which kind of decision a waiting item is, and so which box allows answering it. **Pure.** An item
//! that is none of the kinds, or is destructive, waits for the person whatever is checked.
//!
//! The shape is Sleep's sort, widened: a question with a recommendation, a question without one, a
//! plan, a draft, a walk, and a permission card read as a read or a write.

use ipc::{AutopilotDecision, HelmCallInFlight, WaitingActKind, WaitingItem, WaitingSource};

use super::destructive::{destructive, destructive_text, recommended};

/// Tools that only read. **Spelled as the harness spells them**, and a tool not named here is not a read.
const READS: &[&str] = &["Read", "Grep", "Glob", "LS", "NotebookRead", "WebFetch", "WebSearch"];

/// Tools that change files, beside every run command.
const WRITES: &[&str] = &["Edit", "Write", "MultiEdit", "NotebookEdit"];

/// The run command's tool.
const RUN: &str = "Bash";

/// The kind of `item`, or `None` where it waits for a person. `card` is the open permission card,
/// which a permission item is read from.
pub(crate) fn sorted(item: &WaitingItem, card: Option<&HelmCallInFlight>) -> Option<AutopilotDecision> {
    if destructive(item) {
        return None;
    }
    match item.source {
        WaitingSource::AskCard | WaitingSource::Agent => question(item),
        WaitingSource::Plan => Some(AutopilotDecision::Plan),
        WaitingSource::Draft => Some(AutopilotDecision::Draft),
        WaitingSource::Walk => Some(AutopilotDecision::Walk),
        WaitingSource::Permission => permission(card?),
        // A question asked in the walk is about what is on the page, so it waits for his eyes as
        // the walk does. Every other item is his to send, say or press.
        _ => None,
    }
}

/// A question with options: an errand, one that asks him to run or approve something, and one with
/// no options waits.
fn question(item: &WaitingItem) -> Option<AutopilotDecision> {
    if item.options.is_empty() || item.act.as_ref().is_some_and(|act| act.kind != WaitingActKind::Answer) {
        return None;
    }
    Some(match recommended(item) {
        Some(_) => AutopilotDecision::Recommended,
        None => AutopilotDecision::Open,
    })
}

/// A permission is a read, or a write, and **only where Fleet can read all of it**: a run command
/// it cannot read waits, and so does a card about a path past the Job's slots.
fn permission(card: &HelmCallInFlight) -> Option<AutopilotDecision> {
    let whole = card.whole.as_deref().unwrap_or(&card.detail);
    if card.reaches.is_some() || destructive_text(&format!("{} {whole}", card.tool)) {
        return None;
    }
    if READS.contains(&card.tool.as_str()) {
        return Some(AutopilotDecision::Reads);
    }
    let readable = !whole.trim().is_empty() && (!card.truncated || card.whole.is_some());
    if WRITES.contains(&card.tool.as_str()) || (card.tool == RUN && readable) {
        return Some(AutopilotDecision::Writes);
    }
    None
}
