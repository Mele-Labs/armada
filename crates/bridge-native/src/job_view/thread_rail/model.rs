//! What the contents rail says, worked out without a window: which asks it lists and in what order, the hue and words of
//! each, which one is in view for a scroll top, and where an arrow key lands.

use std::collections::{HashMap, HashSet};

use ipc::{Ask, AskKind, AskState, SessionRow, ThreadItem};

use crate::job_view::kind_chip::model::{ask_label, Hue};

/// One ask of the thread, standing where its first words are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub ask_id: String,
    pub title: String,
    pub kind: AskKind,
    pub state: AskState,
    /// The thread item that holds the ask's first words.
    pub item: usize,
}

/// The asks said in this thread, in the order their first words come in it, not in Fleet's order. An ask whose first row the
/// thread does not hold, such as one filed outside a Job or said in an older stretch not read yet, is left out.
pub(crate) fn marks(asks: &[Ask], items: &[ThreadItem]) -> Vec<Mark> {
    let wanted: HashSet<&str> = asks.iter().map(|ask| ask.first_row.as_str()).collect();
    let mut at: HashMap<&str, usize> = HashMap::new();
    for (item, one) in items.iter().enumerate() {
        if let ThreadItem::Row { row: SessionRow::Message { id, .. } } = one {
            if wanted.contains(id.as_str()) {
                at.entry(id.as_str()).or_insert(item);
            }
        }
    }
    let mut found: Vec<Mark> = asks
        .iter()
        .filter_map(|ask| {
            let item = *at.get(ask.first_row.as_str())?;
            Some(Mark { ask_id: ask.ask_id.clone(), title: ask.title.clone(), kind: ask.kind, state: ask.state, item })
        })
        .collect();
    // Stable, so two asks on one message keep Fleet's order between them.
    found.sort_by_key(|mark| mark.item);
    found
}

/// The ask in view: the last whose first row is at or above the list's scroll top. `entries` are the asks' places in the
/// list, in thread order. None while the top is above the first of them.
pub(crate) fn in_view(entries: &[usize], top: usize) -> Option<usize> {
    entries.iter().rposition(|entry| *entry <= top)
}

/// The colour a kind wears, as the kind chip does: a bug in the error hue, a change request and a fix in the accent, an
/// idea in the awaiting-review hue.
pub(crate) fn hue(kind: AskKind) -> Hue {
    match kind {
        AskKind::Bug => Hue::Bug,
        AskKind::ChangeRequest | AskKind::Fix => Hue::Change,
        AskKind::Idea => Hue::Idea,
    }
}

/// Where an ask stands, in the Story tab's words, lower case after the kind.
pub(crate) fn state_word(state: AskState) -> &'static str {
    match state {
        AskState::Building => "open",
        AskState::Handed => "handed on",
        AskState::Backlog => "backlog",
        AskState::Built => "fixed",
        AskState::Dropped => "dropped",
    }
}

/// The line under an ask's title in the list: its kind and where it stands.
pub(crate) fn meta(mark: &Mark) -> String {
    format!("{}, {}", ask_label(mark.kind), state_word(mark.state))
}

/// What a bar is called: the ask and its kind.
pub(crate) fn bar_name(mark: &Mark) -> String {
    format!("{}, {}", mark.title, ask_label(mark.kind))
}

/// What a row of the list is called: the ask and what its second line says.
pub(crate) fn row_name(mark: &Mark) -> String {
    format!("{}, {}", mark.title, meta(mark))
}

/// Where an arrow lands from `cursor` among `len` rows: one step, kept inside the list.
pub(crate) fn step(cursor: usize, len: usize, down: bool) -> usize {
    if len == 0 {
        0
    } else if down {
        (cursor + 1).min(len - 1)
    } else {
        cursor.saturating_sub(1)
    }
}
