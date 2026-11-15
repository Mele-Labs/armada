//! Asides over the routes: a question answered 202 and `asking`, a blank one refused, the list, and the tell. What an
//! aside reads, answers and sends is Fleet's (`fleet::tests::asides`).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{Aside, AsideList, AsideState, WireError};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn app() -> Router {
    let events = Broadcaster::new();
    router(Served::sharing(Arc::new(FakeDaemon::new(events.clone())), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response.into_body().collect().await.expect("a body").to_bytes().to_vec();
    (status, body)
}

#[tokio::test]
async fn a_side_question_is_accepted_and_answers_the_aside_asking() {
    let (status, body) =
        call(&app(), "POST", "/sessions/aside", r#"{"session_id":"s1","question":"why a mutex?","quote":"lock.lock()"}"#).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", String::from_utf8_lossy(&body));
    let aside: Aside = ipc::decode("an aside", &body).expect("an aside");
    assert_eq!((aside.state, aside.answer.as_str(), aside.quote.as_deref()), (AsideState::Asking, "", Some("lock.lock()")));
    assert_eq!(aside.session_id.as_str(), "s1");
}

#[tokio::test]
async fn a_blank_question_is_refused_and_the_quote_is_optional() {
    let (status, body) = call(&app(), "POST", "/sessions/aside", r#"{"session_id":"s1","question":"  "}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(ipc::decode::<WireError>("a wire error", &body).expect("an error").code, "fleet.aside_blank");
    let (status, _) = call(&app(), "POST", "/sessions/aside", r#"{"session_id":"s1","question":"why?"}"#).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn the_asides_of_a_job_are_listed_by_its_id_and_one_is_told() {
    let (status, body) = call(&app(), "GET", "/sessions/asides?session_id=s1", "").await;
    assert_eq!(status, StatusCode::OK);
    let list: AsideList = ipc::decode("an aside list", &body).expect("a list");
    assert_eq!(list.session_id.as_str(), "s1");

    let (status, body) = call(&app(), "POST", "/sessions/aside/tell", r#"{"session_id":"s1","aside_id":"aside-1"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(ipc::decode::<Aside>("an aside", &body).expect("an aside").told_at.is_some());
}
