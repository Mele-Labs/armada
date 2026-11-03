//! A Job's terminals over the router: each body and query reaches the daemon whole, with the
//! terminal it names, and the answers name theirs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
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
async fn an_open_reaches_the_daemon_and_answers_the_shells_run_id() {
    let app = wired();
    let (status, bytes) = call(&app, "POST", "/sessions/terminal/open", r#"{"session_id":"job-7","cols":120,"rows":40}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let open: ipc::SessionRunUnderway = ipc::decode("a shell", &bytes).expect("a run id");
    assert_eq!(open.run_id, "shell-of-job-7");

    let (status, _) = call(&app, "POST", "/sessions/terminal/open", r#"{"cols":120}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a Job is named");
}

async fn answered<T: serde::de::DeserializeOwned>(app: &Router, method: &str, uri: &str, body: &str, what: &'static str) -> T {
    let (status, bytes) = call(app, method, uri, body).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    ipc::decode(what, &bytes).expect(what)
}

#[tokio::test]
async fn an_open_carries_the_terminal_it_names_or_asks_for_a_new_one() {
    let app = wired();
    let open = |body: &'static str| {
        let app = app.clone();
        async move { answered::<ipc::SessionRunUnderway>(&app, "POST", "/sessions/terminal/open", body, "a shell").await }
    };
    assert_eq!(open(r#"{"session_id":"job-7"}"#).await.terminal.as_deref(), Some("t1"));
    assert_eq!(open(r#"{"session_id":"job-7","terminal":"t3"}"#).await.terminal.as_deref(), Some("t3"));
    assert_eq!(open(r#"{"session_id":"job-7","new":true}"#).await.terminal.as_deref(), Some("t2"));
}

#[tokio::test]
async fn typing_carries_the_terminal_it_names_or_asks_for_a_new_one() {
    let app = wired();
    let type_in = |body: &'static str| {
        let app = app.clone();
        async move { answered::<ipc::TerminalTyped>(&app, "POST", "/sessions/terminal/type", body, "what was printed").await }
    };
    let named = type_in(r#"{"text":"ls","terminal":"t4"}"#).await;
    assert_eq!((named.terminal.as_str(), named.printed.as_str()), ("t4", "ls"));
    assert_eq!(type_in(r#"{"text":"ls","new":true}"#).await.terminal, "t2");
    assert_eq!(type_in(r#"{"text":"ls"}"#).await.terminal, "t1");
}

#[tokio::test]
async fn a_read_carries_the_terminal_in_its_query() {
    let app = wired();
    let read = |uri: &'static str| {
        let app = app.clone();
        async move { answered::<ipc::TerminalScreen>(&app, "GET", uri, "", "a screen").await }
    };
    assert_eq!(read("/sessions/terminal/read?terminal=t3&lines=5").await.terminal, "t3");
    assert_eq!(read("/sessions/terminal/read?lines=5").await.terminal, "t1");
}

#[tokio::test]
async fn the_list_answers_each_open_terminal_with_its_name_and_whether_a_program_runs_in_it() {
    let app = wired();
    for uri in ["/sessions/terminal/list", "/sessions/terminal/list?session_id=job-7", "/sessions/terminal/list?session_id="] {
        let listed: ipc::JobTerminals = answered(&app, "GET", uri, "", "the terminals").await;
        let rows: Vec<(&str, &str, bool)> =
            listed.terminals.iter().map(|row| (row.terminal.as_str(), row.name.as_str(), row.running)).collect();
        assert_eq!(rows, [("t1", "Terminal 1", false), ("t2", "dev server", true)], "{uri}");
    }
}

#[tokio::test]
async fn a_rename_reaches_the_daemon_and_a_blank_name_restores_the_default() {
    let app = wired();
    let named: ipc::ListedTerminal = answered(
        &app,
        "POST",
        "/sessions/terminal/rename",
        r#"{"session_id":"job-7","terminal":"t2","name":"dev server"}"#,
        "a terminal",
    )
    .await;
    assert_eq!((named.terminal.as_str(), named.name.as_str()), ("t2", "dev server"));
    let blank: ipc::ListedTerminal = answered(
        &app,
        "POST",
        "/sessions/terminal/rename",
        r#"{"session_id":"job-7","terminal":"t2","name":"  "}"#,
        "a terminal",
    )
    .await;
    assert_eq!(blank.name, "Terminal 2");

    let (status, _) = call(&app, "POST", "/sessions/terminal/rename", r#"{"session_id":"job-7","terminal":"t2"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a name is given, if blank");
}
