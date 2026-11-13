//! Builds across his Macs, on loopback. A Mac whose build speaks another wire freezes the link: its rows keep
//! their last state, and no act crosses or drains until the builds match. The update is the one call that
//! crosses: Home asks Work to update, Work restarts with `--adopt`, and its Jobs keep running.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::http::StatusCode;
use ipc::{FleetBuildChanging, MacLink, SendSessionMessage, SendSessionMessageBody, SessionId, SessionRow, SessionVoice, UpdateMac};

use super::{job_in, paired, project, shared, until, OneMac, Ticking};

/// A wire neither Mac's build speaks.
const OTHER_WIRE: &str = "0000000000000000";

/// A stand-in `scripts/restart-build` in a repository `mac`'s Fleet serves, which writes its arguments to the
/// answer's file, as a restart would be started.
fn wrapper_on(mac: &OneMac) -> PathBuf {
    let roots = mac.rig.fleet.ledger_roots();
    let root = PathBuf::from(&roots.first().expect("a served repository").1);
    let scripts = root.join("scripts");
    std::fs::create_dir_all(&scripts).expect("scripts");
    let wrapper = scripts.join("restart-build");
    std::fs::write(&wrapper, "#!/bin/sh\necho \"$@\" > ran\n").expect("a wrapper");
    Command::new("chmod").arg("+x").arg(&wrapper).status().expect("chmod");
    root.join("ran")
}

async fn ran(path: &Path) -> String {
    let mut text = String::new();
    until("the restart wrapper is started", || {
        let read = std::fs::read_to_string(path).unwrap_or_default();
        let done = read.ends_with('\n');
        text = read;
        async move { done }
    })
    .await;
    text.trim().to_string()
}

async fn post(mac: &OneMac, path: &str, body: String) -> (StatusCode, String) {
    post_as(mac, "bridge", path, body).await
}

/// `body` posted to `mac`'s Fleet as the caller `caller` names: `bridge`, or `mac` as a Gateway hands on a call
/// another Mac made.
async fn post_as(mac: &OneMac, caller: &str, path: &str, body: String) -> (StatusCode, String) {
    use tower::ServiceExt;
    let request = axum::http::Request::post(path)
        .header("content-type", "application/json")
        .header("x-armada-caller", caller)
        .header("x-armada-mac", "abcd")
        .body(axum::body::Body::from(body))
        .expect("a request");
    let answer = mac.fleet.clone().oneshot(request).await.expect("answered");
    let status = answer.status();
    let body = axum::body::to_bytes(answer.into_body(), 1 << 24).await.expect("a body");
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn said(mac: &OneMac, job: &SessionId, text: &str) -> usize {
    let rows = mac.rig.rows(job).await;
    rows.iter().filter(|row| matches!(row, SessionRow::Message { from: SessionVoice::You, text: said, .. } if said == text)).count()
}

/// Mac `mac`'s Gateway stopped and started again on a build of `wire`, built at `built_at` from `commit`, as a
/// restart onto another build brings it back.
async fn rebuilt(mac: &mut OneMac, wire: &str, built_at: &str, commit: &str) {
    mac.stop().await;
    mac.gateway.way_in.wire = wire.to_string();
    mac.gateway.way_in.built_at = Some(built_at.to_string());
    mac.gateway.way_in.build = Some(commit.to_string());
    mac.restart();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn builds_that_speak_another_wire_freeze_the_link_until_an_update_brings_them_to_match() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, mut work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let job = job_in(&work.rig, Some(&shop)).await;
    let wire = home.gateway.way_in.wire.clone();
    paired(&home, &work).await;
    let id = job.clone();
    until("Work's Job reaches Home's list", || async { home.sessions().await.1.sessions.iter().any(|one| one.id == id) }).await;
    let (_, before) = work.sessions().await;
    let state = before.sessions.iter().find(|one| one.id == job).expect("Work's Job").state;
    let ran_at = wrapper_on(&work);

    // 1. Work comes back on an older build that speaks another wire: each Mac says which is older, and why.
    rebuilt(&mut work, OTHER_WIRE, "2026-11-01T090000Z", "1111111").await;
    let work_id = work.id.clone();
    until("Work shows an older build on Home", || async { home.mac(&work_id).await.and_then(|mac| mac.link) == Some(MacLink::OlderBuild) }).await;
    let on_home = home.mac(&work.id).await.expect("Work on Home");
    assert_eq!(on_home.why.as_deref(), Some("Work runs an older build. Nothing crosses until it updates."));
    assert_eq!(on_home.build.as_deref(), Some("1111111"), "the row says the build it runs");
    let home_id = home.id.clone();
    until("Home shows a newer build on Work", || async { work.mac(&home_id).await.and_then(|mac| mac.link) == Some(MacLink::NewerBuild) }).await;

    // 2. Frozen: Work's rows keep their last state, and a message to its Job waits on Home, unsent.
    assert!(home.sessions().await.1.sessions.iter().any(|one| one.id == job && one.mac.as_deref() == Some(work.id.as_str())));
    let message = SendSessionMessage { session_id: job.clone(), text: "Use the blue tape".into(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() };
    let body = ipc::encode(&SendSessionMessageBody { message, aim: None }).expect("encodes");
    let (status, answered) = post(&home, "/sessions/message", body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    assert_eq!(said(&work, &job, "Use the blue tape").await, 0, "nothing crosses a frozen link");
    assert_eq!(home.rig.fleet.queued_listed(None).await.expect("the queue").queued.len(), 1, "the message waits");

    // 3. Update, pressed on Home's row for Work: the one call a frozen link carries. Work restarts with
    //    `--adopt`, and its Job is still running.
    let asked = ipc::encode(&UpdateMac { mac: Some(work.id.clone()), ..UpdateMac::default() }).expect("encodes");
    let (status, answered) = post(&home, "/macs/update", asked).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{answered}");
    let changing: FleetBuildChanging = ipc::decode("an update", answered.as_bytes()).expect("an update");
    assert_eq!(changing.build, ipc::BuildSource::Main, "Home runs main, so Work moves onto main");
    assert_eq!(ran(&ran_at).await, "main --adopt");
    let (_, after) = work.sessions().await;
    assert_eq!(after.sessions.iter().find(|one| one.id == job).expect("Work's Job").state, state, "the Job keeps running");

    // 4. Work's restart brings it back on Home's wire: linked again, its new build shown, and what waited goes.
    rebuilt(&mut work, &wire, "2026-11-13T120000Z", "2222222").await;
    until("Work is linked again on Home", || async { home.mac(&work_id).await.and_then(|mac| mac.link) == Some(MacLink::Linked) }).await;
    assert_eq!(home.mac(&work.id).await.expect("Work on Home").build.as_deref(), Some("2222222"));
    until("the message reaches Work's Job", || async { said(&work, &job, "Use the blue tape").await == 1 }).await;
    until("nothing waits on Home", || async { home.rig.fleet.queued_listed(None).await.expect("the queue").queued.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_update_another_mac_sends_may_only_move_this_mac() {
    let clock = Arc::new(Ticking::from_nine());
    let home = OneMac::new("Home", "01HMAC", &clock).await;
    let _ = wrapper_on(&home);
    let relayed = ipc::encode(&UpdateMac { mac: Some("ab".into()), ..UpdateMac::default() }).expect("encodes");
    let (status, answered) = post_as(&home, "mac", "/macs/update", relayed).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "never sent on to a third Mac: {answered}");
    let fetched = UpdateMac { build: Some(ipc::BuildSource::Preview), commit: Some("1234567".into()), reference: Some("--upload-pack=x".into()), ..UpdateMac::default() };
    let (status, answered) = post(&home, "/macs/update", ipc::encode(&fetched).expect("encodes")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "a branch that is not preview/<Mac> never reaches git: {answered}");
}
