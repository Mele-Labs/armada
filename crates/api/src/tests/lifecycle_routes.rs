//! Workspaces, lifecycles and rules over the router: each read reaches the daemon with the repository it
//! names, a Job's is placed by its `session_id`, and what answers is what the mirror declares.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{
    InstanceStanding, JobLifecycles, LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved, PieceScope,
    RepositoryAutonomy, RepositoryLifecycleRules, RepositoryLifecycles, RepositoryWorkspaces, WireError,
};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, THE_KEPT_PIECE, THE_LIFECYCLE_JOB};
use crate::tests::shapes::{run_id, THE_MANIFEST};
use crate::{offered, router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder().method("GET").uri(uri).body(Body::empty()).expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn each_read_of_a_repository_names_it_by_manifest_id() {
    let app = wired();
    for (route, what) in [("/workspaces", "workspaces"), ("/lifecycles", "lifecycles"), ("/lifecycles/rules", "rules")] {
        let (status, bytes) = get(&app, &format!("{route}?manifest_id={THE_MANIFEST}")).await;
        assert_eq!(status, StatusCode::OK, "{route}: {}", String::from_utf8_lossy(&bytes));
        let said = String::from_utf8_lossy(&bytes);
        assert!(said.contains(&format!(r#""manifest_id":"{THE_MANIFEST}""#)), "{said}");
        assert!(said.contains(&format!(r#""{what}":[]"#)) && said.contains(r#""left_out":[]"#), "{said}");
    }
    let (_, bytes) = get(&app, "/workspaces").await;
    ipc::decode::<RepositoryWorkspaces>("workspaces", &bytes).expect("reads");
    let (_, bytes) = get(&app, "/lifecycles").await;
    ipc::decode::<RepositoryLifecycles>("lifecycles", &bytes).expect("reads");
    let (_, bytes) = get(&app, "/lifecycles/rules").await;
    ipc::decode::<RepositoryLifecycleRules>("rules", &bytes).expect("reads");
    let (status, bytes) = get(&app, &format!("/autonomy?manifest_id={THE_MANIFEST}")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let read: RepositoryAutonomy = ipc::decode("autonomy", &bytes).expect("reads");
    assert_eq!(read.manifest_id.as_str(), THE_MANIFEST);
}

#[tokio::test]
async fn a_repository_nobody_serves_is_the_callers_to_correct() {
    let app = wired();
    for route in ["/workspaces", "/lifecycles", "/lifecycles/rules", "/autonomy"] {
        let (status, _) = get(&app, &format!("{route}?manifest_id=01NOWHERE")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{route}");
    }
}

#[tokio::test]
async fn a_jobs_lifecycles_are_its_own_and_a_job_that_holds_nothing_answers_empty_lists() {
    let app = wired();
    let (status, bytes) = get(&app, &format!("/sessions/lifecycles?session_id={THE_LIFECYCLE_JOB}")).await;
    assert_eq!(status, StatusCode::OK);
    let held: JobLifecycles = ipc::decode("a Job's lifecycles", &bytes).expect("reads");
    assert_eq!(held.instances[0].label, "PR #812");
    assert_eq!(held.instances[0].standing, InstanceStanding::In { state: "in_review".to_string() });
    assert_eq!(held.moves.len(), 1);

    let (status, bytes) = get(&app, "/sessions/lifecycles?session_id=an-older-job").await;
    assert_eq!(status, StatusCode::OK);
    let held: JobLifecycles = ipc::decode("a Job's lifecycles", &bytes).expect("reads");
    assert!(held.lifecycles.is_empty() && held.instances.is_empty() && held.moves.is_empty());
}

#[tokio::test]
async fn nothing_placing_a_job_is_the_callers_to_correct() {
    let (status, bytes) = get(&wired(), "/sessions/lifecycles").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&bytes).contains("fleet.job_lifecycles_unplaced"));
}

/// The agent reads its own Job's lifecycles, and nothing else of them: the repository reads are Bridge's.
#[test]
fn the_door_offers_the_jobs_read_and_none_of_the_repository_reads() {
    let tools: Vec<_> = offered().into_iter().map(|shape| (shape.operation, shape.path, shape.query)).collect();
    assert!(tools.contains(&("get_job_lifecycles", "/sessions/lifecycles", &["session_id"][..])), "{tools:?}");
    for bridge_only in ["list_repository_workspaces", "list_lifecycles", "list_lifecycle_rules", "list_autonomy_defaults"] {
        assert!(tools.iter().all(|(operation, ..)| *operation != bridge_only), "{bridge_only} reached the agent's tool set");
    }
}

async fn post(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

fn workspace_to_save(id: &str) -> String {
    format!(r#"{{"piece":{{"kind":"workspace","workspace":{{"id":"{id}","name":"My store","repositories":["01ABC"]}}}}}}"#)
}

#[tokio::test]
async fn a_piece_saved_reaches_the_daemon_whole_and_is_answered_with_the_file_written() {
    let (status, bytes) = post(&wired(), "/lifecycles/pieces/save", &workspace_to_save("my-store")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let saved: LifecyclePieceSaved = ipc::decode("a save", &bytes).expect("reads");
    assert_eq!((saved.kind, saved.id.as_str(), saved.scope, saved.replaced), (LifecyclePieceKind::Workspace, "my-store", PieceScope::Everywhere, false));
    assert_eq!(saved.file, "/kit/my-store.yml");
}

#[tokio::test]
async fn replacing_a_piece_is_refused_unless_the_save_says_overwrite() {
    let app = wired();
    let (status, bytes) = post(&app, "/lifecycles/pieces/save", &workspace_to_save(THE_KEPT_PIECE)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: WireError = ipc::decode("an error", &bytes).expect("an error body");
    assert_eq!(error.code, "fleet.piece_exists");

    let said = format!(r#"{{"overwrite":true,{}"#, &workspace_to_save(THE_KEPT_PIECE)[1..]);
    let (status, bytes) = post(&app, "/lifecycles/pieces/save", &said).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let saved: LifecyclePieceSaved = ipc::decode("a save", &bytes).expect("reads");
    assert!(saved.replaced);
}

#[tokio::test]
async fn a_piece_deleted_is_named_by_kind_id_and_scope_and_one_kit_does_not_hold_is_the_callers_to_correct() {
    let app = wired();
    let said = format!(r#"{{"kind":"rule","id":"{THE_KEPT_PIECE}","scope":{{"kind":"within","id":"my-store"}}}}"#);
    let (status, bytes) = post(&app, "/lifecycles/pieces/delete", &said).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let deleted: LifecyclePieceDeleted = ipc::decode("a delete", &bytes).expect("reads");
    assert_eq!((deleted.kind, deleted.scope), (LifecyclePieceKind::Rule, PieceScope::Within { id: "my-store".to_string() }));

    let (status, _) = post(&app, "/lifecycles/pieces/delete", r#"{"kind":"rule","id":"absent"}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_piece_that_is_not_a_known_kind_is_the_callers_fault_and_says_so() {
    let (status, bytes) = post(&wired(), "/lifecycles/pieces/save", r#"{"piece":{"kind":"template"}}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let error: WireError = ipc::decode("an error", &bytes).expect("an error body");
    assert_eq!(error.code, "api.undecodable_request");
}

/// The pieces are Bridge's to write: the agent's tool set has neither.
#[test]
fn the_door_offers_neither_the_save_nor_the_delete() {
    let tools: Vec<_> = offered().into_iter().map(|shape| shape.operation).collect();
    for bridge_only in ["save_lifecycle_piece", "delete_lifecycle_piece"] {
        assert!(!tools.contains(&bridge_only), "{bridge_only} reached the agent's tool set");
    }
}
