//! An agent's item asking for his approval keeps the mark through a reopen of the file, open and
//! closed, and each Approve he pressed is kept beside the dismissal it made.

use crate::tests::{open, TempDir};
use crate::{KeptApproval, KeptClosed, KeptWaiting};

fn item(id: &str, approves: bool) -> KeptWaiting {
    KeptWaiting {
        item_id: id.into(),
        text: format!("Approve {id}"),
        since: "2026-11-10T09:00:00.000Z".into(),
        act: None,
        options: vec!["Approve".into(), "Ask for changes".into()],
        pictures: String::new(),
        steps: Vec::new(),
        approves,
    }
}

#[test]
fn an_item_asking_for_his_approval_keeps_the_mark_open_closed_and_reopened() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_waiting_for("s1", &[item("design", true), item("plain", false)]).unwrap();
    drop(store);
    let mut store = open(&dir);
    let read = store.waiting_for("s1").unwrap();
    assert_eq!(read.iter().map(|one| (one.item_id.as_str(), one.approves)).collect::<Vec<_>>(), [("design", true), ("plain", false)]);

    let closed = KeptClosed {
        item_id: "design".into(),
        text: "Approve design".into(),
        source: "agent".into(),
        act: None,
        options: vec!["Approve".into(), "Ask for changes".into()],
        pictures: String::new(),
        steps: Vec::new(),
        since: "2026-11-10T09:00:00.000Z".into(),
        closed_at: "2026-11-10T10:00:00.000Z".into(),
        approves: true,
    };
    store.keep_closed_waiting("s1", &closed).unwrap();
    drop(store);
    let mut store = open(&dir);
    assert_eq!(store.closed_waiting("s1").unwrap(), [closed]);

    store.reopen_waiting("s1", "design", Some(&item("design", true))).unwrap();
    let back = store.waiting_for("s1").unwrap();
    assert!(back.iter().any(|one| one.item_id == "design" && one.approves), "a reopened item still asks for his approval");
}

#[test]
fn his_approve_is_kept_with_the_dismissal_and_survives_a_reopen_of_the_file() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_waiting_for("s1", &[item("design", true)]).unwrap();
    let approval = KeptApproval { item_id: "design".into(), text: "Approve design".into(), at: "2026-11-10T10:00:00.000Z".into() };
    store.approve_waiting("s1", &approval).unwrap();
    store.approve_waiting("s1", &approval).unwrap();
    drop(store);
    let store = open(&dir);
    assert!(store.waiting_for("s1").unwrap().is_empty(), "the approved item is off the list");
    assert_eq!(store.dismissed_waiting("s1").unwrap(), ["design"], "and dismissed, so a restatement does not bring it back");
    assert_eq!(store.waiting_approvals("s1").unwrap(), [approval], "kept once");
    assert!(store.waiting_approvals("s2").unwrap().is_empty(), "another Job's are its own");
}
