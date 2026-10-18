//! Usage kept a turn at a time: a turn is one row per session and message id, reporting it twice
//! changes nothing, and a read by session and time window returns the model, the four token fields
//! and the names of the tools the turn called.

use crate::session_turn_usage::KeptTurnUsage;
use crate::tests::{open, TempDir};

fn at(minute: u32) -> String {
    format!("2026-10-17T09:{minute:02}:00.000Z")
}

fn turn(message_id: &str, minute: u32, tools: &[&str]) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: message_id.into(),
        at: at(minute),
        model: "model-a".into(),
        input_tokens: 6,
        cache_creation_tokens: 1000,
        cache_read_tokens: 10_000,
        output_tokens: 90,
        tools: tools.iter().map(|one| one.to_string()).collect(),
        subagent: false,
        bash_use: None,
    }
}

#[test]
fn a_turn_is_kept_once_and_read_back_whole_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_turn_usage("s1", &turn("m1", 1, &["Bash", "Read"])).unwrap());
    assert!(!store.keep_turn_usage("s1", &turn("m1", 1, &["Bash", "Read"])).unwrap(), "the same turn again");
    assert!(store.keep_turn_usage("s2", &turn("m1", 1, &[])).unwrap(), "the same message id in another session");
    drop(store);

    let store = open(&dir);
    let read = store.turn_usage("s1", None, None).unwrap();
    assert_eq!(read, [turn("m1", 1, &["Bash", "Read"])]);
    assert_eq!(store.turn_usage("s2", None, None).unwrap(), [turn("m1", 1, &[])]);
    assert!(store.turn_usage("s3", None, None).unwrap().is_empty());
}

#[test]
fn a_read_takes_the_window_from_its_start_to_before_its_end_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, minute) in [("m3", 3), ("m1", 1), ("m2", 2), ("m4", 4)] {
        store.keep_turn_usage("s1", &turn(id, minute, &["Edit"])).unwrap();
    }
    let ids = |since: Option<&str>, until: Option<&str>| -> Vec<String> {
        store.turn_usage("s1", since, until).unwrap().into_iter().map(|one| one.message_id).collect()
    };
    assert_eq!(ids(None, None), ["m1", "m2", "m3", "m4"]);
    assert_eq!(ids(Some(&at(2)), Some(&at(4))), ["m2", "m3"]);
    assert_eq!(ids(Some(&at(3)), None), ["m3", "m4"]);
    assert_eq!(ids(None, Some(&at(2))), ["m1"]);
}

#[test]
fn a_turn_heard_again_with_more_output_replaces_the_partial_one_and_never_the_reverse() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let partial = KeptTurnUsage { output_tokens: 3, ..turn("m1", 1, &[]) };
    let whole = KeptTurnUsage { output_tokens: 90, ..turn("m1", 1, &["Bash"]) };
    assert!(store.keep_turn_usage("s1", &partial).unwrap());
    assert!(store.keep_turn_usage("s1", &whole).unwrap());
    assert!(!store.keep_turn_usage("s1", &partial).unwrap());
    assert_eq!(store.turn_usage("s1", None, None).unwrap(), [whole]);
}

#[test]
fn the_lines_of_one_message_heard_apart_join_their_calls_and_the_first_call_keeps_the_label() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = KeptTurnUsage { bash_use: Some("read_search".into()), ..turn("m1", 1, &["Bash"]) };
    let second = KeptTurnUsage { subagent: true, ..turn("m1", 1, &["Read"]) };
    assert!(store.keep_turn_usage("s1", &first).unwrap());
    assert!(store.keep_turn_usage("s1", &second).unwrap());
    assert!(!store.keep_turn_usage("s1", &first).unwrap(), "the first heard again is no change");
    let held = store.turn_usage("s1", None, None).unwrap();
    assert_eq!(held[0].tools, ["Bash", "Read"]);
    assert_eq!(held[0].bash_use.as_deref(), Some("read_search"));
    assert!(held[0].subagent);
}

#[test]
fn a_transcript_offset_is_kept_with_its_turns_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.transcript_offset("/a.jsonl").unwrap(), None);
    assert_eq!(store.keep_turns_read("s1", &[turn("m1", 1, &[])], "/a.jsonl", 120).unwrap(), 1);
    assert_eq!(store.keep_turns_read("s1", &[turn("m1", 1, &[])], "/a.jsonl", 300).unwrap(), 0);
    drop(store);
    assert_eq!(open(&dir).transcript_offset("/a.jsonl").unwrap(), Some(300));
}
