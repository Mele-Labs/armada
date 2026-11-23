//! A repository's pull requests are kept as they are looked at, so one is new once: the first look keeps what was open
//! and offers none, a later one answers only the numbers it had not kept, and a reopened store remembers.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-11-20T09:00:00.000Z".to_string())
}

#[test]
fn the_first_look_keeps_what_was_open_and_answers_none_then_only_new_numbers_are_answered() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    assert_eq!(store.review_unseen("m1", &[3, 4], &at()).expect("kept"), Vec::<u64>::new());
    assert_eq!(store.review_unseen("m1", &[3, 4, 5], &at()).expect("kept"), [5]);
    assert_eq!(store.review_unseen("m1", &[3, 4, 5], &at()).expect("kept"), Vec::<u64>::new(), "asked once");
    assert_eq!(store.review_unseen("m2", &[5], &at()).expect("kept"), Vec::<u64>::new(), "another repository starts its own");
}

#[test]
fn a_repository_with_no_pull_request_at_the_first_look_offers_the_first_one_that_opens() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    store.review_unseen("m1", &[], &at()).expect("kept");

    assert_eq!(store.review_unseen("m1", &[1], &at()).expect("kept"), [1]);
}

#[test]
fn what_was_seen_is_remembered_across_a_reopen() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.review_unseen("m1", &[1], &at()).expect("kept");
        store.review_unseen("m1", &[1, 2], &at()).expect("kept");
    }
    let mut store = open(&dir);

    assert_eq!(store.review_unseen("m1", &[1, 2, 3], &at()).expect("kept"), [3]);
}
