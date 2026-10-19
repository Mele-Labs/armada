//! Whether an agent's waiting item asks the person to walk something, read from its act, id and text.
//! `docs/spikes/walk-item-classifier.md` has the rules measured, and why no model.
//!
//! **A walk reaches his list only through `show_window`**, as Fleet's own `walk:<url>` item, which
//! settles at his Approve. An agent's item for it is a second call before the window and a stale one
//! after it, so `waiting_for` refuses it and a read leaves off one kept before this rule.

use ipc::WaitingActKind;

/// Phrases only a walk's item uses, matched anywhere in its text once lowercased.
const WALKING: &[&str] = &["?walk=", "walk window", "in the window", "leave a note", "leave notes"];

/// Whether an item asks him to walk: a `walk` act, an id or a text whose first word is `walk`, or a
/// phrase in [`WALKING`]. "Walk" elsewhere is a branch, a plan or a past walk, so it does not count.
pub(crate) fn asks_to_walk(id: &str, text: &str, act: Option<WaitingActKind>) -> bool {
    let text = text.to_lowercase();
    act == Some(WaitingActKind::Walk)
        || first_word(id) == "walk"
        || first_word(&text) == "walk"
        || WALKING.iter().any(|phrase| text.contains(phrase))
}

fn first_word(said: &str) -> String {
    said.trim_start()
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_lowercase()
}

/// What the agent reads when its list is refused for a walk item.
pub(crate) fn walk_item_refusal(id: &str) -> String {
    format!(
        "Refused: item \"{id}\" asks the owner to walk something, and a walk reaches his list only \
         through show_window. Fleet lists each window you show and takes it off when he approves it. \
         Build the walk and call show_window if you have not, then call waiting_for again without the item."
    )
}
