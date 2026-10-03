//! Slice 6 of the backend milestone, in a module of its own because
//! `drone_per_task.rs` is near the size the gate refuses.
//!
//! **What Fleet decides is asserted through the functions it decides with**:
//! which Jobs the Epic holds as its wave, whether a press names that wave, and
//! which members have not landed. The Fleet methods that move the record on
//! those answers need a store, so they are proved in `fleet`'s own tests.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | A Drone's `dispatch_job` creating a proposed member, stamped with its pass | A store; `crates/fleet/src/tests/epic.rs` drives the shipped epic through it |
//! | `approve_wave` moving the parent off its gate and each member to `queued`, and `approve_dispatch` refusing one member alone | A store; `crates/fleet/src/tests/waves.rs` |
//! | The parent held after its plan until every member's pull request merged | A `tokio` turn and the forge's fake; `crates/fleet/src/tests/waves.rs` |
//! | Which forge field says when a pull request merged | The forge is `adapters`' to name; `crates/adapters/src/tests/landing.rs` |

use std::collections::BTreeMap;

use adapter_traits::Landing as Settled;
use core_model::{
    Actor, AdvanceGate, CompleteWhen, DependencyDirection, DependencyEdge, DispatchOrigin, Facts,
    Job, JobId, JobNumber, JobStatus, ManifestId, ModelName, NewJob, StepId, StepSeed, Target,
    Title, TopLevelOrigin, Ulid, Urgency,
};
use fleet::waving::{self, NotTheWave};

use super::at;
use crate::bench::arc::landing_by_the_repository;

/// The epic this repository ships, read at compile time: the file a person's
/// Epic Job freezes.
fn shipped_epic() -> core_model::FrozenWorkflow {
    let path = std::path::Path::new(".armada/workflows/epic.json");
    let text = include_str!("../../../../.armada/workflows/epic.json");
    let roster = config::Roster::of(adapters::HeadlessAgent::models());
    let def = config::WorkflowDef::parse(path, text, &roster)
        .unwrap_or_else(|refused| panic!("the shipped epic did not parse: {refused}"));
    let manifest = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|refused| panic!("the shipped epic did not resolve: {refused}"))
        .frozen()
        .clone()
}

fn id(n: u32) -> JobId {
    JobId::carried(Ulid::carried(format!("01WAVE{n:020}")))
}

/// A Job as the draft makes one, on `workflow`, waiting on `after`.
fn drafted(n: u32, title: &str, workflow: core_model::FrozenWorkflow, after: &[u32]) -> NewJob {
    let steps = workflow
        .steps()
        .iter()
        .enumerate()
        .map(|(ordinal, step)| StepSeed {
            step_id: step.id().clone(),
            ordinal: ordinal as u32,
        })
        .collect();
    NewJob {
        id: id(n),
        title: Title::new(title).expect("a title"),
        workflow,
        owner_manifest_id: ManifestId::carried(Ulid::carried("01FIXTUREMANIFEST")),
        urgency: Urgency::Normal,
        atomic: false,
        model: ModelName::new("a-model").expect("a model name"),
        acceptance_criteria: Vec::new(),
        steps,
        dependencies: after
            .iter()
            .map(|peer| DependencyEdge {
                direction: DependencyDirection::DependsOn,
                peer: id(*peer),
            })
            .collect(),
        gate_manifests: Vec::new(),
        write_targets: None,
        subject: None,
        redispatched_from: None,
        number: JobNumber::carried(n),
        proposal_id: None,
        facts: Facts::new("what the Epic's plan told this piece's agent"),
        scope_revisions: Vec::new(),
        attachments: Vec::new(),
    }
}

/// One piece the Epic's plan proposed on `pass`.
fn proposed(n: u32, title: &str, pass: u32, after: &[u32]) -> Job {
    let by = DispatchOrigin {
        job_id: id(1),
        step_id: Some(StepId::new("plan")),
        pass: Some(pass),
    };
    Job::create_proposed_member(
        drafted(n, title, landing_by_the_repository(), after),
        by,
        at(1),
    )
}

fn named(jobs: &[u32]) -> ipc::ApproveWave {
    let body = format!(
        r#"{{"jobs": [{}]}}"#,
        jobs.iter()
            .map(|n| format!(r#""{}""#, id(*n).as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    ipc::decode("an approve_wave body", body.as_bytes()).expect("Bridge's body decodes")
}

fn ids(jobs: &[u32]) -> Vec<JobId> {
    jobs.iter().map(|n| id(*n)).collect()
}

fn replaced(board: &mut [Job], job: Job) {
    let at = board
        .iter()
        .position(|held| held.id() == job.id())
        .expect("on the board");
    board[at] = job;
}

/// Slice 6: **an Epic's next wave is real Jobs I can read and correct before I
/// approve it, one press starts them all, and the parent finishes when every
/// member's pull request has merged.**
#[test]
fn an_epics_wave_is_jobs_i_correct_then_release_in_one_press_and_it_ends_when_they_merge() {
    // ---------------------------- the step a person answers proposes the wave
    let epic = shipped_epic();
    let plan = StepId::new("plan");
    let proposing: Vec<&str> = epic
        .steps()
        .iter()
        .filter(|step| step.may_dispatch_jobs())
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(proposing, ["plan"], "the Jobs exist while the plan is read");
    assert_eq!(
        epic.step(&plan).map(|step| step.advance_gate()),
        Some(AdvanceGate::HumanAlways),
        "and a person answers the step that made them"
    );

    // ---------------------------------- the wave is Jobs, at the approval gate
    let parent = Job::create_top_level(
        drafted(1, "Carry the error contract", epic, &[]),
        TopLevelOrigin::Manual,
        at(0),
    );
    let earlier = proposed(2, "Handle every refusal at the seam", 1, &[])
        .transition(Target::Queued, Actor::Human, at(1))
        .expect("the first wave was released")
        .job;
    let mut board = vec![
        parent.clone(),
        earlier,
        proposed(3, "Refuse an unknown code", 2, &[]),
        proposed(4, "Name the fault in the toast", 2, &[3]),
        proposed(5, "Carry the code into the journal", 2, &[3]),
    ];
    let wave = &board[2..];
    assert!(wave
        .iter()
        .all(|job| job.status() == JobStatus::AwaitingApproval));
    assert!(
        wave.iter().all(|job| job.origin().top_level().is_none()),
        "a member is sub-dispatched, so it cannot dispatch: depth two is refused"
    );
    let row = ipc::JobSummary::of(&board[3], None, None, None, false, None, None, None);
    assert_eq!(
        row.dispatched_by.as_ref().map(ipc::JobId::as_str),
        Some(id(1).as_str())
    );
    assert_eq!(
        row.dispatched_pass,
        Some(2),
        "the Board row says which pass made it"
    );
    assert_eq!(
        board[3]
            .dependencies()
            .iter()
            .map(|edge| edge.peer.clone())
            .collect::<Vec<_>>(),
        ids(&[3]),
        "and what it waits on"
    );
    assert_eq!(
        waving::held(parent.id(), &board),
        ids(&[3, 4, 5]),
        "the wave Fleet holds is this pass's, not the one already released"
    );

    // --------------------------- corrected and dropped, before the approval
    let edit: ipc::EditJob = ipc::decode(
        "an edit",
        br#"{"title": "Say the fault's code in the toast"}"#,
    )
    .expect("Edit this Job's body decodes");
    let edited = fleet::approving::edited(&board[3], &edit).expect("a proposal nobody released");
    let edited = board[3]
        .proposal_edited(edited, &at(2))
        .expect("still at its gate");
    replaced(&mut board, edited);
    let dropped = board[4]
        .transition(Target::Killed, Actor::Human, at(2))
        .expect("Hold to drop clears it")
        .job;
    replaced(&mut board, dropped);
    assert_eq!(waving::held(parent.id(), &board), ids(&[3, 4]));

    // ---------------------------------------- one press, all or nothing
    assert_eq!(
        waving::released(parent.id(), &board, &named(&[3])),
        Err(NotTheWave::Differs {
            not_held: Vec::new(),
            not_named: vec![id(4).as_str().to_string()],
        }),
        "a press naming part of the wave releases none of it"
    );
    assert!(
        matches!(
            waving::released(parent.id(), &board, &named(&[3, 4, 5])),
            Err(NotTheWave::Differs { .. })
        ),
        "nor one naming a Job that was dropped"
    );
    let released =
        waving::released(parent.id(), &board, &named(&[4, 3])).expect("the wave Fleet holds");
    assert_eq!(released, ids(&[3, 4]));
    for job in &released {
        let at_gate = board
            .iter()
            .find(|held| held.id() == job)
            .expect("held")
            .clone();
        let queued = at_gate
            .transition(Target::Queued, Actor::Human, at(3))
            .expect("released")
            .job;
        replaced(&mut board, queued);
    }
    assert_eq!(
        board[3].title().as_str(),
        "Say the fault's code in the toast",
        "a member edited before the press is released as edited"
    );
    assert_eq!(
        board[4].status(),
        JobStatus::Killed,
        "a dropped one is not released"
    );
    assert_eq!(
        waving::released(parent.id(), &board, &named(&[])),
        Err(NotTheWave::NoneHeld),
        "once released, there is no wave left to approve"
    );

    // ------------------------------- the parent ends on its members' merges
    let completing = fleet::approving::decided(
        &parent,
        &ipc::decode(
            "an approval",
            br#"{"landing": {"branching": "job", "complete_when": "all_members_landed"}}"#,
        )
        .expect("an approval decodes"),
        None,
    )
    .expect("all_members_landed is a setting Fleet runs");
    assert_eq!(
        completing.landing.complete_when,
        CompleteWhen::AllMembersLanded
    );

    let open = |job: u32| Settled::Open {
        url: format!("https://forge.example/pull/{job}"),
        rendering: adapter_traits::Rendering::AsWritten,
    };
    let merged = |job: u32| Settled::Merged {
        url: format!("https://forge.example/pull/{job}"),
    };
    let mut landed: BTreeMap<JobId, Settled> = BTreeMap::from([(id(2), merged(2))]);
    assert_eq!(
        waving::unlanded(parent.id(), &board, &landed),
        ids(&[3, 4]),
        "the dropped member is never waited for"
    );
    landed.insert(id(3), merged(3));
    landed.insert(id(4), open(4));
    assert_eq!(waving::unlanded(parent.id(), &board, &landed), ids(&[4]));
    landed.insert(id(4), merged(4));
    assert!(
        waving::unlanded(parent.id(), &board, &landed).is_empty(),
        "every member's pull request merged, so the parent may finish"
    );

    // ---------------------------- when it merged, read off the forge, served
    let mut row = ipc::JobSummary::of(&board[3], None, None, None, false, None, None, None);
    row.landed = Some(ipc::Settled::Merged);
    row.merged_at = Some(ipc::Instant::from(&at(9)));
    let back: ipc::JobSummary =
        ipc::decode("a row", ipc::encode(&row).expect("encodes").as_bytes()).expect("decodes");
    assert_eq!(
        back.merged_at.as_ref().map(ipc::Instant::as_str),
        Some(at(9).as_str())
    );
    assert!(
        api::SERVED
            .iter()
            .any(|served| served.operation == "approve_wave"),
        "approve_wave is a route Fleet answers"
    );
}
