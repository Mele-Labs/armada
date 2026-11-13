//! Pocket v2's claim: a paired phone reads a Job's whole thread and replies to it,
//! each call passed to Fleet as it was sent and Fleet's answer passed back as it came.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::RawQuery;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::routing::{get, post};
use axum::Router;

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

/// A path, the caller header Fleet was sent, and the body or query.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// A thread as Fleet answers it, with a field the phone list trims away, so a
/// pass-through that trimmed would show.
const THREAD: &str = r#"{"session":{"id":"h1","harness":"term","origin":"bridge","cwd":"/Users/user/armada","title":"Fix it","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{},"attachments":[]},"rows":[{"kind":"ask","id":"c1","question":"Which branch?"}]}"#;
const ANSWER: &str = r#"{"session_id":"h1","call":"c1","answer":"allow_once","answers":[{"question":"Which branch?","chosen":["main"]}]}"#;

const SESSION: &str = r#"{"id":"h2","harness":"term","origin":"bridge","cwd":"/Users/user/armada","title":"Long one","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{},"attachments":[]}"#;

/// A thread of `count` rows, `r1` oldest, each one a row `ipc` reads.
fn thread_of(count: usize) -> String {
    let rows: Vec<String> = (1..=count)
        .map(|n| format!(r#"{{"kind":"command","id":"r{n}","at":"2026-10-08T01:00:00.000Z","text":"/reload-plugins {n}"}}"#))
        .collect();
    format!(r#"{{"session":{SESSION},"rows":[{}]}}"#, rows.join(","))
}

fn caller(headers: &HeaderMap) -> String {
    headers.get("x-armada-caller").and_then(|v| v.to_str().ok()).unwrap_or("").to_string()
}

pub(crate) async fn stub_fleet(seen: Seen) -> u16 {
    let read = seen.clone();
    let wrote = seen;
    let app = Router::new()
        .route(
            "/sessions/one",
            get(move |headers: HeaderMap, RawQuery(query): RawQuery| async move {
                let query = query.unwrap_or_default();
                read.lock().unwrap().push(("/sessions/one".into(), caller(&headers), query.clone()));
                match query.as_str() {
                    h1 if h1 == "session_id=h1" || h1.starts_with("session_id=h1&") => (StatusCode::OK, THREAD.to_string()),
                    // Fleet is sent the whole query, `last` and `before` too, and answers the whole thread.
                    long if long.starts_with("session_id=long") => (StatusCode::OK, thread_of(5)),
                    wide if wide.starts_with("session_id=wide") => (StatusCode::OK, thread_of(300)),
                    _ => (StatusCode::NOT_FOUND, r#"{"code":"fleet.no_session","message":"No such job."}"#.to_string()),
                }
            }),
        )
        .route(
            "/sessions/shown",
            get(|RawQuery(query): RawQuery| async move {
                match query.as_deref() {
                    Some("session_id=h1&path=%2Fr%2Fshot.png") => (StatusCode::OK, [("content-type", "image/png")], vec![0x89, b'P']),
                    _ => (StatusCode::NOT_FOUND, [("content-type", "application/json")], br#"{"code":"fleet.shown_file_not_shown"}"#.to_vec()),
                }
            }),
        )
        .route(
            "/sessions/kept_styles",
            get(|RawQuery(query): RawQuery| async move {
                match query.as_deref() {
                    Some("hash=0123456789abcdef0123456789abcdef") => (StatusCode::OK, r#"{"sheets":[],"fonts":[]}"#.to_string()),
                    _ => (StatusCode::NOT_FOUND, r#"{"code":"fleet.no_such_kept_styles"}"#.to_string()),
                }
            }),
        )
        .route(
            "/sessions/ask/answer",
            post(move |headers: HeaderMap, body: String| async move {
                wrote.lock().unwrap().push(("/sessions/ask/answer".into(), caller(&headers), body));
                (StatusCode::OK, THREAD.to_string())
            }),
        );
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

pub(crate) struct Phone {
    pub rig: Rig,
    pub device: String,
}

pub(crate) async fn phone_on(port: u16) -> Phone {
    let rig = rig_with(Arc::new(move || Ok(port)));
    let device = paired(&rig).await;
    Phone { rig, device }
}

pub(crate) async fn send(phone: &Phone, method: Method, path: &str, body: &str) -> (StatusCode, String) {
    let time = phone.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers = signed(&phone.device, method.as_str(), path, time, body);
    call(&phone.rig.app, method, path, &headers, body).await
}

#[tokio::test]
async fn a_paired_phone_reads_a_jobs_thread_and_answers_its_question() {
    let seen = Seen::default();
    let phone = phone_on(stub_fleet(seen.clone()).await).await;

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=h1", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, THREAD), "the thread arrives as Fleet sent it");

    let (status, body) = send(&phone, Method::POST, "/api/sessions/ask/answer", ANSWER).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, THREAD), "Fleet's answer comes back as it is");

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=nope", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], ("/sessions/one".into(), "phone".into(), "session_id=h1".into()));
    assert_eq!(seen[1], ("/sessions/ask/answer".into(), "phone".into(), ANSWER.into()));
}

#[tokio::test]
async fn the_phone_reads_this_macs_name_off_its_tailnet_address() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    assert_eq!(send(&phone, Method::GET, "/api/machine", "").await, (StatusCode::OK, r#"{"name":"mac"}"#.to_string()));
    assert_eq!(crate::machine::short_name("https://mac-b.tail1234.ts.net").as_deref(), Some("mac-b"));
    assert_eq!(crate::machine::short_name("no tailscale"), None);
}

#[tokio::test]
async fn a_picture_the_job_showed_reaches_the_phone_as_its_bytes() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let time = phone.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let path = "/api/sessions/shown?session_id=h1&path=%2Fr%2Fshot.png";
    let request = signed(&phone.device, "GET", path, time, "")
        .into_iter()
        .fold(axum::http::Request::get(path), |request, (name, value)| request.header(name, value));
    let answer = tower::ServiceExt::oneshot(phone.rig.app.clone(), request.body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
    assert_eq!(answer.headers()["content-type"], "image/png");
    assert_eq!(axum::body::to_bytes(answer.into_body(), 64).await.unwrap().as_ref(), [0x89, b'P']);
    let (status, _) = send(&phone, Method::GET, "/api/sessions/shown?session_id=h1&path=%2Fetc%2Fhosts", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_kept_style_set_reaches_the_phone_as_fleet_answers_it() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let held = send(&phone, Method::GET, "/api/sessions/kept_styles?hash=0123456789abcdef0123456789abcdef", "").await;
    assert_eq!(held, (StatusCode::OK, r#"{"sheets":[],"fonts":[]}"#.to_string()));
    let (status, _) = send(&phone, Method::GET, "/api/sessions/kept_styles?hash=..%2F..%2Fsecret", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Every method, path-and-query and body the Fleet below was sent.
type Heard = Arc<Mutex<Vec<(String, String, String)>>>;

async fn recording_fleet(heard: Heard) -> u16 {
    let app = Router::new().fallback(move |method: Method, uri: axum::http::Uri, body: String| async move {
        let target = uri.path_and_query().map_or(uri.path().to_string(), |p| p.as_str().to_string());
        heard.lock().unwrap().push((method.to_string(), target, body));
        (StatusCode::OK, r#"{"from":"fleet"}"#)
    });
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

#[tokio::test]
async fn the_routes_the_job_screens_use_are_signed_and_reach_fleet_without_api() {
    let heard = Heard::default();
    let phone = phone_on(recording_fleet(heard.clone()).await).await;
    let body = r#"{"session_id":"h1"}"#;
    let posts = [
        "/sessions/begin",
        "/sessions/stop",
        "/sessions/pause",
        "/sessions/resume",
        "/sessions/close",
        "/sessions/tune",
        "/sessions/run/start",
        "/sessions/waiting/dismiss",
        "/sessions/sketch/answer",
        "/sessions/document/answer",
        "/sessions/change_waiting",
        "/sessions/start_waiting",
    ];
    for path in posts {
        let (status, said) = send(&phone, Method::POST, &format!("/api{path}"), body).await;
        assert_eq!((status, said.as_str()), (StatusCode::OK, r#"{"from":"fleet"}"#), "{path}");
        // A signature on another path, or none, opens nothing.
        assert_eq!(call(&phone.rig.app, Method::POST, &format!("/api{path}"), &[], body).await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    for path in ["/plan_usage", "/models"] {
        let (status, said) = send(&phone, Method::GET, &format!("/api{path}"), "").await;
        assert_eq!((status, said.as_str()), (StatusCode::OK, r#"{"from":"fleet"}"#), "{path}");
        assert_eq!(call(&phone.rig.app, Method::GET, &format!("/api{path}"), &[], "").await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    let mut want: Vec<(String, String, String)> =
        posts.iter().map(|path| ("POST".to_string(), path.to_string(), body.to_string())).collect();
    want.extend(["/plan_usage", "/models"].map(|path| ("GET".to_string(), path.to_string(), String::new())));
    assert_eq!(*heard.lock().unwrap(), want, "each reached Fleet once, on its own path, with its body as signed");
}

/// The phone tells a page redrawn or revised since he read it from a refusal by Fleet's own status and code.
#[tokio::test]
async fn a_sketch_or_document_answer_fleet_refuses_comes_back_as_it_refused() {
    let app = Router::new()
        .route("/sessions/sketch/answer", post(|| async { (StatusCode::CONFLICT, r#"{"code":"fleet.sketch_redrawn"}"#) }))
        .route("/sessions/document/answer", post(|| async { (StatusCode::UNPROCESSABLE_ENTITY, r#"{"code":"fleet.document_not_open"}"#) }));
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    let phone = phone_on(port).await;

    let redrawn = send(&phone, Method::POST, "/api/sessions/sketch/answer", "{}").await;
    assert_eq!(redrawn, (StatusCode::CONFLICT, r#"{"code":"fleet.sketch_redrawn"}"#.to_string()));
    let refused = send(&phone, Method::POST, "/api/sessions/document/answer", "{}").await;
    assert_eq!(refused, (StatusCode::UNPROCESSABLE_ENTITY, r#"{"code":"fleet.document_not_open"}"#.to_string()));
}

/// What a windowed read answers, as the phone reads it: the Job's record and the rows. `older` is
/// checked on the raw body, since it is absent rather than false.
#[derive(serde::Deserialize)]
struct Page {
    session: ipc::SessionRecord,
    rows: Vec<ipc::SessionRow>,
}

fn page(body: &str) -> Page {
    ipc::decode("a window of a Job's thread", body.as_bytes()).unwrap()
}

fn ids(body: &str) -> Vec<String> {
    page(body).rows.iter().map(|row| row.id().to_string()).collect()
}

fn rows_of(count: usize) -> Vec<ipc::SessionRow> {
    ipc::decode::<ipc::SessionThread>("a thread", thread_of(count).as_bytes()).unwrap().rows
}

fn window_ids(rows: &[ipc::SessionRow], last: usize, before: Option<&str>) -> (Vec<String>, bool) {
    let (window, older) = crate::thread_window::window(rows, last, before);
    (window.iter().map(|row| row.id().to_string()).collect(), older)
}

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

#[tokio::test]
async fn the_phone_opens_a_long_thread_on_its_newest_rows_and_is_told_older_ones_stand() {
    let seen = Seen::default();
    let phone = phone_on(stub_fleet(seen.clone()).await).await;

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&last=2", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(ids(&body), strings(&["r4", "r5"]), "the newest two, oldest first");
    assert!(body.contains(r#""older":true"#), "{body}");
    let whole = ipc::decode::<ipc::SessionThread>("a thread", thread_of(5).as_bytes()).unwrap();
    assert_eq!(page(&body).session, whole.session, "the Job's record comes whole");

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], ("/sessions/one".into(), "phone".into(), "session_id=long&last=2".into()), "Fleet was asked as the phone asked");
}

#[tokio::test]
async fn a_read_before_a_row_the_phone_holds_gives_the_rows_that_stand_before_it() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&last=2&before=r4", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(ids(&body), strings(&["r2", "r3"]));
    assert!(body.contains(r#""older":true"#), "r1 stands before them: {body}");

    let (_, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&last=2&before=r3", "").await;
    assert_eq!(ids(&body), strings(&["r1", "r2"]));
    assert!(!body.contains("older"), "nothing stands before r1: {body}");

    let (_, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&last=2&before=r1", "").await;
    assert_eq!(ids(&body), Vec::<String>::new(), "nothing stands before the first row");
    assert!(!body.contains("older"), "{body}");

    let (_, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&last=2&before=gone", "").await;
    assert_eq!(ids(&body), strings(&["r4", "r5"]), "a row the thread does not hold reads as no `before`");
    assert!(body.contains(r#""older":true"#), "{body}");
}

#[tokio::test]
async fn a_last_that_reaches_past_the_first_row_gives_the_whole_thread_and_no_older() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    for last in [5, 6, 50] {
        let (status, body) = send(&phone, Method::GET, &format!("/api/sessions/one?session_id=long&last={last}"), "").await;
        assert_eq!(status, StatusCode::OK, "last={last}: {body}");
        assert_eq!(ids(&body), strings(&["r1", "r2", "r3", "r4", "r5"]), "last={last}");
        assert!(!body.contains("older"), "last={last}: {body}");
    }
}

#[tokio::test]
async fn a_thread_read_without_last_is_fleets_answer_untouched_whatever_else_is_asked() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long", "").await;
    assert_eq!((status, body), (StatusCode::OK, thread_of(5)));
    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=long&before=r3", "").await;
    assert_eq!((status, body), (StatusCode::OK, thread_of(5)), "`before` alone pages nothing");
    // Not even a row `ipc` can read is touched: the old thread above has an ask Fleet no longer sends.
    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=h1", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, THREAD));
}

#[tokio::test]
async fn a_windowed_read_of_a_job_fleet_does_not_know_is_fleets_refusal() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=nope&last=2", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::NOT_FOUND, r#"{"code":"fleet.no_session","message":"No such job."}"#));
}

#[tokio::test]
async fn a_windowed_read_of_a_thread_the_gateway_cannot_read_is_a_bad_gateway() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=h1&last=2", "").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(!body.contains("Which branch?"), "nothing of the thread leaks into the refusal: {body}");
}

#[test]
fn the_window_is_the_last_rows_before_the_end_and_says_whether_older_ones_stand() {
    let rows = rows_of(5);
    assert_eq!(window_ids(&rows, 2, None), (strings(&["r4", "r5"]), true));
    assert_eq!(window_ids(&rows, 4, None), (strings(&["r2", "r3", "r4", "r5"]), true));
    assert_eq!(window_ids(&rows, 5, None), (strings(&["r1", "r2", "r3", "r4", "r5"]), false), "the whole thread leaves none older");
    assert_eq!(window_ids(&rows, 99, None), (strings(&["r1", "r2", "r3", "r4", "r5"]), false));
    assert_eq!(window_ids(&rows, 0, None), (Vec::new(), true), "no rows asked for, and all of them stand before the end");
    assert_eq!(window_ids(&[], 3, None), (Vec::new(), false), "an empty thread has none");
}

#[test]
fn the_window_before_a_row_ends_where_that_row_stands_and_a_missing_row_means_the_end() {
    let rows = rows_of(5);
    assert_eq!(window_ids(&rows, 2, Some("r4")), (strings(&["r2", "r3"]), true));
    assert_eq!(window_ids(&rows, 2, Some("r3")), (strings(&["r1", "r2"]), false));
    assert_eq!(window_ids(&rows, 9, Some("r3")), (strings(&["r1", "r2"]), false));
    assert_eq!(window_ids(&rows, 2, Some("r1")), (Vec::new(), false));
    assert_eq!(window_ids(&rows, 2, Some("r5")), (strings(&["r3", "r4"]), true), "the row itself is not in the window");
    assert_eq!(window_ids(&rows, 2, Some("gone")), window_ids(&rows, 2, None));
    assert_eq!(window_ids(&rows, 2, Some("")), window_ids(&rows, 2, None));
}

/// What a read comes back as to a phone that sent `Accept-Encoding`: status, headers, the body's bytes.
async fn read_accepting(phone: &Phone, path: &str, encoding: Option<&str>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let time = phone.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let mut request = signed(&phone.device, "GET", path, time, "")
        .into_iter()
        .fold(axum::http::Request::get(path), |request, (name, value)| request.header(name, value));
    if let Some(encoding) = encoding {
        request = request.header("accept-encoding", encoding);
    }
    let answer = tower::ServiceExt::oneshot(phone.rig.app.clone(), request.body(axum::body::Body::empty()).unwrap()).await.unwrap();
    let (status, headers) = (answer.status(), answer.headers().clone());
    (status, headers, axum::body::to_bytes(answer.into_body(), 1 << 20).await.unwrap().to_vec())
}

fn encoding(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers.get("content-encoding").and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn a_long_thread_comes_gzipped_to_a_phone_that_accepts_it_and_plain_to_one_that_does_not() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    // The same thread read whole, as Fleet sent it, and as its newest rows the gateway cut.
    for path in ["/api/sessions/one?session_id=wide", "/api/sessions/one?session_id=wide&last=300"] {
        let (status, headers, plain) = read_accepting(&phone, path, None).await;
        assert_eq!((status, encoding(&headers)), (StatusCode::OK, None), "{path}: no gzip unasked");
        assert_eq!(plain, thread_of(300).as_bytes(), "{path}");

        let (status, headers, packed) = read_accepting(&phone, path, Some("gzip")).await;
        assert_eq!((status, encoding(&headers)), (StatusCode::OK, Some("gzip")), "{path}");
        assert_eq!(packed[..2], [0x1f, 0x8b], "{path}: a gzip stream");
        assert!(packed.len() * 4 < plain.len(), "{path}: {} bytes gzipped, {} plain", packed.len(), plain.len());
    }
}

#[tokio::test]
async fn the_live_stream_and_the_push_routes_are_never_gzipped() {
    // Fleet is down, so each answer is the gateway's own: a body long enough that gzip would take it.
    let rig = rig_with(Arc::new(|| Err("down".into())));
    let device = paired(&rig).await;
    let phone = Phone { rig, device };

    let (status, headers, body) = read_accepting(&phone, "/api/needs", Some("gzip")).await;
    assert_eq!((status, encoding(&headers)), (StatusCode::SERVICE_UNAVAILABLE, Some("gzip")), "a signed read is gzipped, so the others show a skip");
    assert_eq!(body[..2], [0x1f, 0x8b]);

    let (status, headers, body) = read_accepting(&phone, "/api/live", Some("gzip")).await;
    assert_eq!((status, encoding(&headers)), (StatusCode::SERVICE_UNAVAILABLE, None), "/api/live");
    assert!(String::from_utf8(body).unwrap().contains("not running"), "the refusal reads as text");

    let (status, headers, body) = read_accepting(&phone, "/api/push/key", Some("gzip")).await;
    assert_eq!((status, encoding(&headers)), (StatusCode::OK, None), "/api/push/key");
    assert!(String::from_utf8(body).unwrap().starts_with(r#"{"public_key":""#));
}
