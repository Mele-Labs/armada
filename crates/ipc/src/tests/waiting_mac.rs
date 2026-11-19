//! A `run` act may name the Mac its command is meant for (`WaitingAct.on`), and a payload from before it decodes
//! as naming none. The Needs-you row carries the same name (`NeedsYouRow.on`).

use crate::{NeedsYouRow, WaitingActKind, WaitingInput};

#[test]
fn a_run_act_naming_a_mac_crosses_with_it_and_one_naming_none_leaves_no_key() {
    let named = serde_json::json!({
        "id": "peers",
        "text": "Read the peers on the Kiavi Mac",
        "act": {"kind": "run", "target": "curl -s localhost:4000/v1/peers", "on": "Nick-Meles-MacBook-Pro---Kiavi"}
    });
    let read: WaitingInput = serde_json::from_value(named.clone()).unwrap();
    let act = read.act.as_ref().unwrap();
    assert_eq!((act.kind, act.on.as_deref()), (WaitingActKind::Run, Some("Nick-Meles-MacBook-Pro---Kiavi")));
    assert_eq!(serde_json::to_value(&read).unwrap(), named);

    let before = serde_json::json!({"id": "test", "text": "Run the tests", "act": {"kind": "run", "target": "cargo test"}});
    let read: WaitingInput = serde_json::from_value(before.clone()).unwrap();
    assert_eq!(read.act.as_ref().unwrap().on, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), before, "no `on` key, never null");
}

#[test]
fn a_needs_you_row_carries_the_mac_and_leaves_the_key_off_without_one() {
    let row = serde_json::json!({
        "id": "peers",
        "source": "agent",
        "text": "Read the peers on the Kiavi Mac",
        "since": "2026-11-18T09:00:00Z",
        "glyph": {"kind": "run", "said": "Copies the command"},
        "act": {"kind": "run", "command": "curl -s localhost:4000/v1/peers"},
        "dismiss": "plain",
        "on": "Kiavi"
    });
    let read: NeedsYouRow = serde_json::from_value(row.clone()).unwrap();
    assert_eq!(read.on.as_deref(), Some("Kiavi"));
    assert_eq!(serde_json::to_value(&read).unwrap(), row);
    let mut before = row;
    before.as_object_mut().unwrap().remove("on");
    let read: NeedsYouRow = serde_json::from_value(before.clone()).unwrap();
    assert_eq!(read.on, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), before);
}
