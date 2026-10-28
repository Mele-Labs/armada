//! What the Workflows page asks of an agent: a change typed in his words, or new words for a step that
//! keeps drifting, drafted by one short model run, checked as a save is, and written only when he
//! approves. The decision of 27 Oct 2026, "A feature Job walks before it tests".
//!
//! The model is `testkit::FakeJudge`, which answers in turn and keeps what it was asked. What a save
//! writes is the composition root's and is `Planted` here, which keeps what it was asked to save.

use std::sync::Arc;
use std::time::Duration;

use api::Authoring;
use axum::http::StatusCode;
use ipc::{ChangeOrigin, ChangeState, WorkflowChange, WorkflowChanges};
use testkit::{FakeHarness, FakeJudge, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Locating, SavedWorkflow, WorkflowNotSaved};
use crate::tests::daemon::fitted_with;
use crate::tests::http::call;
use crate::tests::repositories::{code, served, Fixture, Planted};
use crate::tests::tmp::TempDir;
use crate::tests::workflow_page::{carried_id, nudged, read, staged_in_repository, STAGED};

/// The id the fixture's own `armada.yml` declares.
const MANIFEST: &str = "01FIXTUREMANIFEST";
/// A nudge inside the week the fixture clock stands in, and one outside it.
const RECENT: &str = "2026-08-24T10:00:00.000Z";
const OLD: &str = "2026-08-10T10:00:00.000Z";
const FACT: &str = "a test before the walk was approved";

struct Rig {
    home: TempDir,
    planted: Arc<Planted>,
    fleet: Arc<Fixture>,
    app: axum::Router,
    judge: Arc<FakeJudge>,
}

/// A Fleet holding `STAGED` in the repository, whose model answers `replies` in turn.
async fn rig(replies: &[&str]) -> Rig {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let judge = Arc::new(FakeJudge::answering_in_turn(replies));
    let mut fittings = fitted_with(&home, FakeWorkProduct::untouched(), FakeHarness::that_listens());
    fittings.locating = Arc::clone(&planted) as Arc<dyn Locating>;
    fittings.judge = Arc::clone(&judge) as Arc<_>;
    let fleet = Arc::new(Fleet::assembled(fittings));
    let app = served(&fleet);
    planted.will_read(read(staged_in_repository()));
    fleet.workflows_changed(&home.path().to_string_lossy());
    Rig { home, planted, fleet, app, judge }
}

/// `STAGED` with a step added before Deliver: what an agent asked to split the work would return.
fn with_refine() -> String {
    STAGED
        .replace(
            "  - id: deliver",
            "  - id: refine\n    label: Refine\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    advance_gate: auto\n  - id: deliver",
        )
        .trim_end()
        .to_string()
}

/// `STAGED` with the Walk step told more: what an agent asked to redraft that step would return.
fn with_walk_worded() -> String {
    STAGED
        .replace("\"Show the mock.\"", "\"Show the mock on fixtures, then stop until the owner approves the walk.\"")
        .trim_end()
        .to_string()
}

fn asking(text: &str) -> String {
    ipc::encode(&ipc::AskWorkflowChange { workflow_id: ipc::WorkflowId::carried("staged"), source: "repository".into(), text: text.into() })
        .expect("plain data")
}

fn suggesting(step: &str) -> String {
    ipc::encode(&ipc::SuggestWorkflowChange {
        workflow_id: ipc::WorkflowId::carried("staged"),
        source: "repository".into(),
        step_id: ipc::StepId::carried(step),
    })
    .expect("plain data")
}

fn change_of(id: i64) -> String {
    ipc::encode(&ipc::WorkflowChangeRef { change: id }).expect("plain data")
}

fn decoded<T: serde::de::DeserializeOwned>(what: &'static str, body: &[u8]) -> T {
    ipc::decode(what, body).unwrap_or_else(|why| panic!("{what}: {why}: {}", String::from_utf8_lossy(body)))
}

impl Rig {
    async fn post(&self, route: &str, body: &str) -> (StatusCode, Vec<u8>) {
        call(&self.app, "POST", route, body).await
    }

    async fn open(&self) -> Vec<WorkflowChange> {
        let (status, body) = call(&self.app, "GET", "/workflows/changes?workflow_id=staged&source=repository", "").await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        decoded::<WorkflowChanges>("changes", &body).changes
    }

    /// Ask in his words and wait for the draft to come back one way or the other.
    async fn asked(&self, text: &str) -> WorkflowChange {
        let (status, body) = self.post("/workflows/changes/ask", &asking(text)).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let begun: WorkflowChange = decoded("a change", &body);
        assert_eq!(begun.state, ChangeState::Drafting, "it answers before the draft is back");
        self.settled(begun.id).await
    }

    /// The change once it is no longer drafting.
    async fn settled(&self, id: i64) -> WorkflowChange {
        for _ in 0..250 {
            let found = self.open().await.into_iter().find(|one| one.id == id);
            if let Some(one) = found.filter(|one| one.state != ChangeState::Drafting) {
                return one;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("change {id} was still drafting after five seconds");
    }

    async fn drifted(&self, jobs: &[(&str, &str)]) {
        for (job, when) in jobs {
            nudged(&self.fleet, "staged", "repository", "walk", job, FACT, when).await;
        }
    }

    /// What Fleet does when a nudge is kept against Walk.
    async fn nudge_kept(&self) {
        self.fleet.drift_suggested(MANIFEST, "staged", "repository", "walk").await;
    }
}

/// **He types a change and gets a draft: the whole new definition, with the one before it.** The
/// answer is the change drafting, at once; the agent was given the definition and his words, on the
/// proposer's model, and nothing was written.
#[tokio::test]
async fn a_change_typed_in_his_words_is_drafted_as_the_whole_new_definition_and_nothing_is_written() {
    let after = with_refine();
    let rig = rig(&[&after]).await;

    let ready = rig.asked("split the work before Deliver into a Refine step").await;

    assert_eq!(ready.state, ChangeState::Ready, "{ready:?}");
    assert_eq!(ready.after.as_deref(), Some(after.as_str()));
    assert_eq!(ready.before, STAGED, "the file as it stood");
    assert_eq!(ready.origin, ChangeOrigin::Asked { text: "split the work before Deliver into a Refine step".into() });
    assert_eq!((ready.source.as_str(), ready.workflow_id.as_str(), ready.step_id.is_none()), ("repository", "staged", true));
    let asked = rig.judge.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains(STAGED) && asked[0].contains("split the work before Deliver"), "{}", asked[0]);
    let proposer = rig.fleet.proposing().expect("a model to ask").model.as_str().to_string();
    assert_eq!(rig.judge.models(), [proposer], "the model the proposer reads with");
    assert!(rig.planted.saves().is_empty(), "nothing saves without his Approve");
    assert_eq!(rig.open().await, [ready]);
}

/// **A reply wrapped in a code fence is the definition inside it.**
#[tokio::test]
async fn a_draft_the_model_wrapped_in_a_fence_is_read_without_it() {
    let after = with_refine();
    let rig = rig(&[&format!("```json\n{after}\n```")]).await;
    let ready = rig.asked("add Refine").await;
    assert_eq!(ready.after.as_deref(), Some(after.as_str()));
}

/// **Each refusal of an ask is a 422 or a 404 in Fleet's words and keeps no change.**
#[tokio::test]
async fn an_ask_is_refused_for_armadas_own_for_blank_words_and_for_a_place_with_none() {
    let rig = rig(&[]).await;
    let armadas = ipc::encode(&ipc::AskWorkflowChange { workflow_id: ipc::WorkflowId::carried("feature"), source: "armada".into(), text: "x".into() })
        .unwrap();
    let (status, body) = rig.post("/workflows/changes/ask", &armadas).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_is_armadas"));

    let (status, body) = rig.post("/workflows/changes/ask", &asking("   ")).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_change_empty"));

    let nowhere = ipc::encode(&ipc::AskWorkflowChange { workflow_id: ipc::WorkflowId::carried("staged"), source: "kit".into(), text: "x".into() })
        .unwrap();
    let (status, body) = rig.post("/workflows/changes/ask", &nowhere).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.no_such_workflow_definition"));

    assert!(rig.judge.asked().is_empty(), "no model was asked for any of them");
    assert!(rig.open().await.is_empty());
}

/// **A draft the checks refuse is asked for once more, with the loader's own sentence and the answer
/// that did not fit**; a second that does not fit leaves the change failed, with that sentence.
#[tokio::test]
async fn a_draft_that_does_not_fit_is_redrafted_once_with_the_reason_and_then_fails_with_it() {
    let (first, second) = (with_refine(), with_walk_worded());
    let rig = rig(&[&first, &second]).await;
    let unfit = |why: &str| Err(WorkflowNotSaved::Unfit { why: why.to_string() });
    rig.planted.will_check(vec![unfit("step `refine` needs `lint`, and the declared Checks are `test`"), Ok(())]);

    let ready = rig.asked("add Refine").await;

    assert_eq!((ready.state, ready.after.as_deref()), (ChangeState::Ready, Some(second.as_str())));
    let asked = rig.judge.asked();
    assert_eq!(asked.len(), 2);
    assert!(asked[1].contains("needs `lint`") && asked[1].contains(&first), "the reason and the answer go back:\n{}", asked[1]);

    let rig = self::rig(&[&first, &second]).await;
    rig.planted.will_check(vec![unfit("first reason"), unfit("second reason")]);
    let failed = rig.asked("add Refine").await;
    assert_eq!(failed.state, ChangeState::Failed);
    assert_eq!(failed.reason.as_deref(), Some("second reason"), "the last sentence is the one he reads");
    assert_eq!(failed.after, None);
    assert_eq!(rig.judge.asked().len(), 2, "once more and no more");
}

/// **The draft must stay the workflow it was asked about**: another `workflow_id` would be saved as a
/// second workflow, or over another.
#[tokio::test]
async fn a_draft_that_renames_the_workflow_does_not_fit() {
    let renamed = STAGED.replace("workflow_id: staged", "workflow_id: other");
    let rig = rig(&[&renamed, &renamed]).await;
    let failed = rig.asked("rename it").await;
    assert_eq!(failed.state, ChangeState::Failed);
    assert!(failed.reason.as_deref().is_some_and(|why| why.contains("workflow_id") && why.contains("`staged`")), "{failed:?}");
}

/// **An answer that is the definition unchanged is a failure without a second try**: there is nothing
/// for him to approve, and asking again would ask the same.
#[tokio::test]
async fn a_draft_that_changes_nothing_fails_and_is_not_asked_for_again() {
    let rig = rig(&[STAGED, STAGED]).await;
    let failed = rig.asked("do nothing in particular").await;
    assert_eq!(failed.state, ChangeState::Failed);
    assert!(failed.reason.as_deref().is_some_and(|why| why.contains("unchanged")), "{failed:?}");
    assert_eq!(rig.judge.asked().len(), 1);
}

/// **A model that cannot be reached leaves the change failed with the call's own line.**
#[tokio::test]
async fn a_model_call_that_fails_leaves_the_change_failed() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let mut fittings = fitted_with(&home, FakeWorkProduct::untouched(), FakeHarness::that_listens());
    fittings.locating = Arc::clone(&planted) as Arc<dyn Locating>;
    fittings.judge = Arc::new(FakeJudge::that_fails("the network")) as Arc<_>;
    let fleet = Arc::new(Fleet::assembled(fittings));
    let rig = Rig { app: served(&fleet), judge: Arc::new(FakeJudge::saying("")), home, planted, fleet };
    rig.planted.will_read(read(staged_in_repository()));
    rig.fleet.workflows_changed(&rig.home.path().to_string_lossy());

    let failed = rig.asked("add Refine").await;

    assert_eq!(failed.state, ChangeState::Failed);
    assert!(failed.reason.as_deref().is_some_and(|why| why.starts_with("The drafting call ended")), "{failed:?}");
}

/// **A suggestion may change one step's `instructions` and nothing else**: a draft that moves another
/// field is unfit, asked for once more, and fails if it still does.
#[tokio::test]
async fn a_suggestion_that_changes_more_than_the_steps_words_does_not_fit() {
    let relabelled = with_walk_worded().replace("label: Walk", "label: Show it");
    let rig = rig(&[&relabelled, &relabelled]).await;
    rig.drifted(&[("job-1", RECENT)]).await;

    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("walk")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let failed = rig.settled(decoded::<WorkflowChange>("change", &body).id).await;

    assert_eq!(failed.state, ChangeState::Failed);
    assert!(failed.reason.as_deref().is_some_and(|why| why.contains("`instructions` of step `walk` and nothing else")), "{failed:?}");
    assert!(rig.judge.asked()[1].contains("nothing else"), "the reason went back");
}

/// **Suggest a change drafts new words for the step from the nudges that count**, naming the Jobs and
/// what happened, and the card carries the step and the nudges it was drawn from.
#[tokio::test]
async fn suggest_a_change_drafts_the_steps_words_from_the_nudges_that_count() {
    let after = with_walk_worded();
    let rig = rig(&[&after]).await;
    rig.drifted(&[("job-1", RECENT), ("job-2", OLD)]).await;

    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("walk")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let ready = rig.settled(decoded::<WorkflowChange>("change", &body).id).await;

    assert_eq!(ready.state, ChangeState::Ready, "{ready:?}");
    assert_eq!(ready.step_id.as_ref().map(|one| one.as_str()), Some("walk"));
    assert!(matches!(&ready.origin, ChangeOrigin::Drift { nudges } if nudges.len() == 1), "{:?}", ready.origin);
    assert_eq!(ready.after.as_deref(), Some(after.as_str()));
    let asked = rig.judge.asked();
    assert!(asked[0].contains(FACT) && asked[0].contains("Show the mock."), "the facts and the step's words:\n{}", asked[0]);
}

/// **Each refusal of a suggestion**: a step the definition lacks, a step nobody drifted from, Armada's
/// own, and a step that already has a change waiting.
#[tokio::test]
async fn a_suggestion_is_refused_for_a_missing_step_no_drift_armadas_own_and_a_step_with_a_change_waiting() {
    let rig = rig(&[&with_walk_worded()]).await;

    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("nowhere")).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_step_not_here"));
    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("walk")).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_nothing_drifted"));
    let armadas = ipc::encode(&ipc::SuggestWorkflowChange {
        workflow_id: ipc::WorkflowId::carried("feature"),
        source: "armada".into(),
        step_id: ipc::StepId::carried("walk"),
    })
    .unwrap();
    let (status, body) = rig.post("/workflows/changes/suggest", &armadas).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_is_armadas"));

    rig.drifted(&[("job-1", RECENT)]).await;
    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("walk")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, body) = rig.post("/workflows/changes/suggest", &suggesting("walk")).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_change_open"));
}

// ---- Armada drafts one on its own -----------------------------------------------------------------------

/// **One step of one workflow drifting twice in a week is drafted for, with no press.** The second
/// nudge kept begins it, from the two nudges, once.
#[tokio::test]
async fn a_step_that_drifts_twice_in_a_week_is_drafted_for_on_its_own_once() {
    let rig = rig(&[&with_walk_worded()]).await;
    rig.drifted(&[("job-1", RECENT)]).await;
    rig.nudge_kept().await;
    assert!(rig.open().await.is_empty(), "one drift waits for a second");

    rig.drifted(&[("job-2", RECENT)]).await;
    rig.nudge_kept().await;
    let begun = rig.open().await;
    assert_eq!(begun.len(), 1);
    let ready = rig.settled(begun[0].id).await;
    assert_eq!(ready.state, ChangeState::Ready, "{ready:?}");
    assert!(matches!(&ready.origin, ChangeOrigin::Drift { nudges } if nudges.len() == 2), "{:?}", ready.origin);
    assert_eq!(ready.step_id.as_ref().map(|one| one.as_str()), Some("walk"));

    rig.drifted(&[("job-3", RECENT)]).await;
    rig.nudge_kept().await;
    assert_eq!(rig.open().await.len(), 1, "a step with a change waiting is not drafted for again");
    assert_eq!(rig.judge.asked().len(), 1);
}

/// **A nudge he marked not a drift does not count**, nor one older than a week.
#[tokio::test]
async fn a_nudge_marked_not_a_drift_or_older_than_a_week_does_not_count_toward_a_suggestion() {
    let rig = rig(&[&with_walk_worded()]).await;
    rig.drifted(&[("job-1", RECENT), ("job-2", RECENT)]).await;
    let first = rig.fleet.store().lock().await.workflow_nudges("staged", "repository", "walk", 10).unwrap()[0].id;
    let (status, _) = rig.post("/workflows/not_a_drift", &format!(r#"{{"nudge":{first}}}"#)).await;
    assert_eq!(status, StatusCode::OK);

    rig.nudge_kept().await;
    assert!(rig.open().await.is_empty(), "one that counts is one");

    rig.drifted(&[("job-3", OLD)]).await;
    rig.nudge_kept().await;
    assert!(rig.open().await.is_empty(), "a drift from last month is not this week's");
    assert!(rig.judge.asked().is_empty());

    rig.drifted(&[("job-4", RECENT)]).await;
    rig.nudge_kept().await;
    assert_eq!(rig.open().await.len(), 1, "two that count, one of them new");
}

/// **A dismissed suggestion lets the next drift draft another**; a step is not suggested for twice at once.
#[tokio::test]
async fn a_dismissed_suggestion_is_drafted_again_when_the_step_drifts_again() {
    let rig = rig(&[&with_walk_worded(), &with_walk_worded()]).await;
    rig.drifted(&[("job-1", RECENT), ("job-2", RECENT)]).await;
    rig.nudge_kept().await;
    let first = rig.settled(rig.open().await[0].id).await;
    let (status, _) = rig.post("/workflows/changes/dismiss", &change_of(first.id)).await;
    assert_eq!(status, StatusCode::OK);

    rig.drifted(&[("job-3", RECENT)]).await;
    rig.nudge_kept().await;

    let second = rig.open().await;
    assert_eq!(second.len(), 1);
    assert_ne!(second[0].id, first.id);
}

/// **Armada's own definitions get none**: they are read-only.
#[tokio::test]
async fn armadas_own_definitions_are_never_drafted_for() {
    let rig = rig(&[&with_walk_worded()]).await;
    for job in ["job-1", "job-2"] {
        nudged(&rig.fleet, "feature", "armada", "walk", job, FACT, RECENT).await;
    }
    rig.fleet.drift_suggested(MANIFEST, "feature", "armada", "walk").await;
    assert!(rig.judge.asked().is_empty());
    let (status, body) = call(&rig.app, "GET", "/workflows/changes?workflow_id=feature&source=armada", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(decoded::<WorkflowChanges>("changes", &body).changes.is_empty());
}

/// What an agent answers when the fields can't say what was asked: `CAN'T`, then what is missing.
const CANT: &str = "CAN'T\nA step can't end when a document is approved.\n- A step can't end when a sketch is approved.";

/// **An agent that answers `CAN'T` leaves a change Armada can't make**, with each missing line as the
/// person reads it, and the definition as it stands read step by step; it is not asked again, nothing
/// is drafted, and he can dismiss it.
#[tokio::test]
async fn a_cant_answer_is_kept_as_a_change_armada_cannot_make_with_what_it_lacks() {
    let rig = rig(&[CANT]).await;

    let cannot = rig.asked("Propose should end when a walk, a sketch or a document is approved").await;

    assert_eq!(cannot.state, ChangeState::Cannot, "{cannot:?}");
    assert_eq!(cannot.missing, ["A step can't end when a document is approved.", "A step can't end when a sketch is approved."]);
    assert_eq!((cannot.after.as_deref(), cannot.after_steps.as_ref(), cannot.reason.as_deref()), (None, None, None));
    let before: Vec<&str> = cannot.before_steps.iter().flatten().map(|step| step.id.as_str()).collect();
    assert_eq!(before, ["propose", "plan", "walk", "deliver"]);
    assert_eq!(rig.judge.asked().len(), 1, "a can't is not asked for again");
    assert!(rig.judge.asked()[0].contains("never from its words"), "the agent was told what ends a step");
    assert_eq!(rig.open().await, [cannot.clone()]);

    let (status, body) = rig.post("/workflows/changes/dismiss", &change_of(cannot.id)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(rig.open().await.is_empty());
}

/// **A ready change reads each side step by step through the rail**, so a step the draft adds says
/// what ends it, which its text alone cannot.
#[tokio::test]
async fn a_ready_change_reads_each_side_through_the_rail_so_a_new_step_says_what_ends_it() {
    let rig = rig(&[&with_refine()]).await;

    let ready = rig.asked("split the work before Deliver into a Refine step").await;

    assert_eq!(ready.state, ChangeState::Ready, "{ready:?}");
    let ends = |steps: &Option<Vec<ipc::ChangeStep>>| -> Vec<(String, Option<String>)> {
        steps.iter().flatten().map(|step| (step.id.as_str().to_string(), step.ends_when.clone())).collect()
    };
    let said = |pairs: &[(&str, &str)]| -> Vec<(String, Option<String>)> {
        pairs.iter().map(|(id, end)| (id.to_string(), Some(end.to_string()))).collect()
    };
    assert_eq!(
        ends(&ready.before_steps),
        said(&[("propose", "you reply"), ("plan", "you approve the plan"), ("walk", "you approve the walk"), ("deliver", "it is merged")])
    );
    assert_eq!(
        ends(&ready.after_steps),
        said(&[
            ("propose", "you reply"),
            ("plan", "you approve the plan"),
            ("walk", "you approve the walk"),
            ("refine", "it is pushed"),
            ("deliver", "it is merged"),
        ])
    );
    let walk = ready.after_steps.iter().flatten().find(|step| step.id.as_str() == "walk").expect("walk");
    assert_eq!((walk.instructions.as_deref(), walk.passed_over_when.as_deref()), (Some("Show the mock."), Some("nothing would be seen")));
}

/// **Build is refused for a change Armada can make**, and for one it never kept.
#[tokio::test]
async fn build_is_refused_on_a_change_armada_can_make() {
    let rig = rig(&[&with_refine()]).await;
    let (status, body) = rig.post("/workflows/changes/build", &change_of(99)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_workflow_change"));

    let ready = rig.asked("add Refine").await;
    let (status, body) = rig.post("/workflows/changes/build", &change_of(ready.id)).await;

    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_change_not_buildable"));
    assert_eq!(rig.open().await.iter().map(|one| one.state).collect::<Vec<_>>(), [ChangeState::Ready], "it stays");
}

/// A change kept as Armada can't make it, as `change_drafted` leaves one, in a rig's own store.
async fn kept_cannot(rig: &crate::tests::session_host::Rig, manifest: &str) -> i64 {
    let mut kept = rig.fleet.store().lock().await;
    let new = store::NewWorkflowChange {
        manifest_id: manifest,
        workflow_id: "feature",
        source: "repository",
        step_id: None,
        asked: Some("Propose should end when a document is approved"),
        nudges: &[],
        before_text: STAGED,
    };
    let id = kept.begin_workflow_change(&new, RECENT).unwrap();
    let missing = [String::from("A step can't end when a document is approved."), String::from("A step can't end when a sketch is approved.")];
    assert!(kept.workflow_change_cannot(id, &missing, RECENT).unwrap());
    id
}

/// **Build starts a Job on Armada's own repository**, titled after the first missing line and briefed
/// with the workflow, his request and every missing line, and the change closes.
#[tokio::test]
async fn building_support_starts_a_job_on_armadas_repository_briefed_with_the_request_and_what_is_missing() {
    use crate::tests::session_dispute::{brief_of, serving_armada};
    let rig = serving_armada().await;
    let id = kept_cannot(&rig, "armada").await;

    let building = Authoring::build_workflow_support(Arc::clone(&rig.fleet), ipc::WorkflowChangeRef { change: id })
        .await
        .expect("building");

    let kept = rig.fleet.store().lock().await.session(building.job_id.as_str()).unwrap().expect("a Job");
    assert_eq!(kept.manifest_id.as_deref(), Some("armada"));
    assert_eq!(kept.title.as_deref(), Some("Build workflow support: A step can't end when a document is approved"));
    let brief = brief_of(&rig, building.job_id.as_str()).await;
    for said in [
        "`feature`",
        "Propose should end when a document is approved",
        "- A step can't end when a document is approved.",
        "- A step can't end when a sketch is approved.",
        "pull request",
    ] {
        assert!(brief.contains(said), "the brief lacks {said:?}:\n{brief}");
    }
    let closed = rig.fleet.store().lock().await.workflow_change(id).unwrap().expect("kept");
    assert_eq!(closed.state, store::CHANGE_DISMISSED);
}

/// **Where this Fleet does not serve Armada's repository, nothing starts** and the change stays.
#[tokio::test]
async fn building_support_is_refused_where_armada_is_not_served() {
    use crate::tests::session_dispute::serving;
    let rig = serving("somewhere-else").await;
    let id = kept_cannot(&rig, "somewhere-else").await;

    let refused = Authoring::build_workflow_support(Arc::clone(&rig.fleet), ipc::WorkflowChangeRef { change: id })
        .await
        .expect_err("not served");

    assert_eq!(refused.error().code, "fleet.workflow_armada_not_served");
    assert!(rig.fleet.store().lock().await.find_sessions(&store::SessionSearch::default()).unwrap().is_empty());
    assert_eq!(rig.fleet.store().lock().await.workflow_change(id).unwrap().expect("kept").state, store::CHANGE_CANNOT);
}

// ---- approve, dismiss, send back ------------------------------------------------------------------------

fn saved(file: &str) -> SavedWorkflow {
    SavedWorkflow { id: carried_id("staged"), file: file.to_string(), replaced: true }
}

/// **Approve writes "after" through the save path, with overwrite**, in the change's place; the change
/// is approved and leaves the list.
#[tokio::test]
async fn approving_writes_the_draft_through_the_save_path_with_overwrite() {
    let after = with_refine();
    let rig = rig(&[&after]).await;
    let ready = rig.asked("add Refine").await;
    // Approve reads the folders again before it writes, so the file is planted as it stands now.
    rig.planted.will_read(read(staged_in_repository()));
    rig.planted.will_save(Ok(saved("/repo/.armada/workflows/staged.json")));

    let (status, body) = rig.post("/workflows/changes/approve", &change_of(ready.id)).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let approved: ipc::WorkflowChangeApproved = decoded("approved", &body);
    assert_eq!(approved.change.state, ChangeState::Approved);
    assert_eq!(approved.saved.file, "/repo/.armada/workflows/staged.json");
    assert_eq!(
        rig.planted.saves(),
        [ipc::SaveWorkflow { scope: ipc::WorkflowScope::Repository, definition: after, overwrite: true }]
    );
    assert!(rig.open().await.is_empty());
    let (status, body) = rig.post("/workflows/changes/approve", &change_of(ready.id)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_change_not_ready"), "once");
}

/// **Approve is refused if the file changed since "before"**, and nothing is written: an approve must
/// not undo an edit made in between.
#[tokio::test]
async fn approving_is_refused_when_the_file_changed_since_the_draft_began() {
    let rig = rig(&[&with_refine()]).await;
    let ready = rig.asked("add Refine").await;
    let edited = STAGED.replace("Show the mock.", "Show the mock, edited by hand.");
    rig.planted.will_read(read(vec![config::Written::in_repository(crate::tests::workflow_page::at("repo", "staged.json"), edited)]));

    let (status, body) = rig.post("/workflows/changes/approve", &change_of(ready.id)).await;

    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_changed_since"));
    assert!(rig.planted.saves().is_empty());
    assert_eq!(rig.open().await.iter().map(|one| one.state).collect::<Vec<_>>(), [ChangeState::Ready], "it stays for him to dismiss");
}

/// **The other refusals of approve, dismiss and send back**: no such change, one not ready, one settled.
#[tokio::test]
async fn approve_dismiss_and_send_back_refuse_what_is_not_there_or_not_theirs() {
    let rig = rig(&[&with_refine()]).await;
    for route in ["/workflows/changes/approve", "/workflows/changes/dismiss"] {
        let (status, body) = rig.post(route, &change_of(99)).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_workflow_change"), "{route}");
    }
    let back = ipc::encode(&ipc::SendBackWorkflowChange { change: 99, notes: "no".into() }).unwrap();
    let (status, body) = rig.post("/workflows/changes/send_back", &back).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_workflow_change"));

    let ready = rig.asked("add Refine").await;
    let blank = ipc::encode(&ipc::SendBackWorkflowChange { change: ready.id, notes: "  ".into() }).unwrap();
    let (status, body) = rig.post("/workflows/changes/send_back", &blank).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_change_empty"));

    let (status, body) = rig.post("/workflows/changes/dismiss", &change_of(ready.id)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(decoded::<WorkflowChange>("change", &body).state, ChangeState::Dismissed);
    let (status, body) = rig.post("/workflows/changes/dismiss", &change_of(ready.id)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_change_settled"));
    let back = ipc::encode(&ipc::SendBackWorkflowChange { change: ready.id, notes: "no".into() }).unwrap();
    let (status, body) = rig.post("/workflows/changes/send_back", &back).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_change_not_sendable"));
    assert!(rig.open().await.is_empty());
}

/// **Sent back with notes, the change is drafted again** from the definition as it was, with the draft
/// he sent back and what he said was wrong with it; nothing saves.
#[tokio::test]
async fn sending_back_with_notes_redrafts_from_the_draft_and_his_notes() {
    let (first, second) = (with_refine(), with_walk_worded());
    let rig = rig(&[&first, &second]).await;
    let ready = rig.asked("add Refine").await;

    let back = ipc::encode(&ipc::SendBackWorkflowChange { change: ready.id, notes: "keep Deliver last and reword Walk instead".into() })
        .unwrap();
    let (status, body) = rig.post("/workflows/changes/send_back", &back).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let again: WorkflowChange = decoded("change", &body);
    assert_eq!((again.state, again.notes.as_deref()), (ChangeState::Drafting, Some("keep Deliver last and reword Walk instead")));
    assert_eq!(again.after, None, "the old draft is not shown as the new one");

    let redrafted = rig.settled(ready.id).await;
    assert_eq!((redrafted.state, redrafted.after.as_deref()), (ChangeState::Ready, Some(second.as_str())));
    let asked = rig.judge.asked();
    assert!(asked[1].contains(STAGED) && asked[1].contains(&first) && asked[1].contains("reword Walk instead"), "{}", asked[1]);
    assert!(rig.planted.saves().is_empty());
}

/// **A change that failed is sent back with notes, and the notes reach the model** though there is no
/// draft to send with them.
#[tokio::test]
async fn a_failed_change_sent_back_with_notes_is_redrafted_with_the_notes() {
    let rig = rig(&[STAGED, &with_refine()]).await;
    let failed = rig.asked("add Refine").await;
    assert_eq!(failed.state, ChangeState::Failed);

    let back = ipc::encode(&ipc::SendBackWorkflowChange { change: failed.id, notes: "Refine goes before Deliver".into() }).unwrap();
    let (status, body) = rig.post("/workflows/changes/send_back", &back).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let ready = rig.settled(failed.id).await;

    assert_eq!(ready.state, ChangeState::Ready, "{ready:?}");
    assert!(rig.judge.asked()[1].contains("Refine goes before Deliver"), "{}", rig.judge.asked()[1]);
}

/// **A change left drafting by a Fleet that stopped is read as failed**, and can be sent back.
#[tokio::test]
async fn a_draft_lost_when_fleet_stopped_is_read_as_failed() {
    let rig = rig(&[]).await;
    let begun = rig
        .fleet
        .store()
        .lock()
        .await
        .begin_workflow_change(
            &store::NewWorkflowChange {
                manifest_id: MANIFEST,
                workflow_id: "staged",
                source: "repository",
                step_id: None,
                asked: Some("add Refine"),
                nudges: &[],
                before_text: STAGED,
            },
            "2026-08-26T08:00:00.000Z",
        )
        .unwrap();

    let found = rig.open().await;

    assert_eq!(found.len(), 1);
    assert_eq!((found[0].id, found[0].state), (begun, ChangeState::Failed));
    assert!(found[0].reason.as_deref().is_some_and(|why| why.contains("lost when Fleet stopped")));
}

// ---- Propose to Armada ----------------------------------------------------------------------------------

/// **Propose to Armada starts a Job on Armada's own repository** and sends it a brief that says to set
/// the carried file to the definition and land it as a pull request.
#[tokio::test]
async fn propose_to_armada_starts_a_job_on_armadas_repository_briefed_to_land_the_definition() {
    use crate::tests::session_dispute::{brief_of, serving_armada};
    let rig = serving_armada().await;
    let definition = "{\"workflow_id\": \"feature\", \"name\": \"feature\"}";

    let proposed = Authoring::propose_workflow_to_armada(
        Arc::clone(&rig.fleet),
        ipc::ProposeToArmada { workflow_id: ipc::WorkflowId::carried("feature"), definition: definition.into() },
    )
    .await
    .expect("proposed");

    assert_eq!(proposed.send_failed, None);
    let kept = rig.fleet.store().lock().await.session(proposed.session_id.as_str()).unwrap().expect("a Job");
    assert_eq!(kept.manifest_id.as_deref(), Some("armada"));
    let brief = brief_of(&rig, proposed.session_id.as_str()).await;
    for said in [definition, ".armada/workflows/", "pull request", "every install", "`feature`"] {
        assert!(brief.contains(said), "the brief lacks {said:?}:\n{brief}");
    }
}

/// **Where this Fleet does not serve Armada's repository, nothing starts and Fleet says so**, and a
/// blank definition is refused before it looks.
#[tokio::test]
async fn propose_to_armada_is_refused_where_armada_is_not_served_and_for_a_blank_definition() {
    use crate::tests::session_dispute::serving;
    let rig = serving("somewhere-else").await;
    let propose = |definition: &str| ipc::ProposeToArmada { workflow_id: ipc::WorkflowId::carried("feature"), definition: definition.into() };

    let refused = Authoring::propose_workflow_to_armada(Arc::clone(&rig.fleet), propose("{}")).await.expect_err("not served");
    assert_eq!(refused.error().code, "fleet.workflow_armada_not_served");
    let refused = Authoring::propose_workflow_to_armada(Arc::clone(&rig.fleet), propose("  ")).await.expect_err("blank");
    assert_eq!(refused.error().code, "fleet.workflow_change_empty");
    let started = rig.fleet.store().lock().await.find_sessions(&store::SessionSearch::default()).unwrap();
    assert!(started.is_empty(), "{started:?}");
}

/// The contract's section 5h, whole: the shipped words are the drafted block with the slots kept.
#[test]
fn the_shipped_question_is_the_contracts_drafted_block() {
    let contract = include_str!("../../../../docs/contracts/agent-prompt/5h-the-workflow-change.md");
    let drafted = contract.split("~~~\n").nth(1).expect("the drafted block").trim_end();
    let unwrapped = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(unwrapped(crate::workflow_changes::QUESTION), unwrapped(drafted));
}

/// **An override that drops the definition is refused**, as any prompt that loses a slot is.
#[tokio::test]
async fn an_override_that_drops_the_definition_is_refused() {
    let rig = rig(&[]).await;
    let refused = rig
        .fleet
        .save_settings_now(ipc::SaveSettings {
            changes: [("prompts.workflowChange".to_string(), Some(ipc::SettingValue::Text("{request} {attempt}".to_string())))].into(),
        })
        .await;
    assert!(refused.is_err(), "a prompt with no definition rewrites nothing");
}

// ---- a Job proposes a new workflow -----------------------------------------------------------------------

const FRESH: &str = "version: 1\nworkflow_id: fresh\nname: fresh\nsteps: []";

fn proposing(scope: ipc::WorkflowScope, definition: &str) -> String {
    ipc::encode(&ipc::ProposeNewWorkflow { workflow_id: ipc::WorkflowId::carried("fresh"), definition: definition.into(), scope }).expect("plain data")
}

impl Rig {
    async fn open_fresh(&self, source: &str) -> Vec<WorkflowChange> {
        let (_, body) = call(&self.app, "GET", &format!("/workflows/changes?workflow_id=fresh&source={source}"), "").await;
        decoded::<WorkflowChanges>("changes", &body).changes
    }

    async fn proposed(&self, scope: ipc::WorkflowScope) -> WorkflowChange {
        let (status, body) = self.post("/workflows/changes/propose_new", &proposing(scope, FRESH)).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        decoded("a change", &body)
    }
}

/// **A proposed workflow is ready at once with no earlier text, listed in its scope, and nothing is written.**
#[tokio::test]
async fn a_proposed_workflow_is_held_ready_in_its_scope_and_nothing_is_written() {
    let rig = rig(&[]).await;
    let change = rig.proposed(ipc::WorkflowScope::Kit).await;
    assert_eq!((change.state, change.origin.clone()), (ChangeState::Ready, ChangeOrigin::New));
    assert_eq!((change.before.as_str(), change.after.as_deref(), change.source.as_str()), ("", Some(FRESH), "kit"));
    assert_eq!(rig.open_fresh("kit").await, [change]);
    assert!(rig.open_fresh("repository").await.is_empty());
    assert!(rig.planted.saves().is_empty());
    assert!(rig.judge.asked().is_empty(), "no model drafts it");
}

/// **A blank definition and an id the scope already holds are refused, and keep nothing.**
#[tokio::test]
async fn a_proposal_is_refused_when_blank_or_when_the_scope_already_holds_the_id() {
    let rig = rig(&[]).await;
    let (status, body) = rig.post("/workflows/changes/propose_new", &proposing(ipc::WorkflowScope::Repository, "  ")).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_change_empty"));

    rig.planted.will_check(vec![Err(WorkflowNotSaved::Exists { id: "fresh".into(), file: "/repo/.armada/workflows/fresh.json".into() })]);
    let (status, body) = rig.post("/workflows/changes/propose_new", &proposing(ipc::WorkflowScope::Repository, FRESH)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_exists"));
    assert_eq!(rig.planted.checks().last().map(|one| one.overwrite), Some(false));
    assert!(rig.open_fresh("repository").await.is_empty());
}

/// **Approve saves it to the scope without replacing anything**, and it leaves the list.
#[tokio::test]
async fn approving_a_proposed_workflow_saves_it_to_its_scope_without_overwrite() {
    let rig = rig(&[]).await;
    let change = rig.proposed(ipc::WorkflowScope::Kit).await;
    rig.planted.will_save(Ok(SavedWorkflow { id: carried_id("fresh"), file: "/home/.armada/workflows/fresh.json".into(), replaced: false }));

    let (status, body) = rig.post("/workflows/changes/approve", &change_of(change.id)).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let approved: ipc::WorkflowChangeApproved = decoded("approved", &body);
    assert_eq!(approved.change.state, ChangeState::Approved);
    assert_eq!(rig.planted.saves(), [ipc::SaveWorkflow { scope: ipc::WorkflowScope::Kit, definition: FRESH.into(), overwrite: false }]);
    assert!(rig.open_fresh("kit").await.is_empty());
}

/// **If the id was taken after the proposal, Approve's refusal is shown and the change stays ready.**
#[tokio::test]
async fn approve_refuses_a_proposed_workflow_whose_id_was_taken_meanwhile() {
    let rig = rig(&[]).await;
    let change = rig.proposed(ipc::WorkflowScope::Repository).await;
    rig.planted.will_save(Err(WorkflowNotSaved::Exists { id: "fresh".into(), file: "/repo/.armada/workflows/fresh.json".into() }));
    let (status, body) = rig.post("/workflows/changes/approve", &change_of(change.id)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_exists"));
    assert_eq!(rig.open_fresh("repository").await.iter().map(|one| one.state).collect::<Vec<_>>(), [ChangeState::Ready]);
}

/// **Dismiss drops it and writes nothing.**
#[tokio::test]
async fn dismissing_a_proposed_workflow_writes_nothing() {
    let rig = rig(&[]).await;
    let change = rig.proposed(ipc::WorkflowScope::Repository).await;
    let (status, body) = rig.post("/workflows/changes/dismiss", &change_of(change.id)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(decoded::<WorkflowChange>("change", &body).state, ChangeState::Dismissed);
    assert!(rig.open_fresh("repository").await.is_empty());
    assert!(rig.planted.saves().is_empty());
}
