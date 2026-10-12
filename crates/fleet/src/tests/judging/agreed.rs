//! A refusal a panel of different models agreed on goes back to the Drone, and
//! every other refusal still reaches a person. #206.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Worktree};
use core_model::{Actor, CriterionId, Timestamp, AGREED_RETRIES};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::{apply, rule_on, Ruling};
use crate::judging::{JudgeBudget, Judging, Marking};
use crate::policy::Policies;
use crate::tests::gate::{budget, diff_evidence, running_job, worktree};
use crate::tests::judging::THE_QUESTION;
use crate::tests::keeping::keeping_nowhere;

const FIRST: &str = "the-cheap-model";
const SECOND: &str = "the-other-model";

const REDO: &str = "verdict: not_met\nexpected: the loop stops at n\n\
                    produced: the loop stops at n - 1\nconsequence: the last row is dropped\n\
                    risky: no";
const RISKY: &str = "verdict: not_met\nexpected: the loop stops at n\n\
                     produced: the loop stops at n - 1\nconsequence: the last row is dropped\n\
                     risky: it drops the last row of every report a person reads";
const MET: &str = "verdict: met";

fn judged_by(client: Arc<FakeJudge>) -> Judging {
    Judging {
        client,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-default-model").expect("a model name"),
        second_opinion_model: Model::named("the-second-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
        standing: verification::Standing::unstated(),
        reading: None,
        wording: verification::Wording::shipped(),
    }
}

/// The ruling on one step whose one criterion is put to a panel of two
/// models, answered in turn, having already gone back `sent_back` times.
async fn ruled(judge: Arc<FakeJudge>, sent_back: u32, worktree: &Worktree) -> Ruling {
    let workflow = testkit::panelled(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[("c1", THE_QUESTION)],
            scope: None,
            gaming: None,
        }],
        &[FIRST, SECOND],
    );
    let at = AtStep::first(workflow.frozen(), worktree)
        .expect("a first step")
        .sent_back(sent_back);
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        &[],
        crate::gate::Began::At(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &judged_by(judge),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await
}

#[tokio::test]
async fn an_agreed_refusal_goes_back_to_the_drone_with_the_reasons() {
    let worktree = worktree();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[REDO, REDO]));
    let ruling = ruled(judge.clone(), 0, &worktree).await;

    let Ruling::JudgesAgreed {
        refusals, judged, ..
    } = &ruling
    else {
        panic!("expected the work sent back, got {ruling:?}");
    };
    assert_eq!(refusals.criteria(), vec![&CriterionId::new("c1"); 2]);
    assert_eq!(judged.len(), 2, "both members' answers are recorded");
    // Each member on its own model: the panel is a second opinion.
    assert_eq!(judge.models(), vec![FIRST.to_string(), SECOND.to_string()]);
    let told = ruling.tell().expect("the Drone is told").text().to_string();
    assert!(told.contains("the loop stops at n - 1"), "{told}");
    // Signed by the Judge, which is how the store counts these apart.
    assert_eq!(ruling.signed_by(), Actor::Judge);
    assert!(ruling.hands_back().is_some());
    assert!(!ruling.ends_the_drone());
    // The Job does not move: the step goes round again under it.
    let at = Timestamp::from_rfc3339("2026-10-11T22:00:00.000Z");
    assert!(apply(&running_job(), &ruling, at).is_none());
}

#[tokio::test]
async fn the_refusal_after_the_retries_are_spent_goes_to_a_person() {
    let worktree = worktree();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[REDO, REDO]));
    let ruling = ruled(judge, AGREED_RETRIES, &worktree).await;
    assert!(
        matches!(ruling, Ruling::Refused { .. }),
        "expected a person, got {ruling:?}"
    );
}

#[tokio::test]
async fn a_split_panel_goes_to_a_person() {
    let worktree = worktree();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[REDO, MET]));
    let ruling = ruled(judge, 0, &worktree).await;
    assert!(
        matches!(ruling, Ruling::Refused { .. }),
        "expected a person, got {ruling:?}"
    );
}

#[tokio::test]
async fn one_member_calling_the_work_risky_brings_a_person_in() {
    let worktree = worktree();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[REDO, RISKY]));
    let ruling = ruled(judge, 0, &worktree).await;
    let Ruling::Refused { judged, .. } = &ruling else {
        panic!("expected a person, got {ruling:?}");
    };
    assert_eq!(
        judged[1].risky.as_deref(),
        Some("it drops the last row of every report a person reads")
    );
}
