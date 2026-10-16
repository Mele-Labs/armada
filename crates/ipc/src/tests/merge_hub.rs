use crate::{
    decode, encode, CatchUpCheckout, CheckoutCaughtUp, CheckoutDiff, CheckoutDiffAsk, ClaimHolder, FixMain, FixesMain, FixesMainState, FixingElsewhere, HeldCheckout, HolderKind, HubJob, HubMergeable, HubMerged,
    HubPullCi, HubPullRequest, HubQueue, HubQueueState, HubReview, HubSession, Instant, JobId, MainChecking, MainCiState,
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
                session: None,
            }),
            red_commit: Some("a".repeat(40)),
            checking: vec![MainChecking {
                commit: "d".repeat(40),
                pull_request: Some(MainMerge {
                    number: 1852,
                    url: None,
                    branch: Some("fleet/flaky".to_string()),
                    job: None,
                    session: Some(HubSession {
                        id: "c279e883-d316-4202-8134-76ea8b531835".to_string(),
                        title: Some("Session Messages Fail to Send".to_string()),
                    }),
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
            opened_at: Some(Instant::carried("2026-10-06T09:10:00Z")),
            updated_at: Some(Instant::carried("2026-10-06T09:40:00Z")),
            draft: true,
            mergeable: Some(HubMergeable::Conflicting),
            review: Some(HubReview::ChangesRequested),
            additions: Some(120),
            deletions: Some(0),
            owner: None,
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
        fixing_session: None,
        fixing_elsewhere: Some(FixingElsewhere {
            machine: "mac-a".to_string(),
            holder: ClaimHolder {
                kind: HolderKind::Job,
                id: "01OTHER".to_string(),
            },
            since: Instant::carried("2026-10-06T09:40:00Z"),
        }),
        checkout: None,
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
    assert!(
        spelled.contains(r#""fixing_elsewhere":{"machine":"mac-a","holder":{"kind":"job","id":"01OTHER"},"since":"2026-10-06T09:40:00Z"}"#),
        "{spelled}"
    );
    for pull_fact in [
        r#""opened_at":"2026-10-06T09:10:00Z""#,
        r#""updated_at":"2026-10-06T09:40:00Z""#,
        r#""draft":true"#,
        r#""mergeable":"conflicting""#,
        r#""review":"changes_requested""#,
        r#""additions":120"#,
        r#""deletions":0"#,
    ] {
        assert!(spelled.contains(pull_fact), "{pull_fact} in {spelled}");
    }
    let read: MergeLine = decode("a merge line", spelled.as_bytes()).expect("reads");
    assert_eq!(read, line);
}

#[test]
fn a_session_holding_the_red_is_spelled_beside_the_job_field_and_survives_the_wire() {
    let line = a_line(Some(MergeLineHub {
        fixing: None,
        fixing_session: Some(HubSession {
            id: "aaaa-1111".to_string(),
            title: Some("Fix main after #1815".to_string()),
        }),
        ..a_red_hub()
    }));
    let spelled = encode(&line).expect("plain data");
    assert!(
        spelled.contains(r#""fixing_session":{"id":"aaaa-1111","title":"Fix main after #1815"}"#),
        "{spelled}"
    );
    assert!(!spelled.contains(r#""fixing":"#), "{spelled}");
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
        fixing_session: None,
        fixing_elsewhere: None,
        checkout: None,
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
        "fixing_session",
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
        take_over: false,
    };
    assert_eq!(encode(&new).unwrap(), r#"{"root":"/srv/shop"}"#);
    let over = FixMain { take_over: true, ..new.clone() };
    assert_eq!(encode(&over).unwrap(), r#"{"root":"/srv/shop","take_over":true}"#);
    let back = r#"{"root":"/srv/shop","job":"01JOB","brief":"test fails on main."}"#;
    let read: FixMain = decode("a fix", back.as_bytes()).expect("reads");
    assert_eq!(read.job, Some(JobId::carried("01JOB")));
}

#[test]
fn a_pull_request_from_a_fleet_before_its_facts_reads_with_none_and_sends_none() {
    let before = r#"{"number":7,"title":"t","branch":"b","url":"u","author":"nick"}"#;
    let read: HubPullRequest = decode("a pull request", before.as_bytes()).expect("reads");
    assert!(read.opened_at.is_none() && read.updated_at.is_none() && !read.draft);
    assert!(read.mergeable.is_none() && read.review.is_none());
    assert!(read.additions.is_none() && read.deletions.is_none());
    assert_eq!(encode(&read).unwrap(), before);
    let none = r#"{"number":7,"title":"t","branch":"b","url":"u","mergeable":"unknown","review":"none"}"#;
    let read: HubPullRequest = decode("a pull request", none.as_bytes()).expect("reads");
    assert_eq!(read.mergeable, Some(HubMergeable::Unknown));
    assert_eq!(read.review, Some(HubReview::None));
}

#[test]
fn a_pull_requests_owner_is_a_kind_an_id_a_machine_and_whether_it_is_this_one() {
    let with = r#"{"number":7,"title":"t","branch":"b","url":"u","owner":{"kind":"session","id":"s-a","machine":"mac-a","this_machine":false}}"#;
    let read: HubPullRequest = decode("a pull request", with.as_bytes()).expect("reads");
    let owner = read.owner.as_ref().expect("an owner");
    assert_eq!((owner.kind, owner.id.as_str(), owner.machine.as_str(), owner.this_machine), (HolderKind::Session, "s-a", "mac-a", false));
    assert_eq!(encode(&read).unwrap(), with);
}

#[test]
fn a_held_checkout_is_spelled_with_what_holds_it_and_survives_the_wire() {
    let line = a_line(Some(MergeLineHub {
        checkout: Some(HeldCheckout {
            base: "main".to_string(),
            on: None,
            uncommitted: vec!["armada.yml".to_string()],
            clashing: Vec::new(),
            behind: 880,
            ahead: 0,
        }),
        ..a_red_hub()
    }));
    let spelled = encode(&line).expect("plain data");
    assert!(
        spelled.contains(r#""checkout":{"base":"main","uncommitted":["armada.yml"],"behind":880,"ahead":0}"#),
        "{spelled}"
    );
    let read: MergeLine = decode("a merge line", spelled.as_bytes()).expect("reads");
    assert_eq!(read, line);
}

#[test]
fn a_catch_up_names_a_repository_and_carries_no_command() {
    let asked = CatchUpCheckout { manifest_id: crate::ManifestId::carried("shop") };
    assert_eq!(encode(&asked).unwrap(), r#"{"manifest_id":"shop"}"#);
    let read: CatchUpCheckout = decode("a catch-up", br#"{"manifest_id":"shop"}"#).expect("reads");
    assert_eq!(read, asked);
    // The type has nowhere to put a command, so one sent anyway is not read.
    let smuggled: CatchUpCheckout = decode("a catch-up", br#"{"manifest_id":"shop","command":"rm -rf /"}"#).expect("reads");
    assert_eq!(smuggled, asked);
}

#[test]
fn what_a_pull_came_to_survives_the_wire_with_its_code_and_words() {
    for answer in [
        CheckoutCaughtUp { code: 0, output: "Fast-forward".to_string() },
        CheckoutCaughtUp { code: 1, output: "Your local changes are stashed".to_string() },
    ] {
        let spelled = encode(&answer).expect("plain data");
        let read: CheckoutCaughtUp = decode("a pull", spelled.as_bytes()).expect("reads");
        assert_eq!(read, answer);
    }
    assert_eq!(
        encode(&CheckoutCaughtUp { code: 0, output: "ok".to_string() }).unwrap(),
        r#"{"code":0,"output":"ok"}"#
    );
}

#[test]
fn a_checkout_diff_is_asked_by_manifest_and_path_and_answers_with_the_text_unchanged() {
    let asked = CheckoutDiffAsk { manifest_id: crate::ManifestId::carried("shop"), path: "src/a b.rs".to_string() };
    assert_eq!(encode(&asked).unwrap(), r#"{"manifest_id":"shop","path":"src/a b.rs"}"#);
    let read: CheckoutDiffAsk = decode("a diff ask", encode(&asked).unwrap().as_bytes()).expect("reads");
    assert_eq!(read, asked);

    let answer = CheckoutDiff { diff: "diff --git a/f b/f\n@@ -1 +1 @@\n-a\n+b \"quoted\"\n".to_string() };
    let back: CheckoutDiff = decode("a diff", encode(&answer).unwrap().as_bytes()).expect("reads");
    assert_eq!(back, answer);
}
