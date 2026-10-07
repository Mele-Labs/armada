use crate::{
    decode, encode, HubJob, HubPullCi, HubPullRequest, Instant, JobId, MainCiState, MainFailedJob,
    MainMerge, MainStanding, MergeLine, MergeLineHub,
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
        }),
        pull_requests: vec![HubPullRequest {
            number: 1816,
            title: "Debounce".to_string(),
            branch: "armada/debounce".to_string(),
            url: "https://forge.invalid/pull/1816".to_string(),
            author: Some("nick".to_string()),
            ci: Some(HubPullCi::WaitingOnMain),
            job: None,
        }],
        fixing: None,
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
        }),
        pull_requests: Vec::new(),
        fixing: None,
    };
    let spelled = encode(&a_line(Some(green))).expect("plain data");
    assert!(!spelled.contains("null"), "{spelled}");
    assert!(spelled.contains(r#""state":"nothing_ran""#), "{spelled}");
    for left_out in ["failed", "merge", "red_since", "pull_requests", "fixing"] {
        assert!(!spelled.contains(left_out), "{left_out} in {spelled}");
    }
    assert!(!encode(&a_line(None)).unwrap().contains("hub"));
}

#[test]
fn a_line_from_a_fleet_before_the_hub_still_reads_and_a_newer_field_does_not_break_it() {
    let before = r#"{"root":"/srv/shop","line":[],"off":[],"landed":[],"sent_back":[]}"#;
    let read: MergeLine = decode("a merge line", before.as_bytes()).expect("reads");
    assert_eq!(read, a_line(None));
    let newer = r#"{"root":"/srv/shop","line":[],"off":[],"landed":[],"sent_back":[],
        "hub":{"fixing":{"id":"01JOB","title":"x"},"since":"later"}}"#;
    let read: MergeLine = decode("a merge line", newer.as_bytes()).expect("reads");
    assert_eq!(read.hub.unwrap().fixing.unwrap().title, "x");
}
