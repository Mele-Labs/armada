//! A Job's take of a red main is kept once, read back by red and by Job, and
//! ended by the green that follows.

use core_model::Timestamp;

use crate::tests::{job_id, open, TempDir};
use crate::{MainFix, TakenBy, TakenHow};

fn at(text: &str) -> Timestamp {
    Timestamp::from_rfc3339(text)
}

fn take(job: &str, how: TakenHow) -> MainFix {
    MainFix {
        repository: "/repos/armada".to_string(),
        red_at: at("2026-10-06T10:00:00.000Z"),
        by: TakenBy::Job(job_id(job)),
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
fn a_job_takes_a_red_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = take("01MAINFIX000000000000001", TakenHow::Took);
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
    assert!(store.is_fixing_main(first.by.job().expect("a Job")).expect("reads"));
}

#[test]
fn a_green_ends_every_take_and_only_the_job_that_fixed_it_says_so() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let fixer = take("01MAINFIX000000000000001", TakenHow::Dispatched);
    let other = take("01MAINFIX000000000000002", TakenHow::SentBack);
    store.keep_main_fix(&fixer).expect("kept");
    store.keep_main_fix(&other).expect("kept");
    let ended = at("2026-10-06T11:00:00.000Z");
    store
        .end_main_fixes("/repos/armada", &ended, Some((fixer.by.job().expect("a Job"), 1841)))
        .expect("ended");
    let read = store
        .main_fix_of_job(fixer.by.job().expect("a Job"))
        .expect("reads")
        .expect("a row");
    assert_eq!(
        (read.ended_at, read.fixed_in),
        (Some(ended.clone()), Some(1841))
    );
    let read = store
        .main_fix_of_job(other.by.job().expect("a Job"))
        .expect("reads")
        .expect("a row");
    assert_eq!((read.ended_at, read.fixed_in), (Some(ended), None));
    assert!(!store.is_fixing_main(fixer.by.job().expect("a Job")).expect("reads"));
}

#[test]
fn a_session_takes_a_red_once_beside_a_job_and_a_green_ends_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let session = MainFix {
        by: TakenBy::Session("aaaa-1111".to_string()),
        forked_from: Some("bbbb-2222".to_string()),
        ..take("01MAINFIX000000000000001", TakenHow::Dispatched)
    };
    assert!(store.keep_main_fix(&session).expect("kept"));
    assert!(!store.keep_main_fix(&session).expect("again"), "once per red per Session");
    let job = take("01MAINFIX000000000000002", TakenHow::Took);
    assert!(store.keep_main_fix(&job).expect("kept"));
    let read = store.main_fixes_of("/repos/armada", &session.red_at).expect("reads");
    assert!(read.contains(&session) && read.contains(&job), "{read:?}");
    assert!(store.main_fix_of_job(&job_id("aaaa-1111")).expect("reads").is_none(), "a Session is no Job's take");

    let ended = at("2026-10-06T11:00:00.000Z");
    store.end_main_fixes("/repos/armada", &ended, None).expect("ended");
    let read = store.main_fixes_of("/repos/armada", &session.red_at).expect("reads");
    assert!(read.iter().all(|one| one.ended_at.as_ref() == Some(&ended)));
}
