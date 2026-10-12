//! What a Job's Now reads off the wire: a Drone's activity, the Job's issues
//! and what it waits on, the mode on an answer, and the body of a Check act.
//!
//! Every field here is left out when empty, so a Fleet or a Bridge built
//! before it carries on reading the same bytes.

use crate::{
    decode, encode, AnswerMode, ChosenAnswer, CheckNamed, DroneActivity, DroneId, DroneState,
    Instant, JobDrone, JobId, JobIssue, JobWaiting, JudgeAnswer, JudgeAnswered, QuestionId, StepId,
};

fn drone(activity: Option<DroneActivity>) -> JobDrone {
    JobDrone {
        drone_id: DroneId::carried("drone-1"),
        step_id: StepId::carried("implement"),
        task: None,
        model: None,
        state: DroneState::Running,
        since: Instant::carried("2026-10-09T09:00:00.000Z"),
        ended_at: None,
        at_rest_since: None,
        turns: None,
        cost_micros: None,
        activity,
    }
}

/// The line and the tail cross as written, the tail oldest first.
#[test]
fn a_drone_carries_its_line_and_the_few_before_it() {
    let row = drone(Some(DroneActivity {
        line: "Edit src/lib.rs".to_string(),
        tail: vec![
            "Read src/lib.rs".to_string(),
            "Grep fn main".to_string(),
            "Edit src/lib.rs".to_string(),
        ],
    }));
    let json = encode(&row).expect("plain data");
    assert!(
        json.contains(r#""activity":{"line":"Edit src/lib.rs","tail":["Read src/lib.rs","Grep fn main","Edit src/lib.rs"]}"#),
        "{json}"
    );
    assert_eq!(decode::<JobDrone>("a Drone", json.as_bytes()).expect("reads"), row);
}

/// A Drone that has said nothing sends no activity key, and a body from a Fleet
/// before it reads as none.
#[test]
fn a_drone_that_has_said_nothing_carries_no_activity_key() {
    let json = encode(&drone(None)).expect("plain data");
    assert!(!json.contains("activity") && !json.contains("null"), "{json}");
    let only = DroneActivity {
        line: "Read src/lib.rs".to_string(),
        tail: Vec::new(),
    };
    let json = encode(&only).expect("plain data");
    assert_eq!(json, r#"{"line":"Read src/lib.rs"}"#);
    assert_eq!(
        decode::<DroneActivity>("an activity", json.as_bytes()).expect("reads"),
        only
    );
}

/// A Job with nothing wrong and nothing waited on sends neither key, and one
/// that has both sends each kind under the tag Bridge matches on.
#[test]
fn issues_and_waiting_are_left_out_when_empty_and_tagged_when_not() {
    let mut detail = super::detail_of(&super::job(), &[]);
    let json = encode(&detail).expect("plain data");
    assert!(!json.contains("\"issues\"") && !json.contains("\"waiting\""), "{json}");

    detail.issues = vec![
        JobIssue::Breakage {
            check: "test".to_string(),
            test: "ipc::round_trips".to_string(),
        },
        JobIssue::WriteScopeOverlap {
            job_id: JobId::carried("01OTHER"),
        },
        JobIssue::FailedVerdict {
            step_id: StepId::carried("implement"),
            attempt: 2,
            trigger: Some("check_failed".to_string()),
        },
        JobIssue::StuckDrone {
            drone_id: DroneId::carried("drone-1"),
            step_id: StepId::carried("implement"),
            quiet_since: Instant::carried("2026-10-09T09:00:00.000Z"),
        },
    ];
    detail.waiting = vec![
        JobWaiting::Resource {
            name: "capacity".to_string(),
        },
        JobWaiting::Job {
            job_id: JobId::carried("01OTHER"),
            title: None,
        },
        JobWaiting::Transition {
            step_id: StepId::carried("review"),
        },
    ];
    let json = encode(&detail).expect("plain data");
    for tagged in [
        r#"{"kind":"breakage","check":"test","test":"ipc::round_trips"}"#,
        r#"{"kind":"write_scope_overlap","job_id":"01OTHER"}"#,
        r#"{"kind":"failed_verdict","step_id":"implement","attempt":2,"trigger":"check_failed"}"#,
        r#"{"kind":"stuck_drone","drone_id":"drone-1","step_id":"implement","quiet_since":"2026-10-09T09:00:00.000Z"}"#,
        r#"{"kind":"resource","name":"capacity"}"#,
        r#"{"kind":"job","job_id":"01OTHER"}"#,
        r#"{"kind":"transition","step_id":"review"}"#,
    ] {
        assert!(json.contains(tagged), "{tagged} is not in {json}");
    }
    assert_eq!(
        decode::<crate::JobDetail>("a detail", json.as_bytes()).expect("reads"),
        detail
    );
}

/// A kind the wire does not have is refused rather than read as another.
#[test]
fn an_issue_of_a_kind_nobody_declared_is_refused() {
    assert!(decode::<JobIssue>("an issue", br#"{"kind":"gremlins"}"#).is_err());
    assert!(decode::<JobWaiting>("a wait", br#"{"kind":"weather"}"#).is_err());
}

/// A mode alone is an answer, so the label may be absent; a body from before
/// either field reads as the label it always carried.
#[test]
fn an_answer_to_a_question_may_be_a_mode_alone() {
    let quick = ChosenAnswer {
        question_id: QuestionId::carried("01Q"),
        chose: None,
        mode: Some(AnswerMode::Quick),
        decisions: Vec::new(),
    };
    let json = encode(&quick).expect("plain data");
    assert_eq!(json, r#"{"question_id":"01Q","mode":"quick"}"#);
    assert_eq!(decode::<ChosenAnswer>("an answer", json.as_bytes()).expect("reads"), quick);

    let older = decode::<ChosenAnswer>("an answer", br#"{"question_id":"01Q","chose":"Fold it in"}"#)
        .expect("an older body reads");
    assert_eq!(older.chose.as_deref(), Some("Fold it in"));
    assert_eq!(older.mode, None);

    let both = decode::<ChosenAnswer>(
        "an answer",
        br#"{"question_id":"01Q","chose":"Fold it in","mode":"best"}"#,
    )
    .expect("a label and a mode read");
    assert_eq!(both.mode, Some(AnswerMode::Best));
    assert!(decode::<ChosenAnswer>("an answer", br#"{"question_id":"01Q","mode":"slow"}"#).is_err());
}

/// The same two names ride on the judge's and the command's answers.
#[test]
fn the_judge_and_command_answers_carry_the_mode_too() {
    let judged = JudgeAnswered {
        answer: JudgeAnswer::Agree,
        asked_at: None,
        note: None,
        mode: Some(AnswerMode::Best),
    };
    let json = encode(&judged).expect("plain data");
    assert_eq!(json, r#"{"answer":"agree","mode":"best"}"#);
    assert_eq!(decode::<JudgeAnswered>("an answer", json.as_bytes()).expect("reads"), judged);

    let command = decode::<crate::AnswerCommand>(
        "an answer",
        br#"{"call":"toolu_01","answer":"reject","mode":"quick"}"#,
    )
    .expect("reads");
    assert_eq!(command.mode, Some(AnswerMode::Quick));
}

/// Retry and skip name the Check, and nothing else.
#[test]
fn a_check_act_names_the_check() {
    let body = CheckNamed {
        check: "lint".to_string(),
    };
    let json = encode(&body).expect("plain data");
    assert_eq!(json, r#"{"check":"lint"}"#);
    assert_eq!(decode::<CheckNamed>("a Check", json.as_bytes()).expect("reads"), body);
    assert!(decode::<CheckNamed>("a Check", br#"{}"#).is_err());
}
