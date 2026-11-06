//! Whether an agent's waiting item carries a `walk` act.

use ipc::WaitingActKind;

use fleet::session_host::walk_asks::asks_to_walk;

#[test]
fn a_walk_act_is_a_walk_whatever_the_words() {
    assert!(asks_to_walk(Some(WaitingActKind::Walk)));
    assert!(!asks_to_walk(Some(WaitingActKind::Run)));
    assert!(!asks_to_walk(None));
}
