//! Main's CI as last read survives a restart, and a pull request's number finds
//! the Job that opened it.

use core_model::Timestamp;

use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::{Delivery, MainCi, MainFailedJob, MainMerge, MainState, Store};

fn red_main() -> MainCi {
    MainCi {
        repository: "/repos/armada".to_string(),
        base: "main".to_string(),
        commit: "a".repeat(40),
        state: MainState::Red,
        read_at: at("2026-10-06T10:00:00.000Z"),
        red_at: Some(at("2026-10-06T10:00:00.000Z")),
        failed: vec![
            MainFailedJob {
                name: "ci".to_string(),
                check: Some("test".to_string()),
                log_url: Some("https://forge.invalid/r/actions/runs/1/job/11".to_string()),
                tests: vec!["tests::one".to_string(), "tests::nested::two".to_string()],
            },
            MainFailedJob {
                name: "test-all".to_string(),
                check: None,
                log_url: None,
                tests: Vec::new(),
            },
        ],
        merge: Some(MainMerge {
            number: 1812,
            url: Some("https://forge.invalid/r/pull/1812".to_string()),
            branch: Some("armada/cache".to_string()),
            job: Some(job_id("01MAINCI0000000000000001")),
        }),
    }
}

#[test]
fn a_red_main_is_read_back_whole_after_a_restart() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.record_main_ci(&red_main()).expect("kept");
    drop(store);
    let store = Store::open(&dir.db()).expect("reopens");
    assert_eq!(
        store.main_ci("/repos/armada").expect("reads"),
        Some(red_main())
    );
    assert_eq!(store.main_ci("/repos/other").expect("reads"), None);
}

#[test]
fn a_newer_reading_replaces_the_older_one_and_its_failed_jobs() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.record_main_ci(&red_main()).expect("kept");
    let green = MainCi {
        commit: "b".repeat(40),
        state: MainState::Green,
        read_at: Timestamp::from_rfc3339("2026-10-06T11:00:00.000Z"),
        red_at: None,
        failed: Vec::new(),
        merge: None,
        ..red_main()
    };
    store.record_main_ci(&green).expect("kept");
    assert_eq!(store.main_ci("/repos/armada").expect("reads"), Some(green));
}

#[test]
fn a_pull_requests_number_finds_the_jobs_that_opened_it_and_not_a_longer_number() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, url) in [
        (
            "01MAINCI0000000000000001",
            "https://forge.invalid/r/pull/1812",
        ),
        (
            "01MAINCI0000000000000002",
            "https://forge.invalid/r/pull/181",
        ),
        (
            "01MAINCI0000000000000003",
            "https://forge.invalid/r/pull/21812",
        ),
    ] {
        store
            .insert_job(&top_level(id), &created_at())
            .expect("stored");
        store
            .record_delivery(
                &job_id(id),
                &Delivery {
                    commit: None,
                    pushed: None,
                    pull_request: Some(url.to_string()),
                    landed: None,
                    unpushed: None,
                },
            )
            .expect("recorded");
    }
    assert_eq!(
        store.jobs_with_pull_request_number(1812).expect("reads"),
        [job_id("01MAINCI0000000000000001")]
    );
    assert!(store
        .jobs_with_pull_request_number(9)
        .expect("reads")
        .is_empty());
}
