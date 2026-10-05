//! The merge line survives a restart: every case closes the store and opens
//! the file again, because that is what a Fleet restart is to it.

use core_model::Timestamp;

use crate::tests::{job_id, open, top_level, TempDir};
use crate::{Blame, Ended, LineState, Store, TurnHolder};

const ROOT: &str = "/repos/storefront";

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-10-05T09:00:00.000Z")
}

fn a_job(store: &mut Store, id: &str) -> core_model::JobId {
    store
        .insert_job(&top_level(id), &crate::tests::created_at())
        .expect("the job is stored");
    job_id(id)
}

fn holder(run: &str, pid: u32) -> TurnHolder {
    TurnHolder {
        run: run.to_string(),
        pid,
        started: String::from("Mon Oct  5 08:00:00 2026"),
    }
}

/// Entries come back in the order they joined, each with the nonce it was
/// given, and one joined again is the entry it already was.
#[test]
fn the_line_keeps_each_entrys_place_and_nonce_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = a_job(&mut store, "01LINE000000000000000001");
    let second = a_job(&mut store, "01LINE000000000000000002");
    let one = store
        .join_the_line(
            ROOT,
            &first,
            "nonce-1",
            "human",
            "https://forge.invalid/pull/1",
            &at(),
        )
        .expect("joins");
    let two = store
        .join_the_line(
            ROOT,
            &second,
            "nonce-2",
            "fleet",
            "https://forge.invalid/pull/2",
            &at(),
        )
        .expect("joins");
    drop(store);

    let mut store = open(&dir);
    let waiting = store.waiting_in_line(ROOT).expect("reads");
    assert_eq!(waiting, vec![one.clone(), two.clone()]);
    assert!(one.id < two.id);
    let again = store
        .join_the_line(
            ROOT,
            &first,
            "nonce-3",
            "human",
            "https://forge.invalid/pull/1",
            &at(),
        )
        .expect("a press made again");
    assert_eq!(again, one, "it keeps the place and the nonce it had");
}

/// An outcome is written to the entry it was taken for and nowhere else, and is
/// there after a reopen; a landing is finished once.
#[test]
fn an_outcome_is_written_once_to_its_own_entry_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(&mut store, "01LINE000000000000000003");
    let entry = store
        .join_the_line(
            ROOT,
            &job,
            "nonce-a",
            "human",
            "https://forge.invalid/pull/3",
            &at(),
        )
        .expect("joins");
    let landed = Ended {
        state: LineState::Landed,
        kind: None,
        said: None,
        merge_commit: Some(String::from("abc123")),
        base: Some(String::from("main")),
        blamed: None,
    };

    assert!(
        !store
            .end_line_entry(entry.id, "another", &landed, &at())
            .expect("writes"),
        "not another entry's nonce"
    );
    assert!(store
        .end_line_entry(entry.id, "nonce-a", &landed, &at())
        .expect("writes"));
    assert!(
        !store
            .end_line_entry(entry.id, "nonce-a", &landed, &at())
            .expect("writes"),
        "an entry ends once"
    );
    drop(store);

    let mut store = open(&dir);
    assert!(store.waiting_in_line(ROOT).expect("reads").is_empty());
    let read = store.line_entry(entry.id).expect("reads").expect("kept");
    assert_eq!(read.state, LineState::Landed);
    assert_eq!(read.merge_commit.as_deref(), Some("abc123"));
    assert!(!read.finished);
    assert_eq!(store.landings_not_finished(ROOT).expect("reads").len(), 1);
    assert_eq!(
        store.lines_with_work().expect("reads"),
        vec![ROOT.to_string()]
    );
    assert!(store.finish_line_entry(entry.id).expect("writes"));
    assert!(!store.finish_line_entry(entry.id).expect("writes"));
    assert!(store.lines_with_work().expect("reads").is_empty());
}

/// A refusal keeps its kind, its sentence and whose failure it was.
#[test]
fn a_refusal_keeps_its_kind_its_sentence_and_its_blame() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(&mut store, "01LINE000000000000000004");
    let entry = store
        .join_the_line(
            ROOT,
            &job,
            "n",
            "human",
            "https://forge.invalid/pull/4",
            &at(),
        )
        .expect("joins");
    let refused = Ended {
        state: LineState::Refused,
        kind: Some(String::from("gate_failed")),
        said: Some(String::from("suite did not pass")),
        merge_commit: None,
        base: None,
        blamed: Some(Blame::Base),
    };
    store
        .end_line_entry(entry.id, "n", &refused, &at())
        .expect("writes");
    drop(store);

    let store = open(&dir);
    let read = store.line_entry(entry.id).expect("reads").expect("kept");
    assert_eq!(read.kind.as_deref(), Some("gate_failed"));
    assert_eq!(read.blamed, Some(Blame::Base));
    assert!(read.ended_at.is_some());
}

/// The turn is a row: held after a reopen, refused to a second taker, and
/// moved only by the compare-and-swap that names who held it.
#[test]
fn the_turn_outlives_the_process_that_took_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(
        store
            .hold_turn(ROOT, &holder("run-1", 100), &at())
            .expect("takes"),
        None
    );
    drop(store);

    let mut store = open(&dir);
    let held = store.turn_held(ROOT).expect("reads").expect("still held");
    assert_eq!(held.run, "run-1");
    assert_eq!(
        store
            .hold_turn(ROOT, &holder("run-2", 200), &at())
            .expect("asks"),
        Some(held),
        "a second taker is told who holds it"
    );
    assert!(!store
        .take_over_turn(ROOT, "run-9", &holder("run-2", 200), &at())
        .expect("writes"));
    assert!(store
        .take_over_turn(ROOT, "run-1", &holder("run-2", 200), &at())
        .expect("writes"));
    store
        .release_turn(ROOT, "run-1")
        .expect("not the holder, so nothing");
    assert_eq!(
        store.turn_held(ROOT).expect("reads").expect("held").run,
        "run-2"
    );
    store.release_turn(ROOT, "run-2").expect("releases");
    assert_eq!(store.turn_held(ROOT).expect("reads"), None);
}

/// How many a turn takes is kept whole and read back by the next Fleet.
#[test]
fn the_size_a_turn_takes_is_kept_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.line_size(ROOT).expect("reads"), None);
    store
        .keep_line_size(ROOT, 8, "the first turn takes the most", &at())
        .expect("writes");
    store
        .keep_line_size(ROOT, 4, "halved after a red", &at())
        .expect("writes");
    drop(store);

    let store = open(&dir);
    let kept = store.line_size(ROOT).expect("reads").expect("kept");
    assert_eq!((kept.size, kept.reason.as_str()), (4, "halved after a red"));
}

/// A Job forgotten takes its entry with it, rather than leaving a line
/// pointing at nothing.
#[test]
fn a_forgotten_jobs_entry_goes_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(&mut store, "01LINE000000000000000005");
    store
        .join_the_line(
            ROOT,
            &job,
            "n",
            "human",
            "https://forge.invalid/pull/5",
            &at(),
        )
        .expect("joins");

    store.forget_job(&job).expect("forgets");

    assert!(store.waiting_in_line(ROOT).expect("reads").is_empty());
}

/// Why an entry waits and where the next turn takes it are kept, and a restart
/// reads them back as they were.
#[test]
fn why_an_entry_waits_and_its_position_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let ids: Vec<_> = (1..=3)
        .map(|n| {
            let job = a_job(&mut store, &format!("01LINE00000000000000010{n}"));
            store
                .join_the_line(
                    ROOT,
                    &job,
                    "n",
                    "human",
                    "https://forge.invalid/pull/1",
                    &at(),
                )
                .expect("joins")
                .id
        })
        .collect();
    store
        .hold_turn(ROOT, &holder("run-1", 1), &at())
        .expect("takes");
    let late = a_job(&mut store, "01LINE000000000000000199");
    let late = store
        .join_the_line(
            ROOT,
            &late,
            "n",
            "human",
            "https://forge.invalid/pull/9",
            &at(),
        )
        .expect("joins");
    assert_eq!(late.held_back, crate::HeldBack::JoinedAfterTurnBegan);
    store
        .order_the_line(ROOT, &[ids[1]], &[(ids[2], crate::HeldBack::ClashMain)])
        .expect("orders");
    drop(store);

    let store = open(&dir);
    let read = store.waiting_in_line(ROOT).expect("reads");
    let mut seen: Vec<_> = read
        .iter()
        .map(|one| (one.next_position, one.id, one.held_back))
        .collect();
    seen.sort_by_key(|one| one.0);
    assert_eq!(
        seen,
        vec![
            (Some(1), ids[1], crate::HeldBack::None),
            (Some(2), ids[0], crate::HeldBack::None),
            (Some(3), ids[2], crate::HeldBack::ClashMain),
            (Some(4), late.id, crate::HeldBack::JoinedAfterTurnBegan),
        ]
    );
}
