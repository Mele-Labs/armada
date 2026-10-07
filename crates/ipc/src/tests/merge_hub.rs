use crate::{
    decode, encode, FixMain, FixesMain, FixesMainState, HubJob, HubMerged, HubPullCi,
    HubPullRequest, HubQueue, HubQueueState, Instant, JobId, MainChecking, MainCiState,
    MainFailedJob, MainMerge, MainRun, MainRunState, MainStanding, MergeLine, MergeLineHub,
};

fn a_red_hub() -> MergeLineHub {
    MergeLineHub {
        main: Some(MainStanding {
            state: MainCiState::Red,
            commit: "a".repeat(40),
            read_at: Instant::carried("2026-10-06T10:00:00Z"),
            red_since: Some(Instant::carried("2026-10-06T09:58:00Z")),
            failed: vec![MainFailedJob {
                name: "ci".to_string(),
                check: Some("test".to_string()),
                log_url: Some("https://forge.invalid/job/1".to_string()),
                tests: vec!["tests::one".to_string()],
            }],
            merge: Some(MainMerge {
                number: 1812,
                url: Some("https://forge.invalid/pull/1812".to_string()),
                branch: Some("armada/cache".to_string()),
                job: Some(HubJob {
                    id: JobId::carried("01JOB"),
                    title: "Cache the manifest read".to_string(),
                }),
            }),
            red_commit: Some("a".repeat(40)),
            checking: vec![MainChecking {
                commit: "d".repeat(40),
                pull_request: Some(MainMerge {
                    number: 1852,
                    url: None,
                    branch: Some("fleet/flaky".to_string()),
                    job: None,
                }),
            }],
        }),
        pull_requests: vec![HubPullRequest {
            number: 1816,
            title: "Debounce".to_string(),
            branch: "armada/debounce".to_string(),
            url: "https://forge.invalid/pull/1816".to_string(),
            author: Some("nick".to_string()),
            ci: Some(HubPullCi::WaitingOnMain),
            job: None,
            queue: Some(HubQueue {
                state: HubQueueState::Queued,
                position: Some(2),
            }),
        }],
        merged: vec![HubMerged {
            number: 1815,
            title: "Theme tokens".to_string(),
            branch: "nick/theme-tokens".to_string(),
            url: "https://forge.invalid/pull/1815".to_string(),
            author: Some("nick".to_string()),
            merged_at: Instant::carried("2026-10-06T09:50:00Z"),
            commit: Some("c".repeat(40)),
            job: None,
            main_run: Some(MainRun {
                state: MainRunState::Failed,
                failed: vec!["ci".to_string()],
            }),
        }],
        fixing: Some(HubJob {
            id: JobId::carried("01FIX"),
            title: "Fix test on main".to_string(),
        }),
    }
}

fn a_line(hub: Option<MergeLineHub>) -> MergeLine {
    MergeLine {
        root: "/srv/shop".to_string(),
        line: Vec::new(),
        off: Vec::new(),
        landed: Vec::new(),
        sent_back: Vec::new(),
        hub,
    }
}

#[test]
fn a_hub_survives_the_wire_and_the_strict_sets_spell_snake_case() {
    let line = a_line(Some(a_red_hub()));
    let spelled = encode(&line).expect("plain data");
    assert!(spelled.contains(r#""state":"red""#), "{spelled}");
    assert!(spelled.contains(r#""ci":"waiting_on_main""#), "{spelled}");
    assert!(
        spelled.contains(r#""checking":[{"commit":"dddd"#),
        "{spelled}"
    );
    assert!(
        spelled.contains(r#""main_run":{"state":"failed""#),
        "{spelled}"
    );
    let read: MergeLine = decode("a merge line", spelled.as_bytes()).expect("reads");
    assert_eq!(read, line);
}

#[test]
fn what_is_empty_is_left_out_and_never_sent_as_null() {
    let green = MergeLineHub {
        main: Some(MainStanding {
            state: MainCiState::NothingRan,
            commit: "b".repeat(40),
            read_at: Instant::carried("2026-10-06T10:00:00Z"),
            red_since: None,
            failed: Vec::new(),
            merge: None,
            red_commit: None,
            checking: Vec::new(),
        }),
        pull_requests: Vec::new(),
        merged: Vec::new(),
        fixing: None,
    };
    let spelled = encode(&a_line(Some(green))).expect("plain data");
    assert!(!spelled.contains("null"), "{spelled}");
    assert!(spelled.contains(r#""state":"nothing_ran""#), "{spelled}");
    for left_out in [
        "failed",
        "merge",
        "red_since",
        "pull_requests",
        "fixing",
        "merged",
        "red_commit",
        "checking",
        "main_run",
    ] {
        assert!(!spelled.contains(left_out), "{left_out} in {spelled}");
    }
    assert!(!encode(&a_line(None)).unwrap().contains("hub"));
}

#[test]
fn a_line_from_a_fleet_before_the_hub_still_reads_and_a_newer_field_does_not_break_it() {
    let before = r#"{"root":"/srv/shop","line":[],"off":[],"landed":[],"sent_back":[]}"#;
    let read: MergeLine = decode("a merge line", before.as_bytes()).expect("reads");
    assert_eq!(read, a_line(None));
    let held_before = r#"{"root":"/srv/shop","line":[],"off":[],"landed":[],"sent_back":[],
        "hub":{"main":{"state":"red","commit":"a","read_at":"t"},
        "merged":[{"number":1,"title":"t","branch":"b","url":"u","merged_at":"t"}]}}"#;
    let read: MergeLine = decode("a merge line", held_before.as_bytes()).expect("reads");
    let hub = read.hub.unwrap();
    assert!(hub.main.unwrap().checking.is_empty() && hub.merged[0].main_run.is_none());
    let newer = r#"{"root":"/srv/shop","line":[],"off":[],"landed":[],"sent_back":[],
        "hub":{"fixing":{"id":"01JOB","title":"x"},"since":"later"}}"#;
    let read: MergeLine = decode("a merge line", newer.as_bytes()).expect("reads");
    assert_eq!(read.hub.unwrap().fixing.unwrap().title, "x");
}

#[test]
fn a_jobs_part_in_the_red_and_the_act_that_hands_it_over_survive_the_wire() {
    let mark = FixesMain {
        state: FixesMainState::Fixed,
        check: "test".to_string(),
        test: None,
        merge: Some(1839),
        fixed_in: Some(1841),
    };
    let spelled = encode(&mark).expect("plain data");
    assert!(
        spelled.contains(r#""state":"fixed""#) && !spelled.contains("test\":"),
        "{spelled}"
    );
    assert_eq!(
        decode::<FixesMain>("a mark", spelled.as_bytes()).unwrap(),
        mark
    );

    let new = FixMain {
        root: "/srv/shop".to_string(),
        job: None,
        brief: None,
    };
    assert_eq!(encode(&new).unwrap(), r#"{"root":"/srv/shop"}"#);
    let back = r#"{"root":"/srv/shop","job":"01JOB","brief":"test fails on main."}"#;
    let read: FixMain = decode("a fix", back.as_bytes()).expect("reads");
    assert_eq!(read.job, Some(JobId::carried("01JOB")));
}
