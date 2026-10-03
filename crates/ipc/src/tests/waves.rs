//! What an Epic's wave strip and graph read off the wire (#1692): each pass's
//! line on the parent's detail, and each member's waits-on edges on its Board
//! row. Both are absent where empty, so a 23.13 client reads neither key.

use core_model::{
    DependencyDirection, DependencyEdge, DispatchOrigin, Facts, Job, JobId, ManifestId, ModelName,
    NewJob, StepId, Title, Ulid, Urgency,
};

use crate::tests::{at, detail_of, job, workflow};
use crate::{decode, encode, JobDetail, JobSummary, WaveRound};

fn a_member(edges: Vec<DependencyEdge>) -> Job {
    Job::create_proposed_member(
        NewJob {
            id: JobId::carried(Ulid::carried("01SECOND")),
            title: Title::new("wire the printer in").expect("a title"),
            workflow: workflow(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Incident,
            atomic: true,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: Vec::new(),
            dependencies: edges,
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            proposal_id: None,
            number: core_model::JobNumber::carried(2),
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        DispatchOrigin {
            job_id: JobId::carried(Ulid::carried("01EPIC")),
            step_id: Some(StepId::new("plan")),
            pass: Some(2),
        },
        at("2026-10-03T09:00:00.000Z"),
    )
}

/// **The row carries what it waits on, and only that direction.** A `blocks`
/// edge is the same fact read from the other Job, whose own row carries it.
#[test]
fn a_member_s_row_carries_what_it_waits_on_and_round_trips() {
    let member = a_member(vec![
        DependencyEdge {
            direction: DependencyDirection::DependsOn,
            peer: JobId::carried(Ulid::carried("01FIRST")),
        },
        DependencyEdge {
            direction: DependencyDirection::Blocks,
            peer: JobId::carried(Ulid::carried("01THIRD")),
        },
    ]);
    let row = JobSummary::from(&member);
    assert_eq!(
        row.waits_on
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        ["01FIRST"]
    );
    let json = encode(&row).expect("plain data");
    assert!(json.contains(r#""waits_on":["01FIRST"]"#), "{json}");
    assert_eq!(
        decode::<JobSummary>("a Board row", json.as_bytes()).expect("it round-trips"),
        row
    );
}

/// **Absent, never `[]`**: a row that waits on nothing carries no key.
#[test]
fn a_row_that_waits_on_nothing_carries_no_key() {
    let json = encode(&JobSummary::from(&a_member(Vec::new()))).expect("plain data");
    assert!(!json.contains("waits_on"), "{json}");
}

/// **Each pass's line crosses, oldest first**, and a Job with none carries no
/// key.
#[test]
fn a_job_s_wave_rounds_round_trip_and_are_absent_where_none() {
    let mut detail = detail_of(&job(), &[]);
    let bare = encode(&detail).expect("plain data");
    assert!(!bare.contains("wave_rounds"), "{bare}");

    detail.wave_rounds = vec![
        WaveRound {
            pass: 1,
            approach: "Split the parser off first".to_string(),
        },
        WaveRound {
            pass: 2,
            approach: "Take the two the first wave made ready".to_string(),
        },
    ];
    let json = encode(&detail).expect("plain data");
    assert!(
        json.contains(r#""wave_rounds":[{"pass":1,"approach":"Split the parser off first"}"#),
        "{json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}
