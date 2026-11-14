//! The handoff card on the wire: the brief the Job sends, the row the thread keeps, and the figures
//! the card offers, each as Bridge's mirror in `packages/protocol` spells it.

use serde_json::json;

use crate::{handoff_remind_ladder, HandoffBrief, HandoffPick, SessionRow};

#[test]
fn the_ladder_is_the_next_100k_step_above_then_50k_and_100k_past_it() {
    assert_eq!(handoff_remind_ladder(312_000), [400_000, 450_000, 500_000]);
    assert_eq!(handoff_remind_ladder(450_000), [500_000, 550_000, 600_000]);
    assert_eq!(handoff_remind_ladder(400_000), [500_000, 550_000, 600_000], "a step it stands on is not above it");
    assert_eq!(handoff_remind_ladder(0), [100_000, 150_000, 200_000]);
}

#[test]
fn the_brief_reads_with_only_what_is_required_and_writes_nothing_absent() {
    let brief: HandoffBrief = crate::decode("a handoff brief", br#"{"brief":"The fix is on branch fix/reader."}"#).unwrap();
    assert!(brief.session_id.is_none());
    assert_eq!(serde_json::to_value(&brief).unwrap(), json!({"brief":"The fix is on branch fix/reader."}));
}

#[test]
fn an_open_card_round_trips_and_an_answered_one_carries_the_pick() {
    let open = json!({
        "kind": "handoff", "id": "r1", "at": "2026-10-18T08:02:00Z",
        "tokens": 312000, "limit": 300000,
        "brief": "The fix is on branch fix/reader.",
        "remind_at": [400000, 450000, 500000]
    });
    let read: SessionRow = serde_json::from_value(open.clone()).unwrap();
    assert!(matches!(&read, SessionRow::Handoff { answer: None, .. }));
    assert_eq!(read.id(), "r1");
    assert_eq!(serde_json::to_value(&read).unwrap(), open);

    let mut answered = open;
    answered["answer"] = json!({"kind": "remind_at", "tokens": 450000});
    let read: SessionRow = serde_json::from_value(answered.clone()).unwrap();
    assert!(matches!(&read, SessionRow::Handoff { answer: Some(HandoffPick::RemindAt { tokens: 450_000 }), .. }));
    assert_eq!(serde_json::to_value(&read).unwrap(), answered);
    assert_eq!(serde_json::to_value(HandoffPick::CarryOn).unwrap(), json!({"kind": "carry_on"}));
}
