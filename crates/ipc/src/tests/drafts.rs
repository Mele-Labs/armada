//! `propose_draft` on the wire: the call the agent sends, the row the thread keeps and the item
//! Waiting on you reads, each as Bridge's mirror in `packages/protocol` spells it.

use serde_json::json;

use crate::{AnswerWaiting, DraftHow, ProposeDraft, SessionRow, WaitingItem, WaitingSource};

#[test]
fn the_call_reads_with_only_what_is_required_and_writes_nothing_absent() {
    let draft: ProposeDraft = crate::decode(
        "a draft to propose",
        br#"{"title":"Pull request description","text":"Adds the thing."}"#,
    )
    .unwrap();
    assert!(draft.before.is_none() && draft.session_id.is_none());
    let back = serde_json::to_value(&draft).unwrap();
    assert_eq!(back, json!({"title":"Pull request description","text":"Adds the thing."}));
}

#[test]
fn an_open_draft_round_trips_with_what_it_replaces() {
    let row = json!({
        "kind": "draft", "id": "r1", "at": "2026-10-11T09:00:00Z",
        "title": "Pull request description",
        "text": "Adds the thing.",
        "before": "WIP"
    });
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    assert!(matches!(&read, SessionRow::Draft { settled: None, .. }));
    assert_eq!(read.id(), "r1");
    assert_eq!(serde_json::to_value(&read).unwrap(), row);
}

#[test]
fn a_settled_draft_round_trips_with_how_and_the_words() {
    let row = json!({
        "kind": "draft", "id": "r1", "at": "2026-10-11T09:00:00Z",
        "title": "Pull request description",
        "text": "Adds the thing.",
        "settled": {"how": "fixed", "words": "Adds the other thing.", "at": "2026-10-11T09:05:00Z"}
    });
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    let SessionRow::Draft { settled: Some(settled), before: None, .. } = &read else {
        panic!("a settled draft: {read:?}");
    };
    assert_eq!(settled.how, DraftHow::Fixed);
    assert_eq!(serde_json::to_value(&read).unwrap(), row);

    let approved = json!({"how": "approved", "at": "2026-10-11T09:05:00Z"});
    let read: crate::DraftSettled = serde_json::from_value(approved.clone()).unwrap();
    assert!(read.words.is_none());
    assert_eq!(serde_json::to_value(&read).unwrap(), approved);
    assert_eq!(serde_json::to_value(DraftHow::SentBack).unwrap(), json!("sent_back"));
}

#[test]
fn the_item_waits_as_a_draft_and_is_answered_with_the_fixed_text() {
    let item = json!({
        "id": "draft:r1", "text": "Pull request description", "since": "2026-10-11T09:00:00Z",
        "source": "draft"
    });
    let read: WaitingItem = serde_json::from_value(item.clone()).unwrap();
    assert_eq!(read.source, WaitingSource::Draft);
    assert_eq!(serde_json::to_value(&read).unwrap(), item);

    let answer = json!({"session_id": "s1", "item_id": "draft:r1", "fixed": "Adds the other thing."});
    let read: AnswerWaiting = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!(read.fixed.as_deref(), Some("Adds the other thing."));
    assert_eq!(serde_json::to_value(&read).unwrap(), answer);
}
