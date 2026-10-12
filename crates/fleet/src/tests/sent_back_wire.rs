//! A step the Judges sent back says so on `get_job`, and how many times of how many; a refusal's
//! risk reason crosses on its judgment. #2105.

use std::sync::Arc;

use core_model::{
    Actor, EscalationTrigger, StepLevelTrigger, StepTarget, Timestamp, AGREED_RETRIES,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]),
    );
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    Fleet::assembled(fittings)
}

/// One hand-back of the running step, signed `by`, and back to running.
async fn handed_back(fleet: &Fixture, job: &core_model::JobId, by: Actor, when: &str) {
    let step = core_model::StepId::new("implement");
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("step-level");
    let at = |seconds: &str| Timestamp::from_rfc3339(format!("2026-10-12T03:00:{seconds}.000Z"));
    let held = fleet.load(job).await.expect("the Job");
    let back = held
        .transition_step(&step, StepTarget::Retrying(why), by, at(when))
        .expect("sent back");
    let mut store = fleet.store().lock().await;
    store.record_step_transition(&back).expect("recorded");
    let again = back
        .job
        .transition_step(&step, StepTarget::Running, Actor::Fleet, at(when))
        .expect("running again");
    store.record_step_transition(&again).expect("recorded");
}

async fn sent_back(fleet: &Fixture, job: &core_model::JobId) -> Option<ipc::JudgesSentBack> {
    let detail = fleet
        .job_detail(ipc::JobId::from(job))
        .await
        .expect("the detail");
    detail.steps[0].sent_back
}

#[tokio::test]
async fn a_step_the_judges_sent_back_says_how_many_times_of_how_many() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("fix the reader"))
        .await
        .expect("a proposal");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id())
        .await
        .expect("approved and started");
    assert_eq!(sent_back(&fleet, job.id()).await, None, "nothing went back");

    handed_back(&fleet, job.id(), Actor::Check, "01").await;
    assert_eq!(
        sent_back(&fleet, job.id()).await,
        None,
        "a failed Check's hand-back is not the Judges'"
    );

    handed_back(&fleet, job.id(), Actor::Judge, "02").await;
    assert_eq!(
        sent_back(&fleet, job.id()).await,
        Some(ipc::JudgesSentBack {
            times: 1,
            of: AGREED_RETRIES
        }),
        "Sent back by the Judges, 1 of 2"
    );
}
