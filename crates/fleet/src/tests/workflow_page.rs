//! What the Workflows page asks of Fleet beside a definition's text: each step's words, what ends it
//! and the nudges that still count there; a nudge marked not a drift; and a copy deleted or reset to
//! Armada's. The decision of 27 Oct 2026, "A feature Job walks before it tests".
//!
//! What a delete removes and a reset writes is the composition root's and is tested there against real
//! files (`armada::tests::authoring`). What is Fleet's is which copies it will touch at all, the words of
//! a refusal, and that it reads again before answering.

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use store::{KeptSession, SessionFigures, SessionState};

use crate::repositories::{Catalogued, SavedWorkflow, WorkflowNotRemoved, WorkflowNotSaved};
use crate::tests::http::call;
use crate::tests::repositories::{a_fleet_reading, code, served, Fixture, Planted};
use crate::tests::tmp::TempDir;

/// Propose waits for a reply, Plan records the plan, Walk is walked or passed over, Deliver merges.
const STAGED: &str = "version: 1\nworkflow_id: staged\nname: staged\nsteps:
  - id: propose
    label: Propose
    instructions: \"Say what you took the request to mean.\"
    awaits_reply: true
    evidence: {submitted: {type: diff}}
    delivers: false
    advance_gate: auto
  - id: plan
    label: Plan
    records_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - {type: plan_recorded, min_tasks: 1}
    delivers: false
    advance_gate: auto
  - id: walk
    label: Walk
    instructions: \"Show the mock.\"
    walks: true
    evidence: {submitted: {type: diff}}
    delivers: false
    advance_gate: auto
  - id: deliver
    label: Deliver
    evidence: {submitted: {type: diff}}
    delivers: true
    advance_gate: auto
";

fn at(place: &str, file: &str) -> PathBuf {
    PathBuf::from(format!("/{place}/workflows/{file}"))
}

/// The three places read: Armada's carried set, then whatever `written` adds.
fn read(written: Vec<config::Written>) -> Catalogued {
    let mut all = config::carried();
    all.extend(written);
    let (workflows, left_out, files) = config::Catalogue::of(all, &config::Roster::of(["haiku", "sonnet", "opus"]))
        .resolve(&crate::tests::daemon::manifest())
        .into_parts();
    let said: Vec<String> = left_out.iter().map(ToString::to_string).collect();
    assert!(said.is_empty(), "the fixture's definitions are all held: {said:?}");
    Catalogued { workflows, left_out: left_out.iter().map(crate::wire::left_out_workflow).collect(), files }
}

/// A Fleet that holds `written` over what Armada carries.
async fn holding(written: Vec<config::Written>) -> (TempDir, Arc<Planted>, Arc<Fixture>, axum::Router) {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    planted.will_read(read(written));
    fleet.workflows_changed(&home.path().to_string_lossy());
    (home, planted, fleet, app)
}

fn staged_in_repository() -> Vec<config::Written> {
    vec![config::Written::in_repository(at("repo", "staged.json"), STAGED.to_string())]
}

async fn steps(app: &axum::Router, query: &str) -> (StatusCode, Vec<u8>) {
    call(app, "GET", &format!("/workflows/steps?{query}"), "").await
}

fn decoded(body: &[u8]) -> ipc::WorkflowSteps {
    ipc::decode("steps", body).expect("a workflow's steps")
}

async fn nudged(fleet: &Fixture, workflow: &str, source: &str, step: &str, job: &str, fact: &str, when: &str) {
    fleet.store().lock().await.keep_workflow_nudge(workflow, source, step, job, fact, when).expect("a nudge is kept");
}

async fn titled(fleet: &Fixture, id: &str, title: &str) {
    fleet
        .store()
        .lock()
        .await
        .keep_session(&KeptSession {
            id: id.to_string(),
            harness: "a-harness".to_string(),
            origin: "bridge".to_string(),
            manifest_id: None,
            cwd: "/slot".to_string(),
            title: Some(title.to_string()),
            state: SessionState::Live,
            started_at: "2026-10-27T09:00:00Z".to_string(),
            last_seen_at: "2026-10-27T09:00:00Z".to_string(),
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            figures: SessionFigures::default(),
            mod_version: None,
        })
        .expect("a session is kept");
}

/// **Each step is served with its words, what ends it, and when it is passed over**, in the order the
/// definition gives them, from the rules the rail places a Job by. A fact that is not there is absent
/// from the body and not null.
#[tokio::test]
async fn a_definition_is_read_step_by_step_with_its_words_and_what_ends_each_step() {
    let (_home, _planted, _fleet, app) = holding(staged_in_repository()).await;

    let (status, body) = steps(&app, "workflow_id=staged").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let read = decoded(&body);
    assert_eq!(read.workflow_id.as_str(), "staged");
    assert_eq!(read.source, "repository");
    let ids: Vec<&str> = read.steps.iter().map(|step| step.id.as_str()).collect();
    assert_eq!(ids, ["propose", "plan", "walk", "deliver"]);
    let ends: Vec<Option<&str>> = read.steps.iter().map(|step| step.ends_when.as_deref()).collect();
    assert_eq!(ends, [Some("you reply"), Some("you approve the plan"), Some("you approve the walk"), Some("it is merged")]);
    assert_eq!(read.steps[0].label, "Propose");
    assert_eq!(read.steps[0].instructions.as_deref(), Some("Say what you took the request to mean."));
    assert_eq!(read.steps[1].instructions, None, "a step with no words has none");
    assert_eq!(read.steps[2].passed_over_when.as_deref(), Some("nothing would be seen"));
    assert!(read.steps.iter().filter(|step| step.id.as_str() != "walk").all(|step| step.passed_over_when.is_none()));
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("null"), "absent rather than null: {text}");
}

/// **Armada's definition is read beside the copy that replaces it**, so the page can compare; a place
/// that holds none by that id is a 422.
#[tokio::test]
async fn armadas_definition_is_read_beside_the_copy_that_replaces_it() {
    let carried = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("feature is carried");
    let (_home, _planted, _fleet, app) = holding(vec![config::Written::in_repository(
        at("repo", "feature.json"),
        carried.text().replace("\"label\": \"Propose\"", "\"label\": \"Mine\""),
    )])
    .await;

    let (_, running) = steps(&app, "workflow_id=feature").await;
    let (status, armadas) = steps(&app, "workflow_id=feature&source=armada").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&armadas));
    let (running, armadas) = (decoded(&running), decoded(&armadas));
    assert_eq!((running.source.as_str(), armadas.source.as_str()), ("repository", "armada"));
    assert_eq!(running.steps[0].label, "Mine");
    assert_eq!(armadas.steps[0].label, "Propose");
    assert_eq!(armadas.steps.len(), running.steps.len());

    let (status, body) = steps(&app, "workflow_id=feature&source=kit").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.no_such_workflow_definition");
    let (status, body) = steps(&app, "workflow_id=nope").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.no_such_workflow_definition");
}

/// **A step lists the nudges that still count, in the place's own definition**, newest first, each with
/// the Job and its title. One sent from Armada's `staged` is not on the repository's.
#[tokio::test]
async fn a_step_lists_the_nudges_that_still_count_with_the_job_and_its_title() {
    let (_home, _planted, fleet, app) = holding(staged_in_repository()).await;
    titled(&fleet, "job-a", "Cockpit timeline rail").await;
    nudged(&fleet, "staged", "repository", "walk", "job-a", "a test before the walk was approved", "2026-10-27T10:00:00Z").await;
    nudged(&fleet, "staged", "repository", "walk", "job-b", "a test file written with the mock", "2026-10-27T11:00:00Z").await;
    nudged(&fleet, "staged", "armada", "walk", "job-c", "a test before the walk was approved", "2026-10-27T12:00:00Z").await;
    nudged(&fleet, "staged", "repository", "plan", "job-a", "edited before the plan was approved", "2026-10-27T09:30:00Z").await;

    let (_, body) = steps(&app, "workflow_id=staged").await;

    let read = decoded(&body);
    let walk = &read.steps[2];
    let jobs: Vec<&str> = walk.nudges.iter().map(|one| one.job_id.as_str()).collect();
    assert_eq!(jobs, ["job-b", "job-a"], "newest first, and not Armada's");
    assert_eq!(walk.nudges[1].job_title.as_deref(), Some("Cockpit timeline rail"));
    assert_eq!(walk.nudges[0].job_title, None, "a Job Fleet holds no title for has none");
    assert_eq!(walk.nudges[1].fact, "a test before the walk was approved");
    assert_eq!(walk.nudges[1].at.as_str(), "2026-10-27T10:00:00Z");
    assert_eq!(read.steps[1].nudges.len(), 1);
    assert!(read.steps[0].nudges.is_empty() && read.steps[3].nudges.is_empty());
}

/// **Not a drift keeps the nudge and stops it counting.** Marking one twice answers the same and keeps
/// the first time; one Fleet never kept is a 422.
#[tokio::test]
async fn a_nudge_marked_not_a_drift_stops_counting_and_stays_kept() {
    let (_home, _planted, fleet, app) = holding(staged_in_repository()).await;
    nudged(&fleet, "staged", "repository", "walk", "job-a", "a test before the walk was approved", "2026-10-27T10:00:00Z").await;
    nudged(&fleet, "staged", "repository", "walk", "job-b", "a test file written with the mock", "2026-10-27T11:00:00Z").await;
    let (_, body) = steps(&app, "workflow_id=staged").await;
    let first = decoded(&body).steps[2].nudges[1].id;

    let (status, body) = call(&app, "POST", "/workflows/not_a_drift", &format!(r#"{{"nudge":{first}}}"#)).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let marked: ipc::NotADriftMarked = ipc::decode("marked", &body).expect("an answer");
    assert_eq!(marked.nudge, first);
    let (_, body) = steps(&app, "workflow_id=staged").await;
    assert_eq!(decoded(&body).steps[2].nudges.len(), 1, "one still counts");
    let kept = fleet.store().lock().await.workflow_nudges("staged", "repository", "walk", 20).expect("read");
    let marked_row = kept.iter().find(|one| one.id == first).expect("the row is kept");
    let first_time = marked_row.dismissed_at.clone().expect("marked");
    assert_eq!(kept.len(), 2);

    let (again, _) = call(&app, "POST", "/workflows/not_a_drift", &format!(r#"{{"nudge":{first}}}"#)).await;
    assert_eq!(again, StatusCode::OK);
    let kept = fleet.store().lock().await.workflow_nudges("staged", "repository", "walk", 20).expect("read");
    assert_eq!(kept.iter().find(|one| one.id == first).and_then(|one| one.dismissed_at.clone()), Some(first_time));

    let (status, body) = call(&app, "POST", "/workflows/not_a_drift", r#"{"nudge":9999}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.no_such_nudge");
}

/// **A deleted copy is gone from what runs before the answer**, and the answer says whose runs now.
#[tokio::test]
async fn a_copy_is_deleted_and_what_runs_is_read_again() {
    let (_home, planted, _fleet, app) = holding(staged_in_repository()).await;
    let feature = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("carried");
    planted.will_remove(Ok("/repo/.armada/workflows/staged.json".to_string()));
    planted.will_read(read(vec![config::Written::in_repository(at("repo", "feature.json"), feature.text().to_string())]));

    let (status, body) = call(&app, "POST", "/workflows/delete", r#"{"workflow_id":"staged","source":"repository"}"#).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let deleted: ipc::WorkflowCopyDeleted = ipc::decode("deleted", &body).expect("an answer");
    assert_eq!(deleted.file, "/repo/.armada/workflows/staged.json");
    assert_eq!(deleted.source, "repository");
    assert_eq!(deleted.runs_from, None, "nothing else defines `staged`");
    assert_eq!(planted.removals(), [(ipc::WorkflowScope::Repository, "staged".to_string())]);
    let (status, _) = steps(&app, "workflow_id=staged").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "it was read again before the answer");
}

/// **Where Armada has the id, deleting a copy leaves Armada's running**, and the answer says so.
#[tokio::test]
async fn deleting_a_copy_of_one_armada_carries_says_armadas_runs() {
    let carried = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("carried");
    let copy = vec![config::Written::in_kit(at("kit", "feature.json"), carried.text().to_string())];
    let (_home, planted, _fleet, app) = holding(copy).await;
    planted.will_remove(Ok("/kit/workflows/feature.json".to_string()));
    planted.will_read(read(Vec::new()));

    let (status, body) = call(&app, "POST", "/workflows/delete", r#"{"workflow_id":"feature","source":"kit"}"#).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let deleted: ipc::WorkflowCopyDeleted = ipc::decode("deleted", &body).expect("an answer");
    assert_eq!(deleted.runs_from.as_deref(), Some("armada"));
    assert_eq!(planted.removals(), [(ipc::WorkflowScope::Kit, "feature".to_string())]);
}

/// **Armada's own is refused by both acts, and nothing is touched.**
#[tokio::test]
async fn armadas_own_is_neither_deleted_nor_reset() {
    let (_home, planted, _fleet, app) = holding(staged_in_repository()).await;
    for route in ["/workflows/delete", "/workflows/reset"] {
        let (status, body) = call(&app, "POST", route, r#"{"workflow_id":"feature","source":"armada"}"#).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{route}");
        assert_eq!(code(&body), "fleet.workflow_is_armadas", "{route}");
        assert!(String::from_utf8_lossy(&body).contains("carried in the binary"), "{route}");
    }
    assert!(planted.removals().is_empty());
    assert!(planted.saves().is_empty());
}

/// **A copy this repository does not hold is refused by both acts**: no such id, no such place, and no
/// place Fleet writes to.
#[tokio::test]
async fn a_copy_that_is_not_held_is_refused_and_nothing_is_touched() {
    let (_home, planted, _fleet, app) = holding(staged_in_repository()).await;
    for route in ["/workflows/delete", "/workflows/reset"] {
        for body in [
            r#"{"workflow_id":"staged","source":"kit"}"#,
            r#"{"workflow_id":"nope","source":"repository"}"#,
            r#"{"workflow_id":"staged","source":"elsewhere"}"#,
        ] {
            let (status, answer) = call(&app, "POST", route, body).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{route} {body}");
            assert_eq!(code(&answer), "fleet.no_such_workflow_definition", "{route} {body}");
        }
    }
    assert!(planted.removals().is_empty());
    assert!(planted.saves().is_empty());
}

/// **A file that cannot be deleted is a 500 naming it.**
#[tokio::test]
async fn a_file_that_cannot_be_deleted_is_a_500_naming_it() {
    let (_home, planted, _fleet, app) = holding(staged_in_repository()).await;
    planted.will_remove(Err(WorkflowNotRemoved::Unwritable {
        file: "/repo/.armada/workflows/staged.json".to_string(),
        cause: std::io::Error::other("read-only"),
    }));

    let (status, body) = call(&app, "POST", "/workflows/delete", r#"{"workflow_id":"staged","source":"repository"}"#).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(code(&body), "fleet.workflow_unwritable");
    assert!(String::from_utf8_lossy(&body).contains("/repo/.armada/workflows/staged.json"));
}

/// **A reset writes Armada's own text over the copy's file, replacing it**, in the copy's place.
#[tokio::test]
async fn a_copy_is_reset_with_armadas_text_replacing_it() {
    let carried = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("carried");
    let edited = carried.text().replace("\"label\": \"Propose\"", "\"label\": \"Mine\"");
    let (_home, planted, _fleet, app) = holding(vec![config::Written::in_kit(at("kit", "feature.json"), edited)]).await;
    planted.will_save(Ok(SavedWorkflow {
        id: carried_id("feature"),
        file: "/kit/workflows/feature.json".to_string(),
        replaced: true,
    }));
    planted.will_read(read(vec![config::Written::in_kit(at("kit", "feature.json"), carried.text().to_string())]));

    let (status, body) = call(&app, "POST", "/workflows/reset", r#"{"workflow_id":"feature","source":"kit"}"#).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let saved: ipc::WorkflowSaved = ipc::decode("saved", &body).expect("an answer");
    assert!(saved.replaced);
    assert_eq!(saved.scope, ipc::WorkflowScope::Kit);
    assert_eq!(saved.runs_from.as_deref(), Some("kit"));
    assert_eq!(
        planted.saves(),
        [ipc::SaveWorkflow { scope: ipc::WorkflowScope::Kit, definition: carried.text().to_string(), overwrite: true }]
    );
}

fn carried_id(id: &str) -> core_model::WorkflowId {
    core_model::WorkflowId::carried(core_model::Ulid::carried(id))
}

/// **A copy of something Armada does not carry has nothing to be reset to.**
#[tokio::test]
async fn a_copy_of_something_armada_does_not_carry_is_not_reset() {
    let (_home, planted, _fleet, app) = holding(staged_in_repository()).await;

    let (status, body) = call(&app, "POST", "/workflows/reset", r#"{"workflow_id":"staged","source":"repository"}"#).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.workflow_not_carried");
    assert!(String::from_utf8_lossy(&body).contains("Armada carries no `staged`"));
    assert!(planted.saves().is_empty());
}

/// **A reset is refused with the loader's own sentence where Armada's definition does not fit here**,
/// and nothing is written.
#[tokio::test]
async fn a_reset_armadas_definition_does_not_fit_is_refused_with_the_loaders_sentence() {
    let carried = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("carried");
    let (_home, planted, _fleet, app) =
        holding(vec![config::Written::in_kit(at("kit", "feature.json"), carried.text().to_string())]).await;
    planted.will_save(Err(WorkflowNotSaved::Unfit { why: "step `build` needs `lint`, and the declared Checks are `test`".to_string() }));

    let (status, body) = call(&app, "POST", "/workflows/reset", r#"{"workflow_id":"feature","source":"kit"}"#).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.workflow_unfit");
    assert!(String::from_utf8_lossy(&body).contains("needs `lint`"));
}
