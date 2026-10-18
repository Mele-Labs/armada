//! The turns a token rule reads are the last hour of one session and its latest main-thread turns
//! beyond it, oldest first; a trip is a row, and a session's last trip of a rule is found by it.

use crate::usage_rule_trips::KeptTrip;
use crate::session_turn_usage::KeptTurnUsage;
use crate::tests::{open, TempDir};

fn turn(id: &str, hour: u32, minute: u32, subagent: bool) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: format!("2026-10-18T{hour:02}:{minute:02}:00.000Z"),
        model: "model-a".into(),
        input_tokens: 1,
        cache_creation_tokens: 1,
        cache_read_tokens: 1,
        output_tokens: 1,
        tools: Vec::new(),
        subagent,
        bash_use: None,
    }
}

#[test]
fn the_last_hour_and_the_latest_main_turns_beyond_it_are_read_for_one_session_only() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, hour, minute, sub) in [("a", 5, 0, false), ("b", 5, 10, true), ("c", 6, 0, false), ("d", 8, 0, false), ("e", 9, 30, false), ("f", 9, 45, true)] {
        store.keep_turn_usage("s1", &turn(id, hour, minute, sub)).unwrap();
    }
    store.keep_turn_usage("s2", &turn("x", 9, 50, false)).unwrap();
    let ids = |main: usize| -> Vec<String> {
        store.turns_for_rules("s1", main).unwrap().into_iter().map(|one| one.message_id).collect()
    };
    assert_eq!(ids(0), ["e", "f"], "the hour up to the newest, which is f at 09:45");
    assert_eq!(ids(2), ["d", "e", "f"], "the latest main turn beyond it joins");
    assert_eq!(ids(10), ["a", "c", "d", "e", "f"], "a subagent's turn outside the hour does not");
    assert!(store.turns_for_rules("none", 10).unwrap().is_empty());
}

#[test]
fn a_trip_is_kept_once_and_its_rule_found_by_the_latest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let trip = |rule: &str, at: &str| KeptTrip {
        session_id: "s1".into(),
        rule: rule.into(),
        at: at.into(),
        figure: 312_000.0,
        burn_per_hour_before: 4.5,
    };
    store.keep_usage_rule_trip(&trip("context", "2026-10-18T08:00:00.000Z")).unwrap();
    store.keep_usage_rule_trip(&trip("context", "2026-10-18T08:00:00.000Z")).unwrap();
    store.keep_usage_rule_trip(&trip("context", "2026-10-18T09:00:00.000Z")).unwrap();
    store.keep_usage_rule_trip(&trip("cache_hit", "2026-10-18T10:00:00.000Z")).unwrap();
    assert_eq!(store.usage_rule_trips("s1").unwrap().len(), 3);
    assert_eq!(store.last_usage_rule_trip("s1", "context").unwrap().as_deref(), Some("2026-10-18T09:00:00.000Z"));
    assert_eq!(store.last_usage_rule_trip("s1", "subagents").unwrap(), None);
    assert_eq!(store.last_usage_rule_trip("s2", "context").unwrap(), None);
}
