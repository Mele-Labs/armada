//! A dispatched request is a Job from the press, and each ending of the
//! proposer's call moves that Job. #1714, #1716.

use std::sync::Arc;
use std::time::Duration;

use core_model::{
    EscalationTrigger, Facts, Job, JobStatus, NewProposal, ProposalId, Title, TopLevelOrigin,
    TransitionReason,
};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::judging::CallFailed;
use crate::proposing::NotProposed;
use crate::tests::daemon::a_fleet_proposing_through;
use crate::tests::proposing::{a_catalogue, A_REQUEST};
use crate::tests::tmp::TempDir;

const ONE_JOB: &str = "workflow: bug\ntitle: The log reader drops the last line\n\
                       because: a defect with a reproducible symptom";

/// The one Job on the board, escalated with `trigger`, with no workflow
/// frozen and the request still its title.
pub(crate) async fn escalated_as<H, V, W>(fleet: &crate::Fleet<H, V, W>, trigger: EscalationTrigger)
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    let [job] = &loaded.jobs[..] else {
        panic!(
            "the dispatched Job and nothing else, not {}",
            loaded.jobs.len()
        )
    };
    assert_eq!(job.status(), JobStatus::Escalated);
    assert_eq!(
        job.frozen_workflow(),
        None,
        "nothing nobody chose was frozen"
    );
    assert_eq!(job.title().as_str(), A_REQUEST, "the words that were typed");
    assert_eq!(
        fleet.last_reason(job.id()).await.expect("the log reads"),
        Some(TransitionReason::Escalation(trigger))
    );
}

/// The board, polled until a row stands at `proposing`.
async fn a_proposing_row<H, V, W>(fleet: &crate::Fleet<H, V, W>) -> Job
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    for _ in 0..200 {
        let (loaded, _) = fleet.every_job().await.expect("the board reads");
        if let Some(job) = loaded
            .jobs
            .into_iter()
            .find(|job| job.status() == JobStatus::Proposing)
        {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no row stood at `proposing` while the proposer was reading")
}

#[tokio::test]
async fn a_dispatched_request_is_a_row_before_the_proposer_answers_and_the_head_after() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(ONE_JOB).taking(Duration::from_millis(600)),
    ));
    let proposing = Arc::clone(&fleet);
    let call = tokio::spawn(async move { proposing.propose_from(A_REQUEST, None).await });

    let row = a_proposing_row(&fleet).await;
    assert_eq!(row.title().as_str(), A_REQUEST, "the request is the title");
    assert_eq!(row.workflow_id().as_str(), "");
    assert!(row.steps().is_empty());
    assert!(row.proposal_id().is_some());

    let made = call.await.expect("the task").expect("a plan");
    let [head] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    assert_eq!(
        head.id(),
        row.id(),
        "the row proposed is the row at the gate"
    );
    assert_eq!(head.status(), JobStatus::AwaitingApproval);
    assert_eq!(head.title().as_str(), "The log reader drops the last line");
    assert_eq!(head.workflow_id().as_str(), "bug");
    assert!(!head.steps().is_empty());
    // As the store folds it, not only as the call answered it.
    let reloaded = fleet.load(head.id()).await.expect("loads");
    assert_eq!(&reloaded, head);
}

#[tokio::test]
async fn a_request_no_workflow_fits_escalates_its_job() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying("workflow: none\nbecause: this asks for a release"),
    );
    let refused = fleet.propose_from(A_REQUEST, None).await;
    assert!(matches!(refused, Err(Adrift::NoWorkflowFits { .. })));
    escalated_as(&fleet, EscalationTrigger::NoWorkflowFits).await;
}

#[tokio::test]
async fn a_call_that_faults_escalates_its_job_as_proposer_failed() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::that_fails("the network, the quota, the timeout"),
    );
    let refused = fleet.propose_from(A_REQUEST, None).await;
    assert!(matches!(refused, Err(Adrift::NotProposed { .. })));
    escalated_as(&fleet, EscalationTrigger::ProposerFailed).await;
}

/// The owner's answer of 1 Oct 2026: the route keeps its own code, and the Job
/// takes `proposer_failed` with the model named on its log.
#[tokio::test]
async fn a_model_this_machine_does_not_run_escalates_as_proposer_failed() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(
            "workflow: bug\ntitle: The log reader drops the last line\n\
             settings: model=gpt-nonexistent",
        ),
    );
    let refused = fleet.propose_from(A_REQUEST, None).await;
    assert!(
        matches!(refused, Err(Adrift::ModelNotHeld { .. })),
        "the route's own refusal: {refused:?}"
    );
    escalated_as(&fleet, EscalationTrigger::ProposerFailed).await;
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &loaded.jobs[0].handle(),
    ))
    .expect("the Job's own log");
    assert!(
        log.contains("gpt-nonexistent"),
        "the log names the model: {log}"
    );
}

#[tokio::test]
async fn a_person_stopping_the_call_kills_its_job() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(ONE_JOB).taking(Duration::from_secs(30)),
    ));
    let proposing = Arc::clone(&fleet);
    let call = tokio::spawn(async move { proposing.propose_from(A_REQUEST, None).await });
    let row = a_proposing_row(&fleet).await;
    let proposal = row.proposal_id().expect("minted at dispatch");

    assert!(fleet.stop_proposal(&ipc::ProposalId::from(proposal.as_ulid())));

    let stopped = call.await.expect("the task");
    assert!(matches!(
        stopped,
        Err(Adrift::NotProposed {
            cause: NotProposed::Call(CallFailed::Stopped),
            ..
        })
    ));
    assert_eq!(
        fleet.load(row.id()).await.expect("loads").status(),
        JobStatus::Killed
    );
}

/// A call does not survive the Fleet that made it, so a Job left at
/// `proposing` is moved at boot rather than waiting on nothing.
#[tokio::test]
async fn a_job_left_proposing_by_a_dead_fleet_escalates_at_boot() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(ONE_JOB),
    );
    let served = fleet.first();
    let job = {
        let mut store = fleet.store().lock().await;
        let number = store
            .next_job_number(served.manifest().id())
            .expect("a number");
        let job = Job::create_proposing(
            NewProposal {
                id: core_model::JobId::carried(fleet.mint().ulid()),
                title: Title::new(A_REQUEST).expect("a title"),
                owner_manifest_id: served.manifest().id().clone(),
                model: core_model::ModelName::new("sonnet").expect("a model"),
                proposal_id: ProposalId::carried(fleet.mint().ulid()),
                number,
                facts: Facts::new(A_REQUEST),
                attachments: Vec::new(),
            },
            TopLevelOrigin::Manual,
            fleet.now(),
        );
        store.insert_job(&job, &fleet.now()).expect("stored");
        job
    };

    let reconciled = fleet.reconcile().await.expect("the boot reconciles");

    assert_eq!(reconciled.orphaned_proposals, vec![job.id().clone()]);
    escalated_as(&fleet, EscalationTrigger::ProposerFailed).await;
}
