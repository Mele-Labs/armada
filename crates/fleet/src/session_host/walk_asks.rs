//! An agent's waiting item that carries a `walk` act.
//!
//! **A walk reaches his list only through `show_window`**, as Fleet's own `walk:<url>` item, which
//! settles at his Approve. An agent's item for it is a second call before the window and a stale one
//! after it, so `waiting_for` refuses it and a read leaves off one kept before this rule. Only the act
//! says it: Fleet does not read the item's words.

use ipc::WaitingActKind;

/// Whether an item carries a `walk` act.
pub(crate) fn asks_to_walk(act: Option<WaitingActKind>) -> bool {
    act == Some(WaitingActKind::Walk)
}

/// What the agent reads when its list is refused for a walk item.
pub(crate) fn walk_item_refusal(id: &str) -> String {
    format!(
        "Refused: item \"{id}\" asks the owner to walk something, and a walk reaches his list only \
         through show_window. Fleet lists each window you show and takes it off when he approves it. \
         Build the walk and call show_window if you have not, then call waiting_for again without the item."
    )
}
