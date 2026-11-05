//! The standing orders and the conversation are kept across a restart, and each answer in the
//! conversation counts toward the day's calls.

use crate::tests::{open, TempDir};
use crate::{KeptOrder, KeptTalk};

fn order(form: &str) -> KeptOrder {
    KeptOrder { n: 0, form: form.to_string(), said: format!("said {form}"), until: None, at: "2026-11-05T10:00:00.000Z".to_string() }
}

fn talk(from_person: bool, text: &str, call: Option<&str>, at: &str) -> KeptTalk {
    KeptTalk { from_person, text: text.to_string(), call: call.map(str::to_string), at: at.to_string() }
}

#[test]
fn an_order_stands_until_it_is_removed_and_is_removed_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.add_operator_order(&order("one")).expect("kept");
    let second = store.add_operator_order(&KeptOrder { until: Some("2026-11-06T00:00:00.000Z".to_string()), ..order("two") }).expect("kept");

    let kept = store.operator_orders().expect("read");
    assert_eq!(kept.iter().map(|one| one.n).collect::<Vec<_>>(), vec![first, second]);
    assert_eq!(kept[1].until.as_deref(), Some("2026-11-06T00:00:00.000Z"));

    assert!(store.remove_operator_order(first, "2026-11-05T11:00:00.000Z").expect("removed"));
    assert!(!store.remove_operator_order(first, "2026-11-05T12:00:00.000Z").expect("read"), "already gone");
    assert_eq!(store.operator_orders().expect("read").iter().map(|one| one.n).collect::<Vec<_>>(), vec![second]);
}

#[test]
fn the_conversation_reads_its_last_messages_in_order_and_its_answers_count_as_calls() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_operator_talk(&talk(true, "why?", None, "2026-11-04T09:00:00.000Z")).expect("kept");
    store.add_operator_talk(&talk(false, "yesterday", Some("talk:1"), "2026-11-04T09:00:01.000Z")).expect("kept");
    store.add_operator_talk(&talk(true, "and now?", None, "2026-11-05T09:00:00.000Z")).expect("kept");
    store.add_operator_talk(&talk(false, "today", Some("talk:2"), "2026-11-05T09:00:01.000Z")).expect("kept");

    let last = store.operator_talk(2).expect("read");
    assert_eq!(last.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), vec!["and now?", "today"]);
    assert!(last[0].from_person && !last[1].from_person);

    assert_eq!(store.operator_calls_since("2026-11-05T00:00:00.000Z").expect("counted"), 1, "only today's answer");
}
