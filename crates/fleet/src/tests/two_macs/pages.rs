//! A walk a Job on Work shows, opened on Home through the signed link, on loopback. Home sees what Work's page
//! serves, its assets included; an unsigned call to Work's Gateway gets none of it; a Job in a project Work does
//! not share is not found; and a Work asleep says so in place of a blank page.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use ipc::{MacLink, OpenOtherMacPage, OtherMacPageOpened, SessionId, WireError};
use store::{AttachmentState, Holder, KeptAttachment};
use tower::ServiceExt;

use super::{job_in, paired, project, shared, until, OneMac, Ticking};

const PAGE: &str = "<h1>Walk of the auth change</h1>";
const SCRIPT: &str = "console.log('walked')";

/// A walk served on this Mac's loopback, as a Job's mock server serves it: the page, and a script it loads by an
/// absolute path.
async fn walk_served() -> u16 {
    let app = axum::Router::new()
        .route("/", get(|| async { ([("content-type", "text/html")], PAGE) }))
        .route("/assets/app.js", get(|| async { ([("content-type", "text/javascript")], SCRIPT) }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a loopback port for the walk");
    let port = listener.local_addr().expect("bound").port();
    tokio::spawn(async move { axum::serve(listener, app).await.ok() });
    port
}

/// Job `job` showed `url` with `show_window`, as its window stands on the ledger.
async fn showed(mac: &OneMac, job: &SessionId, url: &str) {
    let now = mac.rig.fleet.now().as_str().to_string();
    let window = KeptAttachment {
        holder: Holder::session(job.as_str()),
        kind: "artifact".into(),
        manifest_id: mac.rig.manifest.as_str().to_string(),
        target: url.into(),
        state: AttachmentState::Standing,
        detail: BTreeMap::from([("form".to_string(), "window".to_string()), ("title".to_string(), "Auth walk".to_string())]),
        since: now.clone(),
        changed_at: now,
    };
    mac.rig.fleet.store().lock().await.attach(&window, false).expect("kept");
}

/// Home's Bridge opens Job `job`'s page `url`: the status and what Fleet answered.
async fn opened(home: &OneMac, job: &SessionId, url: &str) -> (StatusCode, String) {
    let body = ipc::encode(&OpenOtherMacPage { session_id: job.clone(), url: url.to_string() }).expect("encodes");
    let request = Request::builder()
        .method(Method::POST)
        .uri("/sessions/other_mac_page")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("a request");
    let answer = home.fleet.clone().oneshot(request).await.expect("answered");
    let status = answer.status();
    let body = to_bytes(answer.into_body(), 1 << 20).await.expect("a body");
    (status, String::from_utf8_lossy(&body).to_string())
}

/// A plain `GET` of `url`, as a browser tab with no signature makes it: the status and the body.
async fn fetched(url: &str) -> (u16, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let rest = url.strip_prefix("http://").expect("an http address");
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let mut stream = tokio::net::TcpStream::connect(authority).await.expect("connected");
    let asked = format!("GET {} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n", if path.is_empty() { "/" } else { path });
    stream.write_all(asked.as_bytes()).await.expect("sent");
    let mut came = Vec::new();
    stream.read_to_end(&mut came).await.expect("read");
    let came = String::from_utf8_lossy(&came).to_string();
    let (head, body) = came.split_once("\r\n\r\n").expect("a head and a body");
    let status = head.split(' ').nth(1).and_then(|code| code.parse().ok()).expect("a status");
    (status, body.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_walk_a_shared_job_on_the_other_mac_shows_opens_here_and_nothing_else_does() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let taxes = project(&work.rig, "Taxes").await;
    let (open, private) = (job_in(&work.rig, Some(&shop)).await, job_in(&work.rig, Some(&taxes)).await);
    let port = walk_served().await;
    let url = format!("http://localhost:{port}/?walk=auth");
    showed(&work, &open, &url).await;
    showed(&work, &private, &url).await;
    paired(&home, &work).await;
    let job = open.clone();
    until("Home lists Work's shared Job", || async { super::from(&home.sessions().await.1, &job).is_some() }).await;

    // Home opens the walk: an address on its own loopback, with the walk's path and query, serving Work's page.
    let (status, body) = opened(&home, &open, &url).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let here: OtherMacPageOpened = ipc::decode("the page opened", body.as_bytes()).expect("an address");
    let origin = here.url.strip_suffix("/?walk=auth").expect("the walk's own path and query").to_string();
    assert!(origin.starts_with("http://127.0.0.1:") && !origin.ends_with(&format!(":{port}")), "{origin}");
    assert_eq!(fetched(&here.url).await, (200, PAGE.to_string()));
    assert_eq!(fetched(&format!("{origin}/assets/app.js")).await, (200, SCRIPT.to_string()), "an absolute path resolves on the page's own origin");

    // Work's Gateway gives an unsigned caller nothing of the page.
    let tunnel = format!("http://127.0.0.1:{}/api/mac/page/{}/{port}/?walk=auth", work.port, open.as_str());
    let (status, body) = fetched(&tunnel).await;
    assert!(!(200..300).contains(&status) && !body.contains(PAGE), "{status} {body}");

    // A Job in a project Work does not share is not on Home's list, and Work answers a signed ask for it as a
    // Job that does not exist.
    let (status, body) = opened(&home, &private, &url).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let asked = pocket::open_other_mac_page(&home.gateway, &work.id, private.as_str(), port, "/").await.expect("Work answered");
    let pocket::PageOpen::Refused { status, body } = asked else { panic!("Work served a private Job's page") };
    let said: WireError = ipc::decode("Work's refusal", &body).expect("a refusal");
    assert_eq!((status, said.code.as_str(), said.message.as_str()), (404, "fleet.no_such_session", format!("No Job has the id {}.", private.as_str()).as_str()));

    // A port the shared Job does not show is not served either.
    let other = walk_served().await;
    let asked = pocket::open_other_mac_page(&home.gateway, &work.id, open.as_str(), other, "/").await.expect("Work answered");
    assert!(matches!(asked, pocket::PageOpen::Refused { status: 404, .. }), "a port the Job never showed");

    // Work stops sharing the project: the page already open on Home stops at its next request.
    shared(&work.rig, &shop, false).await;
    let (status, body) = fetched(&here.url).await;
    assert!(status == 404 && !body.contains(PAGE), "{status} {body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_walk_on_a_mac_asleep_says_so_in_place_of_a_blank_page() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, mut work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let job = job_in(&work.rig, Some(&shop)).await;
    let url = format!("http://localhost:{}/", walk_served().await);
    showed(&work, &job, &url).await;
    paired(&home, &work).await;
    let listed = job.clone();
    until("Home lists Work's Job", || async { super::from(&home.sessions().await.1, &listed).is_some() }).await;

    work.stop().await;
    until("Home shows Work asleep", || async { matches!(home.mac(&work.id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. })) }).await;
    let Some(MacLink::Asleep { since }) = home.mac(&work.id).await.and_then(|mac| mac.link) else { unreachable!() };

    let (status, body) = opened(&home, &job, &url).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let said: WireError = ipc::decode("the refusal", body.as_bytes()).expect("a refusal");
    assert_eq!((said.code.as_str(), said.message.as_str()), ("fleet.other_mac_asleep", "Work is asleep. Its pages open when it wakes."));
    assert_eq!(said.fields.get("since"), Some(&ipc::WireValue::Int(since)));
    assert_eq!(said.fields.get("mac"), Some(&ipc::WireValue::Str(work.id.clone())));
}
