//! A proposal sent back to the proposer with a note, from the approval gate.
//! Since 23.25.

use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use core_model::JobStatus;
use testkit::{FakeJudge, FakeWorkProduct};

use crate::tests::daemon::{a_fleet_proposing_through, manifest};
use crate::tests::planning::A_PLAN;
use crate::tests::tmp::TempDir;

const BUG: &str = "workflow: bug\ntitle: The reader drops the last line\nbecause: a defect";
const FEATURE: &str = "workflow: feature\ntitle: Bound the reader and say so\nbecause: the note";

/// Two workflows that share a step id, `implement`, and one that does not.
fn a_catalogue() -> Vec<config::ResolvedWorkflow> {
    let make = |id: &str, steps: &[&str]| {
        let body: String = steps
            .iter()
            .enumerate()
            .map(|(at, step)| {
                format!(
                    "  - id: {step}\n    label: \"{step}\"\n    evidence: {{submitted: {{type: diff}}}}\n    \
                     delivers: {}\n    advance_gate: auto\n",
                    at == steps.len() - 1
                )
            })
            .collect();
        let def = config::WorkflowDef::parse(
            std::path::Path::new("fixture.yml"),
            &format!(
                "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n{body}"
            ),
            &config::Roster::offering_nothing(),
        )
        .expect("a workflow");
        config::ResolvedWorkflow::resolve(&def, &manifest()).expect("it resolves")
    };
    vec![
        make("bug", &["implement", "gone"]),
        make("feature", &["implement", "summarise"]),
    ]
}

fn a_fleet(home: &TempDir, replies: &[&str]) -> Arc<crate::tests::reviewing::Fixture> {
    Arc::new(a_fleet_proposing_through(
        home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        a_catalogue(),
        FakeJudge::answering_in_turn(replies),
    ))
}

fn note(json: &str) -> ipc::ToProposer {
    ipc::decode("a note", json.as_bytes()).expect("decodes")
}

fn log_of(home: &TempDir, job: &core_model::Job) -> String {
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &job.handle(),
    ))
    .unwrap_or_default()
}

const TUNED: &str = r#"{
  "note": "Bound it, and say what the bound is.",
  "tuning": [
    {"step_id": "implement", "effort": "high", "context": "Keep the public shape."},
    {"step_id": "gone", "effort": "low"}
  ],
  "landing": {"pr_mode": "draft"}
}"#;

/// **The proposer rewrites the proposal whole, and what the person set carries
/// over** where the new workflow still has the step: the words and the workflow
/// are the proposer's, the tuning is the person's for `implement` alone, and the
/// landing stands. What had no step to land on is said, not refused.
#[tokio::test]
async fn a_note_rewrites_the_proposal_and_the_persons_tuning_carries_over_by_step_id() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &[BUG, FEATURE]);
    let made = fleet
        .propose_from("the reader drops a line", None)
        .await
        .expect("a plan");
    let head = made[0].clone();
    assert_eq!(head.workflow_id().as_str(), "bug");

    let plan = Commands::to_proposer(
        Arc::clone(&fleet),
        head.id().into(),
        note(&TUNED.replace(
            r#",
    {"step_id": "gone", "effort": "low"}"#,
            "",
        )),
    )
    .await
    .expect("rewritten");
    let [summary] = &plan.jobs[..] else {
        panic!("one Job, not {}", plan.jobs.len());
    };
    assert_eq!(summary.status.as_wire(), "awaiting_approval");

    let rewritten = fleet.load(head.id()).await.expect("loads");
    assert_eq!(rewritten.id(), head.id(), "the same Job, read again");
    assert_eq!(rewritten.status(), JobStatus::AwaitingApproval);
    assert_eq!(rewritten.workflow_id().as_str(), "feature");
    assert_eq!(rewritten.title().as_str(), "Bound the reader and say so");
    let implement = rewritten
        .workflow()
        .step(&core_model::StepId::new("implement"))
        .expect("the step");
    assert_eq!(implement.effort(), Some(core_model::Effort::High));
    assert_eq!(implement.context(), Some("Keep the public shape."));
    let detail = Queries::get_job(&*fleet, head.id().into())
        .await
        .expect("detail");
    assert_eq!(
        detail
            .landing
            .map(|landing| landing.pr_mode.as_wire().to_string()),
        Some("draft".to_string())
    );
}

/// A tuning for a step the rewrite does not have is dropped and said; the
/// rewrite is not refused for it, and a step the proposal never had is.
#[tokio::test]
async fn a_tuning_for_a_step_the_rewrite_lacks_is_dropped_and_said() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &[BUG, FEATURE]);
    let made = fleet
        .propose_from("the reader drops a line", None)
        .await
        .expect("a plan");

    let nowhere = r#"{"note": "again", "tuning": [{"step_id": "summarise", "effort": "high"}]}"#;
    let refused = Commands::to_proposer(Arc::clone(&fleet), made[0].id().into(), note(nowhere))
        .await
        .expect_err("a step the proposal in front of the person lacks");
    assert!(matches!(refused, Refusal::Unacceptable(_)));
    assert_eq!(
        fleet
            .load(made[0].id())
            .await
            .expect("loads")
            .workflow_id()
            .as_str(),
        "bug",
        "nothing moved"
    );

    Commands::to_proposer(Arc::clone(&fleet), made[0].id().into(), note(TUNED))
        .await
        .expect("rewritten");
    let rewritten = fleet.load(made[0].id()).await.expect("loads");
    assert_eq!(rewritten.workflow_id().as_str(), "feature");
    let log = log_of(&home, &rewritten);
    assert!(log.contains("did not carry over"), "{log}");
    assert!(log.contains("gone"), "{log}");
}

/// **A split comes back as Jobs, each at the gate on its own**, each carrying
/// the tuning its steps match.
#[tokio::test]
async fn a_split_rewrite_brings_every_job_to_the_gate_with_the_tuning() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &[BUG, A_PLAN]);
    let made = fleet
        .propose_from("the reader drops a line", None)
        .await
        .expect("a plan");
    let body = r#"{"note": "split it", "tuning": [{"step_id": "implement", "effort": "low"}]}"#;
    let plan = Commands::to_proposer(Arc::clone(&fleet), made[0].id().into(), note(body))
        .await
        .expect("rewritten");
    assert_eq!(plan.jobs.len(), 2);
    for summary in &plan.jobs {
        assert_eq!(summary.status.as_wire(), "awaiting_approval");
        let job = fleet.load(&summary.id.to_domain()).await.expect("loads");
        let implement = job
            .workflow()
            .step(&core_model::StepId::new("implement"))
            .expect("the step");
        assert_eq!(
            implement.effort(),
            Some(core_model::Effort::Low),
            "{}",
            job.title().as_str()
        );
    }
    let second = fleet
        .load(&plan.jobs[1].id.to_domain())
        .await
        .expect("loads");
    assert_eq!(
        second.dispatched_by().map(|origin| origin.job_id.clone()),
        Some(made[0].id().clone()),
        "the extra names the head"
    );
    // Neither the head nor its extra is sent back again: a rewrite would leave
    // the other at its gate.
    for job in &plan.jobs {
        let again = Commands::to_proposer(
            Arc::clone(&fleet),
            job.id.clone(),
            note(r#"{"note": "once more"}"#),
        )
        .await
        .expect_err("a split");
        assert!(matches!(again, Refusal::Unacceptable(_)), "{again:?}");
    }
}

/// **A proposer that could not read the note leaves the proposal as it was.**
#[tokio::test]
async fn a_note_the_proposer_could_not_use_puts_the_proposal_back() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        &[BUG, "workflow: none\nbecause: this asks for a release"],
    );
    let made = fleet
        .propose_from("the reader drops a line", None)
        .await
        .expect("a plan");
    let body = r#"{"note": "ship it"}"#;
    let refused = Commands::to_proposer(Arc::clone(&fleet), made[0].id().into(), note(body))
        .await
        .expect_err("no workflow fits");
    assert!(matches!(refused, Refusal::Unacceptable(_)), "{refused:?}");
    let held = fleet.load(made[0].id()).await.expect("loads");
    assert_eq!(held.status(), JobStatus::AwaitingApproval);
    assert_eq!(held.workflow_id().as_str(), "bug");
    assert_eq!(held.title().as_str(), "The reader drops the last line");
    assert_eq!(held.steps().len(), made[0].steps().len());
    assert!(log_of(&home, &held).contains("as it was"));
}

/// Refused before the Job moves: a blank note, a Job past its gate.
#[tokio::test]
async fn a_blank_note_and_a_job_past_its_gate_are_refused_and_nothing_moves() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &[BUG, FEATURE]);
    let made = fleet
        .propose_from("the reader drops a line", None)
        .await
        .expect("a plan");
    let blank = Commands::to_proposer(
        Arc::clone(&fleet),
        made[0].id().into(),
        note(r#"{"note": "  "}"#),
    )
    .await
    .expect_err("blank");
    assert!(matches!(blank, Refusal::Unacceptable(_)));
    Commands::approve_dispatch(Arc::clone(&fleet), made[0].id().into(), None)
        .await
        .expect("approved");
    let late = Commands::to_proposer(
        Arc::clone(&fleet),
        made[0].id().into(),
        note(r#"{"note": "wait"}"#),
    )
    .await
    .expect_err("past the gate");
    assert!(matches!(late, Refusal::IllegalMove(_)), "{late:?}");
}
