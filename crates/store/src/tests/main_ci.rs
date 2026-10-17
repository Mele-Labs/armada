//! Main's CI as last read survives a restart.

use core_model::Timestamp;

use crate::tests::{at, open, TempDir};
use crate::{MainCi, MainFailedJob, MainMerge, MainState, Store};

fn red_main() -> MainCi {
    MainCi {
        repository: "/repos/armada".to_string(),
        base: "main".to_string(),
        commit: "a".repeat(40),
        state: MainState::Red,
        read_at: at("2026-10-06T10:00:00.000Z"),
        red_at: Some(at("2026-10-06T10:00:00.000Z")),
        red_commit: Some("9".repeat(40)),
        decided_commit: Some("9".repeat(40)),
        newer_running: 1,
        unfinished: 2,
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
            session: Some("c279e883-d316-4202-8134-76ea8b531835".to_string()),
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
        red_commit: None,
        decided_commit: Some("b".repeat(40)),
        newer_running: 0,
        unfinished: 0,
        failed: Vec::new(),
        merge: None,
        ..red_main()
    };
    store.record_main_ci(&green).expect("kept");
    assert_eq!(store.main_ci("/repos/armada").expect("reads"), Some(green));
}
