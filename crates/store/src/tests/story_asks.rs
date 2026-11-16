//! An ask reads back with its words, features, merges and hands; a correction of his is the newest first; an ask removed leaves its
//! words as talk.

use crate::tests::{open, TempDir};
use crate::{ask_state, message_kind, sorted_by, KeptAsk, KeptHandoff, KeptMerge, KeptMessage};

fn ask(id: &str, session: &str, at: &str) -> KeptAsk {
    KeptAsk {
        ask_id: id.to_string(),
        session_id: session.to_string(),
        manifest_id: Some("armada".to_string()),
        project: Some("p1".to_string()),
        kind: "bug".to_string(),
        title: "Votes show as never cast".to_string(),
        state: ask_state::BUILDING.to_string(),
        building: Some(session.to_string()),
        opened_at: at.to_string(),
        state_at: at.to_string(),
    }
}

fn message(session: &str, row: &str, kind: &str, ask: Option<&str>, by: &str, at: &str) -> KeptMessage {
    KeptMessage {
        session_id: session.to_string(),
        row_id: row.to_string(),
        kind: kind.to_string(),
        ask_id: ask.map(String::from),
        words: format!("words of {row}"),
        at: at.to_string(),
        sorted_by: by.to_string(),
        sorted_at: at.to_string(),
    }
}

fn ids(asks: Vec<KeptAsk>) -> Vec<String> {
    asks.into_iter().map(|one| one.ask_id).collect()
}

#[test]
fn an_ask_reads_back_after_a_reopen_with_its_features_merges_and_hands() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_ask_features("a1", &["voting".to_string(), "pages".to_string(), "voting".to_string()]).unwrap();
    let merge = KeptMerge { ask_id: "a1".into(), pull_request: "3720".into(), merged_at: "2026-11-16T11:00:00.000Z".into() };
    assert!(store.keep_ask_merge(&merge).unwrap());
    assert!(!store.keep_ask_merge(&merge).unwrap(), "a merge is kept once");
    let hand = KeptHandoff { ask_id: "a1".into(), from_session: "s1".into(), to_session: "s2".into(), at: "2026-11-16T10:30:00.000Z".into() };
    assert!(store.keep_ask_handoff(&hand).unwrap());
    drop(store);

    let store = open(&dir);
    assert_eq!(store.ask("a1").unwrap(), Some(ask("a1", "s1", "2026-11-16T10:00:00.000Z")));
    assert_eq!(store.features_of_ask("a1").unwrap(), ["pages", "voting"]);
    assert_eq!(store.merges_of_ask("a1").unwrap(), vec![merge]);
    assert_eq!(store.handoffs_of_ask("a1").unwrap(), vec![hand]);
}

#[test]
fn a_keep_moves_the_state_and_the_builder_and_leaves_where_it_was_said() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    let mut moved = ask("a1", "s9", "2026-11-16T12:00:00.000Z");
    moved.state = ask_state::HANDED.to_string();
    moved.building = Some("s2".to_string());
    store.keep_ask(&moved).unwrap();
    let kept = store.ask("a1").unwrap().unwrap();
    assert_eq!((kept.session_id.as_str(), kept.opened_at.as_str()), ("s1", "2026-11-16T10:00:00.000Z"));
    assert_eq!((kept.state.as_str(), kept.building.as_deref(), kept.state_at.as_str()), ("handed", Some("s2"), "2026-11-16T12:00:00.000Z"));
}

#[test]
fn a_jobs_asks_are_those_said_in_it_built_by_it_or_handed_through_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("said", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    let mut built = ask("built", "s2", "2026-11-16T10:01:00.000Z");
    built.building = Some("s1".to_string());
    store.keep_ask(&built).unwrap();
    store.keep_ask(&ask("other", "s3", "2026-11-16T10:02:00.000Z")).unwrap();
    assert_eq!(ids(store.asks_of_session("s1").unwrap()), ["said", "built"]);
    assert_eq!(ids(store.asks_of_session("s3").unwrap()), ["other"]);
    let hand = KeptHandoff { ask_id: "other".into(), from_session: "s3".into(), to_session: "s1".into(), at: "2026-11-16T10:03:00.000Z".into() };
    store.keep_ask_handoff(&hand).unwrap();
    assert_eq!(ids(store.asks_of_session("s1").unwrap()), ["said", "built", "other"]);
}

#[test]
fn a_projects_asks_narrow_to_a_feature_and_an_ask_on_two_shows_on_both() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_ask(&ask("a2", "s1", "2026-11-16T10:01:00.000Z")).unwrap();
    store.keep_ask_features("a1", &["voting".to_string(), "pages".to_string()]).unwrap();
    store.keep_ask_features("a2", &["pages".to_string()]).unwrap();
    assert_eq!(ids(store.asks_of_project("p1", None).unwrap()), ["a1", "a2"]);
    assert_eq!(ids(store.asks_of_project("armada", Some("voting")).unwrap()), ["a1"]);
    assert_eq!(ids(store.asks_of_project("p1", Some("pages")).unwrap()), ["a1", "a2"]);
    assert!(store.asks_of_project("elsewhere", None).unwrap().is_empty());
}

#[test]
fn only_a_building_or_handed_ask_with_a_builder_is_in_work() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    let mut backlog = ask("a2", "s1", "2026-11-16T10:01:00.000Z");
    backlog.state = ask_state::BACKLOG.to_string();
    backlog.building = None;
    store.keep_ask(&backlog).unwrap();
    let mut built = ask("a3", "s1", "2026-11-16T10:02:00.000Z");
    built.state = ask_state::BUILT.to_string();
    store.keep_ask(&built).unwrap();
    assert_eq!(ids(store.asks_in_work().unwrap()), ["a1"]);
}

#[test]
fn a_message_sorting_is_replaced_by_a_correction_and_the_newest_corrections_come_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_message(&message("s1", "m1", message_kind::TALK, None, sorted_by::FLEET, "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_message(&message("s1", "m2", message_kind::NEW_ASK, Some("a1"), sorted_by::FLEET, "2026-11-16T10:01:00.000Z")).unwrap();
    store.keep_message(&message("s1", "m3", message_kind::REVISION, Some("a1"), sorted_by::FLEET, "2026-11-16T10:02:00.000Z")).unwrap();
    assert!(store.corrections(5).unwrap().is_empty(), "what Fleet sorted is no correction");

    store.keep_message(&message("s1", "m1", message_kind::NEW_ASK, Some("a1"), sorted_by::OWNER, "2026-11-16T10:05:00.000Z")).unwrap();
    store.keep_message(&message("s1", "m3", message_kind::ANSWER, Some("a1"), sorted_by::OWNER, "2026-11-16T10:06:00.000Z")).unwrap();
    let corrections = store.corrections(5).unwrap();
    let said: Vec<(&str, &str, Option<&str>)> = corrections.iter().map(|one| (one.words.as_str(), one.kind.as_str(), one.ask_kind.as_deref())).collect();
    assert_eq!(said, [("words of m3", "answer", None), ("words of m1", "new_ask", Some("bug"))]);
    assert_eq!(store.corrections(1).unwrap().len(), 1);
    assert_eq!(store.message_sorted("s1", "m1").unwrap().unwrap().sorted_by, "owner");
    let of_ask: Vec<String> = store.messages_of_ask("a1").unwrap().into_iter().map(|one| one.row_id).collect();
    assert_eq!(of_ask, ["m1", "m2", "m3"]);
}

#[test]
fn an_ask_removed_leaves_its_words_as_talk_and_takes_its_features_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_ask(&ask("a1", "s1", "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_ask_features("a1", &["voting".to_string()]).unwrap();
    store.keep_message(&message("s1", "m1", message_kind::NEW_ASK, Some("a1"), sorted_by::FLEET, "2026-11-16T10:00:00.000Z")).unwrap();
    store.remove_ask("a1").unwrap();
    assert_eq!(store.ask("a1").unwrap(), None);
    assert!(store.features_of_ask("a1").unwrap().is_empty());
    let kept = store.message_sorted("s1", "m1").unwrap().unwrap();
    assert_eq!((kept.kind.as_str(), kept.ask_id), ("talk", None));
}

#[test]
fn the_words_of_an_edited_message_are_replaced() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_message(&message("s1", "m1", message_kind::TALK, None, sorted_by::FLEET, "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_message_words("s1", "m1", "new words").unwrap();
    assert_eq!(store.message_sorted("s1", "m1").unwrap().unwrap().words, "new words");
}

#[test]
fn a_delivered_message_is_taken_once_for_sorting_and_shows_in_no_list_until_it_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_message_unsorted(&message("s1", "m1", message_kind::UNSORTED, None, sorted_by::FLEET, "2026-11-16T10:00:00.000Z")).unwrap();
    store.keep_message_unsorted(&message("s1", "m2", message_kind::UNSORTED, None, sorted_by::FLEET, "2026-11-16T10:01:00.000Z")).unwrap();
    assert!(store.messages_sorted_in("s1").unwrap().is_empty(), "a message waiting is not a sorted one");

    let taken = store.take_unsorted(1, "2026-11-16T10:02:00.000Z").unwrap();
    assert_eq!(taken.iter().map(|one| one.row_id.as_str()).collect::<Vec<_>>(), ["m1"]);
    let again = store.take_unsorted(5, "2026-11-16T10:02:01.000Z").unwrap();
    assert_eq!(again.iter().map(|one| one.row_id.as_str()).collect::<Vec<_>>(), ["m2"], "m1 is already taken");

    let read = message("s1", "m1", message_kind::TALK, None, sorted_by::FLEET, "2026-11-16T10:03:00.000Z");
    assert!(store.keep_message_read(&read).unwrap());
    assert!(!store.keep_message_read(&read).unwrap(), "a read is kept once");
    assert_eq!(store.messages_sorted_in("s1").unwrap().len(), 1);

    store.drop_message_read("s1", "m2").unwrap();
    assert_eq!(store.message_sorted("s1", "m2").unwrap(), None);
}

#[test]
fn a_message_he_sorted_while_the_model_read_it_is_not_overwritten() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_message_unsorted(&message("s1", "m1", message_kind::UNSORTED, None, sorted_by::FLEET, "2026-11-16T10:00:00.000Z")).unwrap();
    store.take_unsorted(5, "2026-11-16T10:00:01.000Z").unwrap();
    store.keep_message(&message("s1", "m1", message_kind::ANSWER, None, sorted_by::OWNER, "2026-11-16T10:00:02.000Z")).unwrap();
    let read = message("s1", "m1", message_kind::TALK, None, sorted_by::FLEET, "2026-11-16T10:00:03.000Z");
    assert!(!store.keep_message_read(&read).unwrap());
    store.drop_message_read("s1", "m1").unwrap();
    assert_eq!(store.message_sorted("s1", "m1").unwrap().unwrap().sorted_by, "owner");
}
