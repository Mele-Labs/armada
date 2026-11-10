//! Toasts: what the Operator raises during the day, in the window's bottom trailing corner
//! (`floating-layers.md`, Toast). Each stays until its item is settled or he closes it, carries the
//! Operator's pick as its act, and is kept in the sitting's history either way.
//!
//! This is the shelf: which toasts are drawn and which have stepped into history. It knows nothing
//! of the wire beyond the item, so a mock and the Operator page's `Arrived` feed it the same way.

use ipc::operator_page::{OperatorTurnItem, OperatorTurnKind};

/// How hard a toast asks: its edge, whether it sounds, and whether it holds its place in the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    /// Holds its place and sounds.
    Urgent,
    /// His to decide.
    Decision,
    /// What the Operator would do itself, waiting on his word.
    GoAhead,
}

/// Read here until Fleet serves `urgency` on `OperatorTurnItem`, which replaces this.
pub fn urgency_of(item: &OperatorTurnItem) -> Urgency {
    if crate::operator::face::is_red_main(item) {
        return Urgency::Urgent;
    }
    match item.kind {
        OperatorTurnKind::Decision => Urgency::Decision,
        OperatorTurnKind::GoAhead => Urgency::GoAhead,
    }
}

/// How a toast left the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// Its item was answered, here or anywhere else.
    Settled,
    /// He closed it; the item still waits in Your turn.
    Closed,
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
    /// New items raise a toast each; one already raised is not raised again.
    pub fn raise(&mut self, items: &[OperatorTurnItem]) -> Vec<&Toast> {
        let from = self.toasts.len();
        for item in items {
            if self.toasts.iter().all(|toast| toast.item.id != item.id) {
                self.toasts.push(Toast { urgency: urgency_of(item), item: item.clone(), ended: None });
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
