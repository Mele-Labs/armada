//! A Drone on the Plan step asking several decisions in one ask, and a person
//! answering all of them at once. The single question is `super`'s.
//!
//! Not proved here: that the Drone *acts* on the answers, for `super`'s reason.
use std::sync::Arc;

use api::Queries;
use core_model::{JobStatus, StepId};
use ipc::mcp::{AskQuestion, AskedDecision, AskedOption};
use ipc::ChosenDecision;
use testkit::FakeHarness;

use super::{a_drone_that_works, queued_reason, Held, QUIET_AFTER};
use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::questioning::{NotAnswered, NotAsked};
use crate::silence::Liveness;
use crate::slots::Concurrency;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{
    a_proposal_for, fitted_with, one, plan_and_implement, worktree_directory,
};
use crate::tests::restarting::{on_it, the_handle, until_spoken};
use crate::tests::tmp::TempDir;
use testkit::{FakeJudge, FakeWorkProduct};

use super::Fixture;

const PLAN: &str = "plan";

fn offered(first: &str, second: &str) -> Vec<AskedOption> {
    vec![
        AskedOption {
            label: first.to_string(),
            consequence: format!("{first} is what gets built"),
        },
        AskedOption {
            label: second.to_string(),
            consequence: format!("{second} is what gets built"),
        },
    ]
}

/// Two decisions the plan turns on.
fn an_interview() -> AskQuestion {
    AskQuestion {
        question: String::new(),
        options: Vec::new(),
        decisions: vec![
            AskedDecision {
                question: "Where does the schema change live?".to_string(),
                options: offered("Its own Job", "Fold it in"),
            },
            AskedDecision {
                question: "Which side lands first?".to_string(),
                options: offered("Reader first", "Writer first"),
            },
        ],
    }
}

fn picked(decision: &str, chose: &str) -> ChosenDecision {
    ChosenDecision {
        decision_id: decision.to_string(),
        chose: chose.to_string(),
    }
}

fn both() -> Vec<ChosenDecision> {
    vec![picked("d1", "Fold it in"), picked("d2", "Reader first")]
}

/// A Fleet whose one workflow opens on the Plan step.
fn planning(home: &TempDir, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.starting().workflows = one(plan_and_implement());
    fittings.clock = Arc::new(Held::started());
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.norms = StepNorms::of(
        60,
        std::time::Duration::from_secs(1_500),
        std::time::Duration::from_secs(120),
    );
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a question"));
    fittings.concurrency = Concurrency::of(2);
    Fleet::assembled(fittings)
}

/// Approve a Job on the Plan workflow and let its Drone take a place.
async fn planning_job(fleet: &Fixture, home: &TempDir, title: &str) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal_for(title, "fixture-plan"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    job.id().clone()
}

/// **The requirement.** A Drone on the Plan step asks two decisions in one ask:
/// both are on the Job, the Drone is gone, and the wire serves both.
#[tokio::test]
async fn a_plan_drone_asks_two_decisions_in_one_ask() {
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    assert_eq!(
        fleet
            .load(&job)
            .await
            .unwrap()
            .current_step_id()
            .map(StepId::as_str),
        Some(PLAN),
        "the Drone is on the Plan step"
    );

    let asked = fleet
        .ask_question(&job, an_interview())
        .await
        .expect("a working Drone may ask several decisions");

    assert_eq!(
        asked.decisions.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        ["d1", "d2"],
        "Fleet gave each an id, in the order asked"
    );
    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Queued, "the Drone gave its place back");
    assert_eq!(record.question(), Some(&asked), "and both are on the record");
    assert_eq!(
        queued_reason(&fleet, &job).await.as_deref(),
        Some("awaiting_answer")
    );

    let served = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job reads")
        .asking
        .expect("the pending decisions are served");
    assert_eq!(served.step_id.as_str(), PLAN);
    assert_eq!(served.decisions.len(), 2);
    assert_eq!(served.decisions[0].decision_id, "d1");
    assert_eq!(served.decisions[0].question, "Where does the schema change live?");
    assert_eq!(served.decisions[1].options[1].label, "Writer first");
    assert!(
        !served.decisions[1].options[1].consequence.is_empty(),
        "each option says what it commits to"
    );
    assert!(served.question.is_empty() && served.options.is_empty());
}

/// The decisions are on the record, so a Fleet restart in the hours before the
/// answer loses none of them and starts nothing.
#[tokio::test]
async fn the_pending_decisions_survive_a_restart() {
    let home = TempDir::new();
    let first = planning(&home, a_drone_that_works());
    let job = planning_job(&first, &home, "plan the parser change").await;
    let asked = first.ask_question(&job, an_interview()).await.unwrap();
    drop(first);

    let second = planning(&home, FakeHarness::that_echoes_its_first_turn());
    second.reconcile().await.expect("the boot read");
    let record = second.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Queued, "nothing escalated it");
    assert_eq!(record.question(), Some(&asked), "both decisions outlived the Fleet");
    assert_eq!(
        queued_reason(&second, &job).await.as_deref(),
        Some("awaiting_answer")
    );
    let served = second
        .get_job(ipc::JobId::from(&job))
        .await
        .unwrap()
        .asking
        .expect("still served");
    assert_eq!(served.decisions.len(), 2);
}

/// **The point of an interview.** One answer carries every pick; the next Drone
/// opens knowing each decision and what was chosen for it.
#[tokio::test]
async fn answering_both_opens_the_next_brief_with_every_decision() {
    let home = TempDir::new();
    let fleet = planning(&home, FakeHarness::that_echoes_its_first_turn());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    let asked = fleet.ask_question(&job, an_interview()).await.unwrap();

    fleet
        .answer_decisions(&job, asked.id.as_str(), &both())
        .await
        .expect("a pick for each decision");
    assert!(
        !fleet.load(&job).await.unwrap().waiting_on_an_answer(),
        "nothing waits once every decision is answered"
    );
    started(&fleet, &job).await.unwrap();

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Running, "a fresh Drone is on it");
    assert_eq!(record.current_step_id().map(StepId::as_str), Some(PLAN));
    let said = until_spoken(
        &home,
        &the_handle(&fleet, &job).await,
        &on_it(&fleet, &job).await,
    )
    .await;
    for wanted in [
        "WHAT A PERSON DECIDED",
        "Where does the schema change live?",
        "Fold it in",
        "Fold it in is what gets built",
        "Which side lands first?",
        "Reader first",
        "Reader first is what gets built",
    ] {
        assert!(said.contains(wanted), "`{wanted}` missing from the brief: {said}");
    }
    assert!(
        !said.contains("Writer first"),
        "an option nobody picked is not in the brief: {said}"
    );
    assert!(
        fleet.load(&job).await.unwrap().question().is_none(),
        "delivered once, and gone from the record"
    );
}

/// A partial answer is refused by naming the decisions it left open, and the
/// interview still stands.
#[tokio::test]
async fn a_partial_answer_is_refused() {
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    let asked = fleet.ask_question(&job, an_interview()).await.unwrap();

    let refused = fleet
        .answer_decisions(&job, asked.id.as_str(), &[picked("d1", "Fold it in")])
        .await
        .expect_err("d2 was not answered");
    assert!(
        matches!(&refused, NotAnswered::Unanswered { decisions } if decisions == &["d2"]),
        "{refused:?}"
    );
    let record = fleet.load(&job).await.unwrap();
    assert!(record.waiting_on_an_answer(), "the interview still stands");
    assert!(
        record.question().unwrap().decisions.iter().all(|d| d.chose.is_none()),
        "and nothing of the partial answer was kept"
    );
}

/// An unknown decision, an unoffered option and a decision named twice are each
/// refused by name, and nothing is kept.
#[tokio::test]
async fn an_unknown_decision_or_option_is_refused() {
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    let asked = fleet.ask_question(&job, an_interview()).await.unwrap();
    let id = asked.id.as_str();

    let refused = fleet
        .answer_decisions(&job, id, &[picked("d1", "Fold it in"), picked("d9", "Reader first")])
        .await
        .expect_err("no such decision");
    assert!(matches!(&refused, NotAnswered::NoSuchDecision { decision } if decision == "d9"));

    // `Reader first` is d2's option, not d1's: a label matching another
    // decision's options is not an answer to this one.
    let refused = fleet
        .answer_decisions(&job, id, &[picked("d1", "Reader first"), picked("d2", "Reader first")])
        .await
        .expect_err("not offered for d1");
    assert!(
        matches!(&refused, NotAnswered::NotOfferedFor { decision, .. } if decision == "d1"),
        "{refused:?}"
    );
    assert!(
        refused.to_string().contains("Its own Job"),
        "and it names what d1 offered: {refused}"
    );

    let refused = fleet
        .answer_decisions(
            &job,
            id,
            &[picked("d1", "Fold it in"), picked("d1", "Its own Job"), picked("d2", "Reader first")],
        )
        .await
        .expect_err("d1 twice");
    assert!(matches!(&refused, NotAnswered::DecidedTwice { decision } if decision == "d1"));

    let refused = fleet
        .answer_decisions(&job, "01SOMEOTHERQUESTION", &both())
        .await
        .expect_err("not the question outstanding");
    assert!(matches!(refused, NotAnswered::Superseded { .. }));
    assert!(fleet.load(&job).await.unwrap().waiting_on_an_answer());
}

/// The two forms are not interchangeable: one label cannot answer several
/// decisions, and per-decision picks cannot answer a single question.
#[tokio::test]
async fn each_form_is_answered_in_its_own_shape() {
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    let asked = fleet.ask_question(&job, an_interview()).await.unwrap();

    let refused = fleet
        .answer_question(&job, asked.id.as_str(), Some("Fold it in"), None)
        .await
        .expect_err("a label answers one question, not an interview");
    assert!(matches!(refused, NotAnswered::AnInterview { .. }), "{refused:?}");
    assert!(fleet.load(&job).await.unwrap().waiting_on_an_answer());

    let other = planning_job(&fleet, &home, "plan the writer change").await;
    let asked = fleet
        .ask_question(&other, super::a_question())
        .await
        .unwrap();
    let refused = fleet
        .answer_decisions(&other, asked.id.as_str(), &both())
        .await
        .expect_err("picks answer an interview, not one question");
    assert!(matches!(refused, NotAnswered::NotAnInterview { .. }), "{refused:?}");
}

/// A second ask while the interview waits is refused, naming it.
#[tokio::test]
async fn an_interview_outstanding_takes_no_second_ask() {
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    fleet.ask_question(&job, an_interview()).await.unwrap();
    // The Drone stood down, so a second ask finds nothing working; what matters
    // is that nothing was stacked.
    let refused = fleet.ask_question(&job, an_interview()).await;
    assert!(matches!(
        refused,
        Err(NotAsked::NothingIsWorking) | Err(NotAsked::AlreadyAsking { .. })
    ));
}

/// The route's own door: `answer_question` takes the picks in `decisions` and a
/// label in `chose`, and a partial set is a refusal the caller can read.
#[tokio::test]
async fn the_answer_route_takes_every_pick_at_once_and_refuses_a_partial_one() {
    use api::Commands;
    let home = TempDir::new();
    let fleet = planning(&home, a_drone_that_works());
    let job = planning_job(&fleet, &home, "plan the parser change").await;
    let asked = fleet.ask_question(&job, an_interview()).await.unwrap();
    let fleet = Arc::new(fleet);
    let answer = |decisions: Vec<ChosenDecision>| ipc::ChosenAnswer {
        question_id: ipc::QuestionId::from(&asked.id),
        chose: None,
        mode: None,
        decisions,
    };

    let refused = Arc::clone(&fleet)
        .answer_question(ipc::JobId::from(&job), answer(vec![picked("d1", "Fold it in")]))
        .await
        .expect_err("d2 is open");
    assert!(
        format!("{refused:?}").contains("d2"),
        "the refusal names the open decision: {refused:?}"
    );
    assert!(fleet.load(&job).await.unwrap().waiting_on_an_answer());

    Arc::clone(&fleet)
        .answer_question(ipc::JobId::from(&job), answer(both()))
        .await
        .expect("every decision answered");
    assert!(!fleet.load(&job).await.unwrap().waiting_on_an_answer());
}
