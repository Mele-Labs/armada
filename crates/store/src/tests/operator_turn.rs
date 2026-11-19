use crate::tests::{open, TempDir};
use crate::{KeptPage, KeptTurn};

fn turn(session: &str, item: &str) -> KeptTurn {
    KeptTurn { session: session.to_string(), item: item.to_string(), body: "{}".to_string(), raised_at: "2026-11-10T09:00:00.000Z".to_string(), ..KeptTurn::default() }
}

#[test]
fn an_item_waits_once_and_keeps_the_persons_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let a = store.add_operator_turn(&turn("01JOB", "plan:1")).expect("kept").expect("new");
    assert_eq!(store.add_operator_turn(&turn("01JOB", "plan:1")).expect("kept"), None, "one open row for an item");
    let b = store.add_operator_turn(&turn("01JOB", "walk:x")).expect("kept").expect("new");
    let c = store.add_operator_turn(&turn("", "suggestion:3")).expect("kept").expect("new");
    assert_eq!(store.operator_turn_open().expect("read").iter().map(|one| one.n).collect::<Vec<_>>(), vec![a, b, c]);

    store.order_operator_turn(&[c, 9999, a]).expect("ordered");
    assert_eq!(store.operator_turn_open().expect("read").iter().map(|one| one.n).collect::<Vec<_>>(), vec![c, a, b], "named first, the rest as they were");
}

#[test]
fn a_settled_item_leaves_once_and_may_wait_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let a = store.add_operator_turn(&turn("01JOB", "plan:1")).expect("kept").expect("new");
    assert!(store.later_operator_turn(a, "2026-11-11T05:00:00.000Z").expect("later"));
    assert!(store.answer_operator_turn(a, "yes").expect("answered"));
    assert!(store.settle_operator_turn(a, "2026-11-10T10:00:00.000Z", Some("no")).expect("settled"));
    assert!(!store.settle_operator_turn(a, "2026-11-10T11:00:00.000Z", Some("yes")).expect("read"), "settled once");
    assert!(!store.later_operator_turn(a, "2026-11-12T05:00:00.000Z").expect("read"), "a settled item is not put last");
    assert!(store.operator_turn_open().expect("read").is_empty());
    let kept = store.operator_turn_item(a).expect("read").expect("kept");
    assert_eq!((kept.answer.as_deref(), kept.settled_at.as_deref()), (Some("no"), Some("2026-11-10T10:00:00.000Z")));
    assert_eq!(store.operator_turn_settled_since("2026-11-10T10:00:00.000Z").expect("read").len(), 1);
    assert_eq!(store.operator_turn_settled_since("2026-11-10T10:00:01.000Z").expect("read").len(), 0);
    assert_eq!(store.add_operator_turn(&turn("01JOB", "plan:1")).expect("kept"), None, "an item he settled is not queued again while it is still on its list");
    assert_eq!(store.operator_turn_present().expect("read").len(), 1);
    assert!(store.leave_operator_turn(a, "2026-11-10T12:00:00.000Z").expect("left"));
    assert!(!store.leave_operator_turn(a, "2026-11-10T13:00:00.000Z").expect("read"), "left once");
    assert_eq!(store.operator_turn_item(a).expect("read").expect("kept").settled_at.as_deref(), Some("2026-11-10T10:00:00.000Z"), "leaving keeps the settle it had");
    assert!(store.add_operator_turn(&turn("01JOB", "plan:1")).expect("kept").is_some(), "once it has left, the same item may be raised again");
}

#[test]
fn the_page_keeps_what_it_was_told() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.operator_page().expect("read"), KeptPage::default());
    let page = KeptPage { seen_at: Some("a".into()), looked_at: Some("b".into()), top: Some("c".into()), top_at: Some("d".into()) };
    store.keep_operator_page(&page).expect("kept");
    store.keep_operator_page(&KeptPage { top: Some("e".into()), ..page.clone() }).expect("kept");
    assert_eq!(store.operator_page().expect("read").top.as_deref(), Some("e"));
}

#[test]
fn a_picked_card_keeps_its_steps_comes_back_once_and_may_be_picked_anew() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let a = store.add_operator_turn(&turn("", "card:1")).expect("kept").expect("new");
    let kept = store.operator_turn_item(a).expect("read").expect("kept");
    assert_eq!((kept.picked, kept.picked_at, kept.steps, kept.returned_at), (None, None, None, None), "a row as it was raised reads as not picked");
    assert!(!store.step_operator_turn(a, "2026-11-14T09:00:00.000Z", "Started").expect("read"), "no step on a card not picked");

    assert!(store.pick_operator_turn(a, "Start it", "2026-11-14T09:00:00.000Z").expect("picked"));
    assert!(!store.pick_operator_turn(a, "Wait", "2026-11-14T09:01:00.000Z").expect("read"), "picked once while it is worked on");
    assert!(store.step_operator_turn(a, "2026-11-14T09:02:00.000Z", "Started a Job").expect("step"));
    assert!(store.step_operator_turn(a, "2026-11-14T09:03:00.000Z", "Opened #12").expect("step"));
    let kept = store.operator_turn_item(a).expect("read").expect("kept");
    assert_eq!(kept.picked.as_deref(), Some("Start it"));
    let steps: serde_json::Value = serde_json::from_str(kept.steps.as_deref().expect("steps")).expect("JSON");
    assert_eq!(steps, serde_json::json!([{ "at": "2026-11-14T09:02:00.000Z", "said": "Started a Job" }, { "at": "2026-11-14T09:03:00.000Z", "said": "Opened #12" }]));

    assert_eq!(store.operator_turn_picked_by("2026-11-14T08:59:00.000Z").expect("read").len(), 0, "picked after then");
    assert_eq!(store.operator_turn_picked_by("2026-11-14T09:15:00.000Z").expect("read").len(), 1);
    assert!(store.return_operator_turn(a, "No slot is free", "2026-11-14T09:04:00.000Z").expect("returned"));
    assert!(!store.return_operator_turn(a, "Again", "2026-11-14T09:05:00.000Z").expect("read"), "returned once");
    assert_eq!(store.operator_turn_picked_by("2026-11-14T09:15:00.000Z").expect("read").len(), 0, "a returned card is not worked on");

    assert!(store.pick_operator_turn(a, "Wait", "2026-11-14T09:06:00.000Z").expect("picked anew"));
    let kept = store.operator_turn_item(a).expect("read").expect("kept");
    assert_eq!((kept.picked.as_deref(), kept.returned_why, kept.steps), (Some("Wait"), None, None));
}
