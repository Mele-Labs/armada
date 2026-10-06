//! A step tuned at the approval press: frozen with the Job, and read where the
//! step runs. Since 23.20.

use std::path::Path;
use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::started;
use crate::tests::daemon::{a_proposal_for, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The fixture Fleet offers it, `tests::daemon`'s.
const OFFERED: &str = "another-model";

const MANIFEST: &str =
    "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  build:\n    run: /usr/bin/true\n  \
                        test:\n    run: /usr/bin/true\n";

/// `implement` runs two Manifest Checks and a Judge; `handoff` asks nobody.
const WORKFLOW: &str = "version: 1\nworkflow_id: fixture-tuned\nname: fixture\n\
     steps:\n  - id: implement\n    label: \"Implement\"\n    evidence: {submitted: {type: diff}}\n    \
     mechanical_checks:\n      - type: diff_nonempty\n      - type: manifest_check\n        check: build\n      \
     - type: manifest_check\n        check: test\n    judge_checks:\n      - criteria:\n          - \
     criterion_id: c1\n            question: Is it bounded?\n            on_refusal: refuse\n    \
     delivers: false\n    advance_gate: auto_if_judge_passes\n  - id: handoff\n    label: \"Hand off\"\n    \
     delivers: true\n    advance_gate: human_always\n";

fn a_fleet(home: &TempDir) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST)
        .unwrap_or_else(|why| panic!("the fixture manifest did not parse: {why}"));
    let def = config::WorkflowDef::parse(
        Path::new("fixture-tuned.yml"),
        WORKFLOW,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|why| panic!("the fixture workflow did not parse: {why}"));
    let workflow = config::ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|why| panic!("the fixture workflow did not resolve: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(workflow);
    fittings.starting().manifest = manifest;
    Arc::new(Fleet::assembled(fittings))
}

fn body(json: &str) -> ipc::ApproveDispatch {
    ipc::decode("an approval", json.as_bytes()).expect("decodes")
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

const TUNED: &str = r#"{"tuning": [{
  "step_id": "implement",
  "model": "another-model",
  "harness": "a-harness",
  "effort": "high",
  "context": "  Keep the reader's public shape.  ",
  "judges": 3,
  "checks_off": ["test"]
}]}"#;

/// **What a person tuned is what runs**: the step's Drone is spawned on the
/// model and effort they set and handed their words, its gate runs without the
/// Check they turned off, and three Judges answer.
#[tokio::test]
async fn a_tuned_step_runs_as_it_was_tuned() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-tuned"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(TUNED)))
        .await
        .expect("approved as tuned");

    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    let implement = &detail.steps[0];
    let checks: Vec<String> = implement
        .checks
        .iter()
        .flatten()
        .map(|check| check.name.clone().unwrap_or_else(|| check.kind.clone()))
        .collect();
    assert_eq!(checks, ["diff_nonempty", "build"]);
    let panels: Vec<Option<u32>> = implement
        .judge_checks
        .iter()
        .flatten()
        .map(|judge| judge.panel_size)
        .collect();
    assert_eq!(panels, [Some(3)]);

    started(&fleet, job.id()).await.expect("it starts");
    let configured = fleet.harness().configured();
    let spawned = configured.last().expect("a Drone was spawned");
    assert_eq!(spawned.model().as_str(), OFFERED);
    assert_eq!(spawned.model().effort(), Some(adapter_traits::Effort::High));
    assert!(
        spawned
            .prompt()
            .as_str()
            .contains("\"Keep the reader's public shape.\""),
        "{}",
        spawned.prompt().as_str()
    );

    let reloaded = fleet.load(job.id()).await.expect("the Job");
    let step = reloaded
        .workflow()
        .step(&core_model::StepId::new("implement"))
        .expect("the step");
    assert_eq!(step.effort(), Some(core_model::Effort::High));
    assert_eq!(step.context(), Some("Keep the reader's public shape."));
}

/// A step nobody tuned is spawned as it always was: no effort, no words.
#[tokio::test]
async fn an_untuned_step_runs_as_its_workflow_declares() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-tuned"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), None)
        .await
        .expect("approved as it stands");
    started(&fleet, job.id()).await.expect("it starts");
    let configured = fleet.harness().configured();
    let spawned = configured.last().expect("a Drone was spawned");
    assert_eq!(spawned.model().effort(), None);
    assert!(!spawned.prompt().as_str().contains("FOR THIS PART"));
}

/// **Every tuning nothing could honour is refused, and nothing is kept**: a
/// step the workflow lacks, a model the machine does not offer, a panel of
/// nobody or on a step with no Judge, a Check the step does not run, one of
/// Fleet's own looks, and a field Fleet takes no setting for.
#[tokio::test]
async fn a_tuning_nothing_could_honour_is_refused_and_keeps_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-tuned"))
        .await
        .expect("a Job at the approval gate");
    let refused = [
        (
            r#"{"step_id": "deploy", "effort": "low"}"#,
            "no step `deploy`",
        ),
        (
            r#"{"step_id": "implement", "model": "a-model-nobody-runs"}"#,
            "a-model-nobody-runs",
        ),
        (r#"{"step_id": "implement", "judges": 0}"#, "no Judges"),
        (r#"{"step_id": "handoff", "judges": 2}"#, "no panel to size"),
        (
            r#"{"step_id": "implement", "harness": "another"}"#,
            "runs a-harness",
        ),
        (
            r#"{"step_id": "implement", "checks_off": ["lint"]}"#,
            "no Check `lint`",
        ),
        (
            r#"{"step_id": "implement", "checks_off": ["diff_nonempty"]}"#,
            "not a gate on it",
        ),
    ];
    for (tuning, says) in refused {
        let sent = body(&format!(r#"{{"tuning": [{tuning}]}}"#));
        let why = Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(sent))
            .await
            .expect_err(tuning);
        let (Refusal::Unacceptable(error) | Refusal::IllegalMove(error)) = &why else {
            panic!("{tuning}: {}", code(&why));
        };
        assert!(error.message.contains(says), "{tuning}: {}", error.message);
    }
    let held = fleet.load(job.id()).await.expect("the Job");
    assert_eq!(held.status(), core_model::JobStatus::AwaitingApproval);
    let step = held
        .workflow()
        .step(&core_model::StepId::new("implement"))
        .expect("the step");
    assert_eq!(step.checks().len(), 3);
}
