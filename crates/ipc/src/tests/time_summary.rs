//! Where Jobs' time went, on the wire. A figure nobody produced leaves no key, so a client cannot
//! read it as `0`, and a field this build does not know does not break a parse.

use crate::{
    decode, encode, BuildFigures, BuildSource, HourFigures, JobSplit, JobTime, NamedUnits, RecordSplit, SlowJob, UsageHours, UsageTime,
    UsualTime,
};

fn split(you: u64) -> RecordSplit {
    RecordSplit { model: 1_000, tools: 2_000, you, ..Default::default() }
}

#[test]
fn a_jobs_time_round_trips_and_a_job_with_none_kept_sends_only_its_id() {
    let time = JobTime {
        session_id: "s1".into(),
        time: Some(JobSplit {
            workflow: "build".into(),
            began_at: "2026-11-03T09:00:00.000Z".into(),
            ended_at: "2026-11-03T12:00:00.000Z".into(),
            spent: split(600_000),
            usual: Some(UsualTime { workflow: "build".into(), jobs: 3, split: split(1_200_000) }),
        }),
    };
    let json = encode(&time).expect("plain data");
    assert!(json.contains(r#""you":600000"#) && json.contains(r#""jobs":3"#), "{json}");
    assert_eq!(decode::<JobTime>("a job's time", json.as_bytes()).expect("round-trips"), time);

    let none = encode(&JobTime { session_id: "s2".into(), time: None }).expect("plain data");
    assert_eq!(none, r#"{"session_id":"s2"}"#, "an unkept split is an absent key and not a null");

    let loose = JobSplit { workflow: String::new(), usual: None, ..time.time.expect("kept") };
    assert!(!encode(&loose).expect("plain data").contains("usual"), "a Job with no workflow has no usual");
}

#[test]
fn a_windows_time_round_trips_and_an_untitled_job_has_no_title_key() {
    let time = UsageTime {
        since: "2026-10-27T00:00:00.000Z".into(),
        until: "2026-11-03T00:00:00.000Z".into(),
        jobs: 2,
        total: split(5),
        slowest: vec![
            SlowJob {
                session_id: "s1".into(),
                title: Some("Add the audit table".into()),
                workflow: "build".into(),
                began_at: "2026-11-02T09:00:00.000Z".into(),
                ended_at: "2026-11-02T12:00:00.000Z".into(),
                split: split(3),
            },
            SlowJob {
                session_id: "s2".into(),
                title: None,
                workflow: String::new(),
                began_at: "2026-11-02T09:00:00.000Z".into(),
                ended_at: "2026-11-02T10:00:00.000Z".into(),
                split: split(2),
            },
        ],
        usual: vec![UsualTime { workflow: "build".into(), jobs: 2, split: split(4) }],
    };
    let json = encode(&time).expect("plain data");
    assert_eq!(json.matches("\"title\"").count(), 1, "only the titled Job carries one: {json}");
    assert!(!json.contains("null"), "{json}");
    assert_eq!(decode::<UsageTime>("a window's time", json.as_bytes()).expect("round-trips"), time);

    let newer = json.replacen('{', r#"{"a_field_from_a_newer_fleet":1,"#, 1);
    assert_eq!(decode::<UsageTime>("a window's time", newer.as_bytes()).expect("an unknown field is ignored"), time);
}

#[test]
fn an_hour_with_no_figure_sends_no_key_for_it_and_never_a_zero() {
    let quiet = HourFigures { hour: 2, at: "2026-11-03T12:00:00.000Z".into(), burn: vec![], model: None, tools: None, checks: None, pull_request: None };
    assert_eq!(encode(&quiet).expect("plain data"), r#"{"hour":2,"at":"2026-11-03T12:00:00.000Z"}"#);

    let busy = HourFigures {
        burn: vec![NamedUnits { name: "model-a".into(), units: 1.5 }],
        model: Some(30.0),
        checks: Some(0.0),
        ..quiet
    };
    let json = encode(&busy).expect("plain data");
    assert!(json.contains(r#""model":30.0"#) && json.contains(r#""checks":0.0"#), "a median that is really nought is sent: {json}");
    assert!(!json.contains("tools") && !json.contains("pull_request"), "{json}");
    assert_eq!(decode::<HourFigures>("an hour", json.as_bytes()).expect("round-trips"), busy);
}

#[test]
fn a_build_with_no_turn_sends_no_burn_and_no_median_and_a_main_build_no_branches() {
    let bare = BuildFigures {
        commit: "ddddddd".into(),
        started: "2026-11-03T12:50:00.000Z".into(),
        until: None,
        source: None,
        main_commit: None,
        branches: vec![],
        turns: 0,
        burn: None,
        model: None,
        tools: None,
        checks: None,
        pull_request: None,
        jobs: vec![],
        split: RecordSplit::default(),
    };
    let json = encode(&bare).expect("plain data");
    // `split` is last, and has keys of the same names as the medians.
    let figures = &json[..json.find("\"split\"").expect("a split")];
    for absent in ["until", "source", "main_commit", "burn", "model", "tools", "checks", "pull_request", "null", "NaN"] {
        assert!(!figures.contains(absent), "{absent} in {json}");
    }
    assert!(json.contains(r#""branches":[]"#) && json.contains(r#""jobs":[]"#) && json.contains(r#""turns":0"#), "{json}");

    let preview = BuildFigures {
        source: Some(BuildSource::Preview),
        main_commit: Some("ccccccc".into()),
        branches: vec!["armada/x".into()],
        turns: 40,
        burn: Some(41_000.0),
        model: Some(5.2),
        ..bare
    };
    let json = encode(&UsageHours {
        since: "2026-11-01T13:00:00.000Z".into(),
        until: "2026-11-03T13:00:00.000Z".into(),
        hours: vec![],
        builds: vec![preview.clone()],
    })
    .expect("plain data");
    assert!(json.contains(r#""source":"preview""#), "the spelling is BuildSource's: {json}");
    let read: UsageHours = decode("hours", json.as_bytes()).expect("round-trips");
    assert_eq!(read.builds, [preview]);
    assert!(read.hours.is_empty());
}
