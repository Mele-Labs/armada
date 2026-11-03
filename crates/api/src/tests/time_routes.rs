//! The time reads over the router and at the agent's door: each path reaches the daemon with the window it
//! names, a Job may call its own split and a window's sums, and the Usage page's hours are Bridge's alone.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobTime, UsageHours, UsageTime};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{offered, router, Broadcaster, Served, DOOR_PATH};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    (status, response.into_body().collect().await.expect("a body").to_bytes().to_vec())
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    send(app, Request::builder().method("GET").uri(uri).body(Body::empty()).expect("a request")).await
}

#[tokio::test]
async fn each_time_read_reaches_the_daemon_with_the_window_it_names() {
    let app = wired();

    let (status, bytes) = get(&app, "/sessions/time?session_id=s1").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let job: JobTime = ipc::decode("a job's time", &bytes).expect("reads");
    assert_eq!((job.session_id.as_str(), job.time.is_none()), ("s1", true));

    let (status, bytes) = get(&app, "/usage/time?since=2026-11-01T00:00:00Z&until=2026-11-03T00:00:00Z&slowest=3").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let time: UsageTime = ipc::decode("a window's time", &bytes).expect("reads");
    assert_eq!(time.since, "2026-11-01T00:00:00Z|2026-11-03T00:00:00Z|3");

    let (status, bytes) = get(&app, "/usage/hours?since=2026-11-03T10:00:00Z&hours=5").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let hours: UsageHours = ipc::decode("hours", &bytes).expect("reads");
    assert_eq!(hours.since, "2026-11-03T10:00:00Z|5");

    let (_, bytes) = get(&app, "/usage/hours").await;
    assert_eq!(ipc::decode::<UsageHours>("hours", &bytes).expect("reads").since, "|", "every bound is optional");
    let (status, _) = get(&app, "/usage/time?slowest=many").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a count that is no number is refused before the daemon");
}

#[test]
fn a_job_may_call_its_own_split_and_a_windows_sums_but_not_the_hours() {
    let shape = |operation: &str| offered().into_iter().find(|shape| shape.operation == operation);
    let job = shape("get_job_time").expect("offered");
    assert_eq!((job.method, job.path, job.query), ("GET", "/sessions/time", &["session_id"][..]));
    let window = shape("get_usage_time").expect("offered");
    assert_eq!((window.method, window.path, window.query), ("GET", "/usage/time", &["since", "until", "slowest"][..]));
    assert!(shape("get_usage_hours").is_none(), "the page's hours are Bridge's");
}

/// What `armada job` lists and `--help` prints is read off this list.
#[tokio::test]
async fn armada_job_lists_both_acts_with_a_line_saying_what_each_answers() {
    let app = wired();
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(ipc::job_cli::list_message()))
        .expect("a request");
    let (status, bytes) = send(&app, request).await;
    assert_eq!(status, StatusCode::OK);
    let acts = ipc::job_cli::acts(&bytes).expect("the list reads");

    let job = acts.iter().find(|act| act.name == "get_job_time").expect("listed");
    assert_eq!(job.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["session_id"]);
    assert!(job.description.starts_with("Where this Job's time went"), "{}", job.description);
    let window = acts.iter().find(|act| act.name == "get_usage_time").expect("listed");
    assert_eq!(window.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["since", "slowest", "until"], "the door lists a tool's fields in name order");
    assert!(window.description.starts_with("Where the time of the Jobs that ended in a window went"), "{}", window.description);
    assert!(acts.iter().all(|act| act.name != "get_usage_hours"));
}
