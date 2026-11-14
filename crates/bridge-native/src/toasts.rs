//! Toasts: what the Operator raises during the day, in the window's bottom trailing corner
//! (`floating-layers.md`, Toast). Each stays until its item is settled or he closes it, carries the
//! Operator's pick as its act, and is kept in the sitting's history either way.
//!
//! This is the shelf: which toasts are drawn and which have stepped into history. It knows nothing
//! of the wire beyond the item, so a mock and the Operator page's `Arrived` feed it the same way.

use ipc::operator_page::OperatorTurnItem;

mod view;

pub(crate) use view::about;
pub use view::{at_rest, bind, dispatch_held, dispatched, fleet, job_started, tell, tell_retry, BellSlot, Retry, Toasts};

/// A state of Fleet the app reads for itself, raised as a toast for as long as it lasts: Fleet alive
/// and not answering. Bridge raises it, not the Operator, since Fleet is not there to send it; the
/// title bar reads it (`titlebar::fleet_down`). Fleet stopped or on another build is the centred panel's.
#[derive(Clone, Debug, PartialEq)]
pub struct FleetNotice {
    pub id: &'static str,
    pub headline: gpui::SharedString,
    pub next: gpui::SharedString,
    pub code: &'static str,
    /// A fault draws the red edge; a degraded state the degraded one.
    pub fault: bool,
}

/// How hard a toast asks, as Fleet serves it: its edge, whether it sounds, and whether it holds its
/// place in the stack. Urgent holds its place and sounds; Decision is his to decide; GoAhead is what the
/// Operator would do itself, waiting on his word.
pub use ipc::operator_page::OperatorUrgency as Urgency;

/// How a toast left the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// Its item was answered, here or anywhere else.
    Settled,
    /// He closed it; the item still waits in Your turn.
    Closed,
    /// He put it Later: last in Your turn for the rest of the day.
    Later,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub item: OperatorTurnItem,
    pub urgency: Urgency,
    pub ended: Option<Ended>,
}

/// The most drawn at once. An older open toast steps into history; nothing counts them.
pub const SHOWN: usize = 3;

/// Every toast this sitting, oldest first.
#[derive(Default)]
pub struct Shelf {
    toasts: Vec<Toast>,
}

impl Shelf {
    /// The history kept from an earlier sitting, oldest first.
    pub fn kept(toasts: Vec<Toast>) -> Shelf {
        Shelf { toasts }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.toasts.iter().any(|toast| toast.item.id == id && toast.ended.is_none())
    }

    /// New items raise a toast each; one already raised is not raised again.
    pub fn raise(&mut self, items: &[OperatorTurnItem]) -> Vec<&Toast> {
        let from = self.toasts.len();
        for item in items {
            if self.toasts.iter().all(|toast| toast.item.id != item.id) {
                self.toasts.push(Toast { urgency: item.urgency, item: item.clone(), ended: None });
            }
        }
        self.toasts[from..].iter().collect()
    }

    /// After a read of the page: an open toast whose item is no longer in Your turn was settled.
    pub fn keep_open(&mut self, waiting: &[&str]) {
        for toast in self.toasts.iter_mut().filter(|toast| toast.ended.is_none()) {
            if !waiting.contains(&toast.item.id.as_str()) {
                toast.ended = Some(Ended::Settled);
            }
        }
    }

    pub fn end(&mut self, id: &str, how: Ended) {
        if let Some(toast) = self.toasts.iter_mut().find(|toast| toast.item.id == id && toast.ended.is_none()) {
            toast.ended = Some(how);
        }
    }

    /// What the corner draws, top to bottom: the open toasts, urgent ones held whatever their age,
    /// then the newest of the rest, to `SHOWN` in all; the newest is at the bottom, by the corner.
    pub fn shown(&self) -> Vec<&Toast> {
        let open: Vec<&Toast> = self.toasts.iter().filter(|toast| toast.ended.is_none()).collect();
        let held = open.iter().filter(|toast| toast.urgency == Urgency::Urgent).count();
        let room = SHOWN.saturating_sub(held);
        let others = open.iter().filter(|toast| toast.urgency != Urgency::Urgent).count();
        let mut skip = others.saturating_sub(room);
        open.into_iter()
            .filter(|toast| {
                if toast.urgency == Urgency::Urgent || skip == 0 {
                    return true;
                }
                skip -= 1;
                false
            })
            .collect()
    }

    /// The history, newest first: every toast raised, open or ended.
    pub fn history(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::operator_page::OperatorTurnKind;

    fn item(id: &str, kind: OperatorTurnKind) -> OperatorTurnItem {
        ipc::decode(
            "OperatorTurnItem",
            format!(
                r#"{{"id":"{id}","kind":"{}","item":{{"id":"w","text":"t","source":"agent","since":"2026-11-10T22:00:00Z"}},"raised_at":"2026-11-10T22:00:00Z"}}"#,
                if kind == OperatorTurnKind::GoAhead { "go_ahead" } else { "decision" }
            )
            .as_bytes(),
        )
        .expect("an item")
    }

    fn ids(toasts: Vec<&Toast>) -> Vec<String> {
        toasts.iter().map(|toast| toast.item.id.clone()).collect()
    }

    #[test]
    fn an_item_raises_one_toast_however_often_it_arrives() {
        let mut shelf = Shelf::default();
        assert_eq!(shelf.raise(&[item("turn:1", OperatorTurnKind::Decision)]).len(), 1);
        assert!(shelf.raise(&[item("turn:1", OperatorTurnKind::Decision)]).is_empty());
        assert_eq!(shelf.history().count(), 1);
    }

    #[test]
    fn three_are_drawn_the_newest_last_and_an_urgent_one_holds_its_place() {
        let mut shelf = Shelf::default();
        shelf.raise(&(1..=4).map(|n| item(&format!("turn:{n}"), OperatorTurnKind::GoAhead)).collect::<Vec<_>>());
        assert_eq!(ids(shelf.shown()), ["turn:2", "turn:3", "turn:4"]);
        shelf.toasts[0].urgency = Urgency::Urgent;
        assert_eq!(ids(shelf.shown()), ["turn:1", "turn:3", "turn:4"]);
    }

    #[test]
    fn a_toast_goes_when_its_item_leaves_your_turn_or_he_closes_it_and_history_keeps_both() {
        let mut shelf = Shelf::default();
        shelf.raise(&[item("turn:1", OperatorTurnKind::Decision), item("turn:2", OperatorTurnKind::Decision)]);
        shelf.keep_open(&["turn:2"]);
        shelf.end("turn:2", Ended::Closed);
        assert!(shelf.shown().is_empty());
        let ended: Vec<_> = shelf.history().map(|toast| toast.ended).collect();
        assert_eq!(ended, [Some(Ended::Closed), Some(Ended::Settled)]);
    }
}
