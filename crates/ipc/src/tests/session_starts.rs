//! The calls an agent makes to start or fork a Session, the row the thread keeps and the item the
//! person is left waiting on, each as Bridge's mirror in `packages/protocol` spells it.

use serde_json::json;

use crate::{BeginSession, DeliveryChoice, DeliveryPicked, DeliveryPickedFrom, ForkThisSession, HostedFacts, StartHeld, StartSession, SessionHeld, SessionRow, StartAnotherSession, WaitingActKind, WaitingItem, WaitingSource};

#[test]
fn a_start_reads_its_repository_title_and_first_message() {
    let start: StartAnotherSession = crate::decode(
        "a session to start",
        br#"{"manifest_id":"armada","title":"Check the reader","message":"Read crates/reader and say why it drops lines."}"#,
    )
    .unwrap();
    assert_eq!(start.title, "Check the reader");
    assert!(crate::decode::<StartAnotherSession>("a session to start", br#"{"manifest_id":"armada","title":"x"}"#).is_err());
}

#[test]
fn a_fork_may_leave_the_message_it_ends_at_to_fleet_and_writes_nothing_absent() {
    let fork: ForkThisSession = crate::decode("a fork", br#"{"title":"Try the other way","message":"Do it with a map."}"#).unwrap();
    assert!(fork.at.is_none());
    assert_eq!(serde_json::to_value(&fork).unwrap(), json!({"title":"Try the other way","message":"Do it with a map."}));
    let cut: ForkThisSession = crate::decode("a fork", br#"{"title":"t","message":"m","at":"r4"}"#).unwrap();
    assert_eq!(cut.at.as_deref(), Some("r4"));
}

#[test]
fn the_answer_the_row_and_the_item_round_trip() {
    let held = json!({"session_id": "s2", "title": "Check the reader", "fork": false});
    let read: SessionHeld = serde_json::from_value(held.clone()).unwrap();
    assert_eq!(serde_json::to_value(&read).unwrap(), held);

    let row = json!({
        "kind": "started", "id": "r9", "at": "2026-10-12T09:00:00Z",
        "session": {"id": "s2", "title": "Check the reader"}, "fork": true
    });
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    assert_eq!(read.id(), "r9");
    assert_eq!(serde_json::to_value(&read).unwrap(), row);

    let item = json!({
        "id": "unsent:s2", "text": "First message", "since": "2026-10-12T09:00:00Z",
        "source": "first_message", "act": {"kind": "message", "target": "s2"}
    });
    let read: WaitingItem = serde_json::from_value(item.clone()).unwrap();
    assert_eq!((read.source, read.act.as_ref().map(|act| act.kind)), (WaitingSource::FirstMessage, Some(WaitingActKind::Message)));
    assert_eq!(serde_json::to_value(&read).unwrap(), item);
}

#[test]
fn a_group_is_optional_on_a_start_and_written_only_when_set() {
    let plain = StartAnotherSession { manifest_id: crate::ManifestId::carried("armada"), title: "t".into(), message: "m".into(), group: None, after: None };
    assert_eq!(serde_json::to_value(&plain).unwrap(), json!({"manifest_id":"armada","title":"t","message":"m"}));
    let grouped: StartAnotherSession =
        crate::decode("a session to start", br#"{"manifest_id":"armada","title":"t","message":"m","group":"g2"}"#).unwrap();
    assert_eq!(grouped.group.as_deref(), Some("g2"));
}

#[test]
fn a_hand_names_the_session_and_the_group() {
    let hand: crate::HandToSession = crate::decode("a group to hand", br#"{"session_id":"s1","group":"g2"}"#).unwrap();
    assert_eq!((hand.session_id.as_str(), hand.group.as_str()), ("s1", "g2"));
    assert!(crate::decode::<crate::HandToSession>("a group to hand", br#"{"session_id":"s1"}"#).is_err());
}

#[test]
fn a_linked_row_names_the_session_and_what_it_did() {
    let row = json!({
        "kind": "linked", "id": "r10", "at": "2026-10-16T09:00:00Z",
        "session": {"id": "s3", "title": "Migrations apply in order"}, "event": "needs"
    });
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    assert_eq!(read.id(), "r10");
    assert!(matches!(read, SessionRow::Linked { event: crate::LinkedEvent::Needs, .. }));
    assert_eq!(serde_json::to_value(&read).unwrap(), row);
    for (spelling, event) in [("done", crate::LinkedEvent::Done), ("merged", crate::LinkedEvent::Merged)] {
        assert_eq!(serde_json::from_value::<crate::LinkedEvent>(json!(spelling)).unwrap(), event);
    }
}

#[test]
fn a_release_row_round_trips_as_kind_release() {
    let row = json!({"kind": "release", "id": "r5", "at": "2026-10-22T09:00:00Z", "slot": 3, "branch": "armada/session-1"});
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    assert_eq!(read.id(), "r5");
    assert!(matches!(read, SessionRow::Release { slot: 3, .. }));
    assert_eq!(serde_json::to_value(&read).unwrap(), row);
}

#[test]
fn a_start_is_held_only_when_it_says_so_and_a_plain_one_writes_nothing() {
    let plain = r#"{"manifest_id":"armada"}"#;
    let read: StartSession = crate::decode("a start", plain.as_bytes()).unwrap();
    assert!(!read.held);
    assert_eq!(serde_json::to_value(&read).unwrap(), json!({"manifest_id":"armada"}));
    let held: StartSession = crate::decode("a start", br#"{"manifest_id":"armada","held":true}"#).unwrap();
    assert!(held.held);
    assert_eq!(serde_json::to_value(&held).unwrap(), json!({"manifest_id":"armada","held":true}));
}

#[test]
fn a_held_jobs_start_card_round_trips_and_is_left_out_once_it_has_begun() {
    let waiting = json!({
        "turn": {"state": "idle"}, "mode": "auto", "running": false,
        "start": {"asks_delivery": true, "picked": {"choice": "auto_land", "from": "usual"}}
    });
    let read: HostedFacts = serde_json::from_value(waiting.clone()).unwrap();
    assert_eq!(
        read.start,
        Some(StartHeld {
            asks_delivery: true,
            picked: Some(DeliveryPicked { choice: DeliveryChoice::AutoLand, from: DeliveryPickedFrom::Usual }),
            message: None,
        })
    );
    assert_eq!(serde_json::to_value(&read).unwrap(), waiting);

    let named = json!({"asks_delivery": false, "picked": {"choice": "keep_local", "from": "named"}});
    let read: StartHeld = serde_json::from_value(named.clone()).unwrap();
    assert_eq!(read.picked.as_ref().map(|one| one.from), Some(DeliveryPickedFrom::Named));
    assert_eq!(serde_json::to_value(&read).unwrap(), named);

    let begun = json!({"turn": {"state": "idle"}, "mode": "auto", "running": false});
    let read: HostedFacts = serde_json::from_value(begun.clone()).unwrap();
    assert_eq!(read.start, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), begun);
}

#[test]
fn begin_names_the_job_and_the_landing_only_when_one_was_settled() {
    let begin: BeginSession = crate::decode("a begin", br#"{"session_id":"s2","delivery":"open_pr"}"#).unwrap();
    assert_eq!(begin.delivery, Some(DeliveryChoice::OpenPr));
    assert_eq!(serde_json::to_value(&begin).unwrap(), json!({"session_id":"s2","delivery":"open_pr"}));
    let bare: BeginSession = crate::decode("a begin", br#"{"session_id":"s2"}"#).unwrap();
    assert_eq!(bare.delivery, None);
    assert_eq!(serde_json::to_value(&bare).unwrap(), json!({"session_id":"s2"}));
    assert!(crate::decode::<BeginSession>("a begin", br#"{"session_id":"s2","delivery":"push_it"}"#).is_err());
}

#[test]
fn a_held_jobs_card_carries_the_first_message_and_a_landing_fleet_defaulted() {
    let card = json!({
        "asks_delivery": true,
        "picked": {"choice": "auto_land", "from": "default"},
        "message": "Fix the reader.\nPointed at: button \"Save\""
    });
    let read: StartHeld = serde_json::from_value(card.clone()).unwrap();
    assert_eq!(read.picked.as_ref().map(|one| one.from), Some(DeliveryPickedFrom::Default));
    assert_eq!(read.message.as_deref(), Some("Fix the reader.\nPointed at: button \"Save\""));
    assert_eq!(serde_json::to_value(&read).unwrap(), card);

    let bare: StartHeld = serde_json::from_value(json!({"asks_delivery": false})).unwrap();
    assert_eq!(bare.message, None);
    assert_eq!(serde_json::to_value(&bare).unwrap(), json!({"asks_delivery": false}));
}

#[test]
fn begin_carries_the_message_as_he_left_it_only_when_given() {
    let begin: BeginSession = crate::decode("a begin", br#"{"session_id":"s2","message":"Fix the reader, then the writer."}"#).unwrap();
    assert_eq!(begin.message.as_deref(), Some("Fix the reader, then the writer."));
    assert_eq!(serde_json::to_value(&begin).unwrap(), json!({"session_id":"s2","message":"Fix the reader, then the writer."}));
    let bare: BeginSession = crate::decode("a begin", br#"{"session_id":"s2"}"#).unwrap();
    assert_eq!(bare.message, None);
}
