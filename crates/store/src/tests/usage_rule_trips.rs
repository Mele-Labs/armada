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
        burn_per_hour_after: None,
        after_turns: None,
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

#[test]
fn a_trip_is_measured_once_its_hour_is_read_and_the_window_lists_newest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (session, rule, at) in [("s1", "context", "2026-10-18T08:00:00.000Z"), ("s2", "cache_hit", "2026-10-18T09:00:00.000Z"), ("s1", "context", "2026-10-11T08:00:00.000Z")] {
        store
            .keep_usage_rule_trip(&KeptTrip {
                session_id: session.into(),
                rule: rule.into(),
                at: at.into(),
                figure: 1.0,
                burn_per_hour_before: 2.0,
                burn_per_hour_after: None,
                after_turns: None,
            })
            .unwrap();
    }
    assert_eq!(store.unmeasured_usage_rule_trips("s1").unwrap().len(), 2);
    store.keep_usage_rule_trip_measured("s1", "context", "2026-10-18T08:00:00.000Z", Some(1.5), 7).unwrap();
    store.keep_usage_rule_trip_measured("s1", "context", "2026-10-11T08:00:00.000Z", None, 3).unwrap();
    assert!(store.unmeasured_usage_rule_trips("s1").unwrap().is_empty(), "a measure with no reading is still measured");
    let all = store.usage_rule_trips("s1").unwrap();
    assert_eq!((all[0].burn_per_hour_after, all[0].after_turns), (None, Some(3)));
    assert_eq!((all[1].burn_per_hour_after, all[1].after_turns), (Some(1.5), Some(7)));

    let (since, until, listed) =
        store.usage_rule_trips_between(None, Some("2026-10-18T12:00:00Z"), "2026-10-19T00:00:00Z").unwrap().unwrap();
    assert_eq!((since.as_str(), until.as_str()), ("2026-10-11T12:00:00.000Z", "2026-10-18T12:00:00.000Z"));
    let ats: Vec<&str> = listed.iter().map(|one| one.at.as_str()).collect();
    assert_eq!(ats, ["2026-10-18T09:00:00.000Z", "2026-10-18T08:00:00.000Z"], "newest first, the older one out of the week");
    assert!(store.usage_rule_trips_between(Some("lately"), None, "2026-10-19T00:00:00Z").unwrap().is_none());
}

#[test]
fn the_hour_after_an_instant_is_its_turns_up_to_60_minutes_on() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, hour, minute) in [("a", 8, 0), ("b", 8, 1), ("c", 9, 0), ("d", 9, 1)] {
        store.keep_turn_usage("s1", &turn(id, hour, minute, false)).unwrap();
    }
    let ids: Vec<String> = store
        .turns_in_the_hour_after("s1", "2026-10-18T08:00:00.000Z")
        .unwrap()
        .into_iter()
        .map(|one| one.message_id)
        .collect();
    assert_eq!(ids, ["b", "c"], "after the instant, up to and including the hour on");
}
