//! What a Job holds in the states of its lifecycles: taken on once, linked, read, and every move
//! kept in the order it was made.

use crate::tests::{open, TempDir};
use crate::{KeptRead, KeptSession, SessionFigures, SessionState};

fn session(id: &str, state: SessionState) -> KeptSession {
    KeptSession {
        id: id.into(),
        harness: "a_harness".into(),
        origin: "bridge".into(),
        manifest_id: Some("armada".into()),
        cwd: "/repos/armada".into(),
        title: None,
        state,
        started_at: "2026-10-29T09:00:00.000Z".into(),
        last_seen_at: "2026-10-29T09:00:00.000Z".into(),
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        figures: SessionFigures::default(),
        mod_version: None,
    }
}

fn read<'a>(key: &'a str, seen: &'a str, moved: Option<(&'a str, &'a str)>, at: &'a str) -> KeptRead<'a> {
    KeptRead { session_id: "s1", lifecycle: "ticket", key, seen_source: "tracker", seen, moved, at }
}

#[test]
fn a_job_marked_after_the_table_existed_takes_instances_on_and_one_never_marked_holds_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(!store.is_lifecycle_job("old").expect("reads"));
    assert!(store.mark_lifecycle_job("s1", "t0").expect("marks"));
    assert!(!store.mark_lifecycle_job("s1", "t1").expect("marks again"), "marked once");
    assert!(store.is_lifecycle_job("s1").expect("reads"));
    assert!(store.lifecycle_instances_of("old").expect("reads").is_empty());
}

#[test]
fn a_thing_is_taken_on_once_and_a_link_it_was_not_given_yet_is_added_in_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.take_on_lifecycle_instance("s1", "ticket", "ARM-41", &[], "t0").expect("ticket"));
    assert!(store.take_on_lifecycle_instance("s1", "ticket", "ARM-42", &[], "t0").expect("ticket"));
    assert!(store.take_on_lifecycle_instance("s1", "pr", "812", &[("ticket", "ARM-41")], "t1").expect("pr"));
    assert!(!store.take_on_lifecycle_instance("s1", "pr", "812", &[("ticket", "ARM-42")], "t2").expect("again"));

    let held = store.lifecycle_instances_of("s1").expect("reads");
    let keys: Vec<(&str, &str)> = held.iter().map(|one| (one.lifecycle.as_str(), one.key.as_str())).collect();
    assert_eq!(keys, [("ticket", "ARM-41"), ("ticket", "ARM-42"), ("pr", "812")]);
    assert_eq!(held[2].taken_on_at, "t1", "taking it on again changed nothing");
    assert_eq!(
        held[2].links,
        [("ticket".to_string(), "ARM-41".to_string()), ("ticket".to_string(), "ARM-42".to_string())]
    );
    assert!(held[0].links.is_empty());
    assert_eq!((held[0].seen_source.as_str(), held[0].seen.as_str()), ("", ""), "not read yet");
}

#[test]
fn a_read_keeps_what_was_seen_and_a_move_only_where_the_standing_changed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.take_on_lifecycle_instance("s1", "ticket", "ARM-41", &[], "t0").expect("taken on");

    store.keep_lifecycle_read(&read("ARM-41", "In Progress", Some(("unread", "unmapped:In Progress")), "t1")).expect("first");
    store.keep_lifecycle_read(&read("ARM-41", "Code Review", Some(("unmapped:In Progress", "in:in_review")), "t2")).expect("second");
    store.keep_lifecycle_read(&read("ARM-41", "In Review", None, "t3")).expect("same standing, other words");

    let held = store.lifecycle_instances_of("s1").expect("reads");
    assert_eq!((held[0].seen_source.as_str(), held[0].seen.as_str()), ("tracker", "In Review"));
    let moves = store.lifecycle_moves_of("s1").expect("reads");
    let told: Vec<(&str, &str, &str, &str)> =
        moves.iter().map(|one| (one.from.as_str(), one.to.as_str(), one.seen.as_str(), one.at.as_str())).collect();
    assert_eq!(
        told,
        [
            ("unread", "unmapped:In Progress", "In Progress", "t1"),
            ("unmapped:In Progress", "in:in_review", "Code Review", "t2"),
        ]
    );
    assert!(moves[0].id < moves[1].id);
    assert!(store.lifecycle_moves_of("s2").expect("reads").is_empty(), "another Job's moves are its own");
}

#[test]
fn a_reader_asks_after_the_live_jobs_that_hold_a_lifecycle_and_after_what_a_watch_follows() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session(&session("s1", SessionState::Live)).expect("kept");
    store.keep_session(&session("s2", SessionState::Ended)).expect("kept");
    for job in ["s1", "s2"] {
        store.take_on_lifecycle_instance(job, "pr", "7", &[], "t0").expect("taken on");
    }
    store.take_on_lifecycle_instance("s1", "deploy", "web", &[], "t0").expect("taken on");
    assert_eq!(store.live_lifecycle_holders("pr").expect("reads"), ["s1"], "an ended Job is not read again");

    store.follow_lifecycle_instance_with_watch("s1", "deploy", "web", "w1").expect("followed");
    let followed = store.lifecycle_instances_followed_by("w1").expect("reads");
    assert_eq!(followed.iter().map(|one| one.key.as_str()).collect::<Vec<_>>(), ["web"]);
    assert!(store.lifecycle_instances_followed_by("w2").expect("reads").is_empty());
}
