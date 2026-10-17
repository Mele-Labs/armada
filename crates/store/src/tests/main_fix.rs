//! A Session's take of a red main is kept once, read back by red, and ended by
//! the green that follows.

use core_model::Timestamp;

use crate::tests::{open, TempDir};
use crate::{MainFix, TakenBy, TakenHow};

fn at(text: &str) -> Timestamp {
    Timestamp::from_rfc3339(text)
}

fn take(session: &str, how: TakenHow) -> MainFix {
    MainFix {
        repository: "/repos/armada".to_string(),
        red_at: at("2026-10-06T10:00:00.000Z"),
        by: TakenBy::Session(session.to_string()),
        forked_from: None,
        how,
        check: "test".to_string(),
        test: Some("tests::one".to_string()),
        merge: Some(1839),
        taken_at: at("2026-10-06T10:05:00.000Z"),
        ended_at: None,
        fixed_in: None,
    }
}

#[test]
fn a_session_takes_a_red_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = take("aaaa-1111", TakenHow::Took);
    assert!(store.keep_main_fix(&first).expect("kept"));
    assert!(!store
        .keep_main_fix(&MainFix {
            how: TakenHow::SentBack,
            ..first.clone()
        })
        .expect("refused quietly"));
    assert_eq!(
        store
            .main_fixes_of("/repos/armada", &first.red_at)
            .expect("reads"),
        [first.clone()]
    );
}

#[test]
fn a_green_ends_every_take_and_only_the_session_that_fixed_it_says_so() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let fixer = MainFix {
        forked_from: Some("bbbb-2222".to_string()),
        ..take("aaaa-1111", TakenHow::Dispatched)
    };
    let other = take("cccc-3333", TakenHow::SentBack);
    store.keep_main_fix(&fixer).expect("kept");
    store.keep_main_fix(&other).expect("kept");
    let ended = at("2026-10-06T11:00:00.000Z");
    store
        .end_main_fixes("/repos/armada", &ended, Some((fixer.by.id(), 1841)))
        .expect("ended");
    let read = store
        .main_fixes_of("/repos/armada", &fixer.red_at)
        .expect("reads");
    let fixed = |id: &str| {
        let row = read.iter().find(|one| one.by.id() == id).expect("a row");
        (row.ended_at.clone(), row.fixed_in)
    };
    assert_eq!(fixed("aaaa-1111"), (Some(ended.clone()), Some(1841)));
    assert_eq!(fixed("cccc-3333"), (Some(ended), None));
}
