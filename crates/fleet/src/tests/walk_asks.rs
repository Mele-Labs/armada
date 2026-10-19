//! Whether an agent's waiting item asks the person to walk. Every item here is written for the test,
//! in the shapes agents send; `docs/spikes/walk-item-classifier.md` has the measurement over real ones.

use ipc::WaitingActKind;

use crate::session_host::walk_asks::asks_to_walk;

/// Items that ask him to walk, as id and text, with no act.
const WALKS: &[(&str, &str)] = &[
    ("pr-panel", "Walk it: open a pull request panel, close it, open it again."),
    ("", "Walk Set up with a session and approve it or leave notes"),
    ("walk-live-log", "Next: the live log and Stop on a running check, once the first is settled"),
    ("files", "Check the Files tab at localhost:5187/?walk=filesAsATree, then say land it."),
    ("takes", "Pick a take for a sending message in the walk window, then Approve"),
    ("reach", "Try sessions-by-repository in the window, then approve it"),
    ("cockpit", "Approve the Cockpit check panel fix (now in your preview), or leave a note"),
];

/// Items that only mention a walk, or ask something else of him.
const OTHERS: &[(&str, &str)] = &[
    ("approve-plan", "Approve the plan: Fleet refuses any item that asks you to walk"),
    ("approve-2261", "Merge the fix for the setup-with-a-session walk my filter broke"),
    ("delete-branches", "Delete merged local branches armada/fix-main-session-walk"),
    ("context", "Context is at 151k: carry on in a new Job, or stay here? (walk then PR)"),
    ("walker", "Should the walker's footer show a count?"),
    ("preview", "Refresh your preview onto the hover fix, then tell me if the flicker is gone"),
    ("try-bang", "Try `! pwd` in a Session's box once Bridge runs the branch"),
];

#[test]
fn an_item_that_asks_him_to_walk_is_read_as_a_walk() {
    for (id, text) in WALKS {
        assert!(asks_to_walk(id, text, None), "{id}: {text}");
    }
}

#[test]
fn an_item_that_only_mentions_a_walk_is_not_one() {
    for (id, text) in OTHERS {
        assert!(!asks_to_walk(id, text, None), "{id}: {text}");
    }
}

#[test]
fn a_walk_act_is_a_walk_whatever_the_words() {
    assert!(asks_to_walk("look", "Look at the other page", Some(WaitingActKind::Walk)));
    assert!(!asks_to_walk("look", "Look at the other page", Some(WaitingActKind::Run)));
}
