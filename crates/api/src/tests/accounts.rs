//! The account's four routes over the router: signing in answers the code, the
//! profile list reads back, an account this machine has not joined is a 422
//! by name, and a body that names none is a 400 the daemon never sees.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{AccountFlow, Accounts, SignedOut, WireError};
use tower::ServiceExt;

use crate::tests::fake::{profile, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired(profiles: Vec<ipc::Profile>) -> Router {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    daemon.accounts.lock().expect("not poisoned").profiles = profiles;
    router(Served::by(daemon, run_id(), events))
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
async fn signing_in_answers_the_code_and_the_list_then_shows_it_in_flight() {
    let app = wired(Vec::new());
    let (status, body) = call(&app, "GET", "/accounts", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: Accounts = ipc::decode("accounts", &body).expect("decodes");
    assert!(read.profiles.is_empty());
    assert_eq!(read.flow, AccountFlow::Idle { ended: None });

    let (status, body) = call(&app, "POST", "/accounts/start_sign_in", "").await;
    assert_eq!(status, StatusCode::OK);
    let begun: Accounts = ipc::decode("accounts", &body).expect("decodes");
    assert!(matches!(begun.flow, AccountFlow::Code { ref code, .. } if code == "WDJB-MJHT"));

    let (_, body) = call(&app, "GET", "/accounts", "").await;
    let again: Accounts = ipc::decode("accounts", &body).expect("decodes");
    assert_eq!(again, begun);
}

#[tokio::test]
async fn choosing_a_profile_makes_it_the_one_that_is_active() {
    let app = wired(vec![profile(1, "work", true, false), profile(2, "home", false, true)]);
    let (status, body) = call(&app, "POST", "/accounts/set_active_account", r#"{"account_id":2}"#).await;
    assert_eq!(status, StatusCode::OK);
    let now: Accounts = ipc::decode("accounts", &body).expect("decodes");
    assert_eq!(now.profiles.iter().map(|one| one.active).collect::<Vec<_>>(), [false, true]);
}

#[tokio::test]
async fn signing_out_leaves_the_others_and_says_whether_the_store_row_remains() {
    let app = wired(vec![profile(1, "work", true, false), profile(2, "home", false, true)]);
    let (status, body) = call(&app, "POST", "/accounts/sign_out", r#"{"account_id":1}"#).await;
    assert_eq!(status, StatusCode::OK);
    let out: SignedOut = ipc::decode("a sign-out", &body).expect("decodes");
    assert_eq!(out.accounts.profiles.len(), 1);
    assert_eq!(out.accounts.profiles[0].login, "home");
    assert!(out.accounts.profiles[0].active, "the profile left is the one that acts");
    assert!(!out.store_row_remains);
}

#[tokio::test]
async fn an_account_the_machine_has_not_joined_is_a_422_by_name_and_a_body_with_no_account_a_400() {
    let app = wired(vec![profile(1, "work", true, false)]);
    for uri in ["/accounts/sign_out", "/accounts/set_active_account"] {
        let (status, body) = call(&app, "POST", uri, r#"{"account_id":9}"#).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
        let refused: WireError = ipc::decode("a refusal", &body).expect("decodes");
        assert_eq!(refused.code, "fleet.no_such_account");

        let (status, _) = call(&app, "POST", uri, r#"{"login":"work"}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
    }
}
