//! `get_flight_controls`, `save_flight_controls`, a repository's own pair, `commit_to_repository`
//! and `run_session_shell` over the router: the routes exist, a body reaches the daemon whole, one
//! with a key nobody knows is a 400, and a repository Fleet does not serve is a 404.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{FlightControlsRead, FlightControls, RepositoryFlightControls};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, SERVED_ROOT};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn the_controls_are_read_and_a_save_reaches_the_daemon_whole() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/kit/flight_controls", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: FlightControlsRead = ipc::decode("flight controls", &body).expect("decodes");
    assert_eq!(read, FlightControlsRead::Read { controls: FlightControls::default() });

    let sent = r#"{"version":1,"job":[{"id":"ask","name":"Update","key":"1","hidden":true},
        {"id":"pull","kind":"shell","text":"git pull"}],"deck":[{"id":"dispatch"}]}"#;
    let (status, body) = call(&app, "POST", "/kit/flight_controls", sent).await;
    assert_eq!(status, StatusCode::OK);
    let FlightControlsRead::Read { controls } = ipc::decode("saved", &body).expect("decodes") else {
        panic!("the fake answers what it was sent");
    };
    assert_eq!(controls.job[0].name.as_deref(), Some("Update"));
    assert!(controls.job[0].hidden);
    assert_eq!(controls.job[1].kind, Some(ipc::FlightControlKind::Shell));
    assert_eq!(controls.deck[0].id, "dispatch");
}

#[tokio::test]
async fn a_key_nobody_knows_is_a_400() {
    let app = wired();
    let (status, _) = call(&app, "POST", "/kit/flight_controls", r#"{"version":1,"job":[{"id":"a","hiden":true}]}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_shell_action_is_routed_to_the_daemon() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/sessions/shell", r#"{"session_id":"s1","command":"true"}"#).await;
    // The fake runs nothing and says so, which only a routed request can.
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", String::from_utf8_lossy(&body));
    let (status, _) = call(&app, "POST", "/sessions/shell", r#"{"command":"true"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_repositorys_controls_are_read_by_root_and_say_where_they_were_read() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/repositories/flight_controls?repository=/work/shop", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: RepositoryFlightControls = ipc::decode("a repository's flight controls", &body).expect("decodes");
    assert_eq!(answered.repository, SERVED_ROOT);
    assert_eq!(answered.place, ipc::RunPlace::ThisMac);
    assert_eq!(answered.read, FlightControlsRead::Read { controls: FlightControls::default() });
    let wire = String::from_utf8_lossy(&body);
    assert!(wire.contains(r#""place":"this_mac""#) && wire.contains(r#""read":{"what":"read""#), "{wire}");

    let (status, _) = call(&app, "GET", "/repositories/flight_controls", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a read names its repository");

    let (status, body) = call(&app, "GET", "/repositories/flight_controls?repository=/work/shop&session_id=s1", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: RepositoryFlightControls = ipc::decode("read in a Job", &body).expect("decodes");
    assert_eq!(answered.in_repository, FlightControlsRead::NotRead { why: String::from("read in s1") }, "the fake was handed the Job");
}

#[tokio::test]
async fn a_commit_to_the_repository_reaches_the_daemon_whole_for_each_kind() {
    let app = wired();
    for (piece, path) in [
        (r#"{"kind":"flight_controls","job":["specs"]}"#, ".armada/flight-controls.toml"),
        (r#"{"kind":"workflow","workflow_id":"quick"}"#, ".armada/workflows/quick"),
        (r#"{"kind":"review_tool","id":"security"}"#, ".armada/review-tools/security.md"),
    ] {
        let sent = format!(r#"{{"repository":"/work/shop","piece":{piece},"overwrite":true}}"#);
        let (status, body) = call(&app, "POST", "/repositories/commit", &sent).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let answered: ipc::CommittedToRepository = ipc::decode("committed", &body).expect("decodes");
        assert_eq!((answered.session_id.as_str(), answered.paths.as_slice()), ("s1", [path.to_string()].as_slice()));
        assert_eq!(answered.pull_request_not_opened.as_deref(), Some("No pull request was opened."));
    }

    let (status, body) = call(&app, "POST", "/repositories/commit", r#"{"repository":"/work/shop","piece":{"kind":"review_tool","id":"security"}}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "overwrite arrives false where absent");
    assert!(String::from_utf8_lossy(&body).contains("fleet.commit_exists"));

    let (status, _) = call(&app, "POST", "/repositories/commit", r#"{"repository":"/work/shop","session_id":"s1","piece":{"kind":"review_tool","id":"x"}}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a Job is no longer named");

    let (status, _) = call(&app, "POST", "/repositories/commit", r#"{"repository":"/work/shop","piece":{"kind":"armada_yml"}}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a kind nobody knows");

    let (status, body) = call(&app, "POST", "/repositories/commit", r#"{"repository":"/not/served","piece":{"kind":"review_tool","id":"x"}}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.run_memory_repository_not_served"));
}

#[tokio::test]
async fn a_repositorys_save_reaches_the_daemon_whole() {
    let app = wired();
    let sent = r#"{"repository":"/work/shop","controls":{"version":1,"job":[{"id":"specs","kind":"shell","text":"bin/rspec","key":"r"}],"deck":[]}}"#;
    let (status, body) = call(&app, "POST", "/repositories/flight_controls", sent).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: RepositoryFlightControls = ipc::decode("saved", &body).expect("decodes");
    let FlightControlsRead::Read { controls } = answered.read else { panic!("the fake answers what it was sent") };
    assert_eq!(controls.job[0].kind, Some(ipc::FlightControlKind::Shell));
    assert_eq!(controls.job[0].key.as_deref(), Some("r"));

    let (status, _) = call(&app, "POST", "/repositories/flight_controls", r#"{"repository":"/work/shop","controls":{"version":1},"place":"cloud"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a key nobody knows on the body");
}

#[tokio::test]
async fn a_repository_fleet_does_not_serve_is_a_404_on_both() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/repositories/flight_controls?repository=/not/served", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.run_memory_repository_not_served"));

    let sent = r#"{"repository":"/not/served","controls":{"version":1,"job":[],"deck":[]}}"#;
    let (status, body) = call(&app, "POST", "/repositories/flight_controls", sent).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.run_memory_repository_not_served"));
}

#[tokio::test]
async fn approving_a_repository_tile_reaches_the_daemon_whole() {
    let app = wired();
    let sent = r#"{"repository":"/work/shop","tile_id":"specs","command_hash":"abc"}"#;
    let (status, body) = call(&app, "POST", "/repositories/flight_controls/approve", sent).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: RepositoryFlightControls = ipc::decode("approved", &body).expect("decodes");
    assert_eq!(answered.in_repository, FlightControlsRead::NotRead { why: String::from("approved specs as abc") });

    let (status, _) = call(&app, "POST", "/repositories/flight_controls/approve", r#"{"repository":"/work/shop","tile_id":"specs"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "an approval names the hash it was shown");

    let (status, _) = call(&app, "POST", "/repositories/flight_controls/approve", r#"{"repository":"/not/served","tile_id":"specs","command_hash":"abc"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn keeping_a_kit_manifest_reaches_the_daemon_whole() {
    let app = wired();
    let sent = r#"{"repository":"/work/shop","text":"checks:\n  test:\n    run: make test\n"}"#;
    let (status, body) = call(&app, "POST", "/repositories/kit_manifest", sent).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: ipc::KitManifestSaved = ipc::decode("saved", &body).expect("decodes");
    assert_eq!((answered.repository.as_str(), answered.file.as_str()), ("/work/shop", "35 bytes"));

    let (status, _) = call(&app, "POST", "/repositories/kit_manifest", r#"{"repository":"/work/shop"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "the text is required");

    let (status, _) = call(&app, "POST", "/repositories/kit_manifest", r#"{"repository":"/not/served","text":"x: 1"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
