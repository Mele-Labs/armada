//! What the contents rail says, worked out without a window: the Story items it lists in the Story's order, the words of
//! each, which ask is in view for a scroll top, and where an arrow key lands.

use std::collections::{HashMap, HashSet};

use ipc::{Ask, SessionRow, ThreadItem};

use crate::job_view::story::model::Kind;

/// One ask of the thread, standing where its first words are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub ask_id: String,
    /// The thread item that holds the ask's first words.
    pub item: usize,
}

/// One row of the Job's Story, as the rail draws it: the Story tab's own words for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    pub key: String,
    pub kind: Kind,
    pub title: String,
    pub says: String,
    pub when: String,
}

/// One bar: a Story item, and where in the thread it stands when it is an ask whose first words the thread holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Bar {
    pub item: Item,
    pub at: Option<usize>,
}

/// The bars, one for each Story item in the Story's order. An ask whose first row the thread does not hold is still a bar,
/// which opens the Story where a press on it cannot scroll the thread.
pub(crate) fn bars(items: Vec<Item>, marks: &[Mark]) -> Vec<Bar> {
    items
        .into_iter()
        .map(|item| {
            let at = marks.iter().find(|mark| crate::job_view::story::model::ask_key(&mark.ask_id) == item.key).map(|mark| mark.item);
            Bar { item, at }
        })
        .collect()
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
            Some(Mark { ask_id: ask.ask_id.clone(), item })
        })
        .collect();
    // Stable, so two asks on one message keep Fleet's order between them.
    found.sort_by_key(|mark| mark.item);
    found
}

/// The bar in view: of the asks the thread holds, given as (bar, place in the list), the last whose first row is at or above
/// the list's scroll top. None while the top is above the first of them.
pub(crate) fn in_view(asks: &[(usize, usize)], top: usize) -> Option<usize> {
    asks.iter().filter(|(_, entry)| *entry <= top).max_by_key(|(_, entry)| *entry).map(|(bar, _)| *bar)
}

/// The line under a title in the list: what the row says and when.
pub(crate) fn meta(bar: &Bar) -> String {
    [bar.item.says.as_str(), bar.item.when.as_str()].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" · ")
}

/// What a bar is called: the item and who it is with.
pub(crate) fn bar_name(bar: &Bar) -> String {
    format!("{}, {}", bar.item.title, bar.item.kind.said())
}

/// What a row of the list is called: the title, what it says and when.
pub(crate) fn row_name(bar: &Bar) -> String {
    [bar.item.title.as_str(), bar.item.says.as_str(), bar.item.when.as_str()].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(", ")
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
