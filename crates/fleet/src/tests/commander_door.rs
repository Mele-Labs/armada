//! A Job's Commander at the agent's door, behind the real router: placed by the
//! connection the host's process holds, narrowed to its Job, and bounded at
//! three times. `#2166`. `commander_conversation` has the host these share.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use super::commander_conversation::{working, Hosted, Scripted};
use crate::allowance::{Allowance, Micros};
use crate::daemon::Fleet;
use crate::helm::Hosting;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

const PID: u32 = 4242;
const RELAY: u16 = 52100;
const SOMEONE: u16 = 52101;

/// A Fleet whose host is running a Commander turn, behind the real router, with
/// the shipped allowance so a raise meets its ceiling.
fn behind_the_door(home: &TempDir, host: &Arc<Scripted>) -> (Arc<Hosted>, Router) {
    let mut fitted = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.allowance = Allowance::of(Micros::dollars(5), 300);
    let peers = Placing::nothing();
    peers.holding(PID + 1, RELAY, fitted.host.port);
    peers.started(PID, PID + 1);
    peers.holding(9999, SOMEONE, fitted.host.port);
    fitted.peers = peers;
    let fleet = Arc::new(
        Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>),
    );
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    (fleet, app)
}

async fn at_the_door(app: &Router, port: u16, message: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(api::DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(message.to_string()))
        .expect("a request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

/// **Placed by the connection, narrowed to its Job, bounded at three times.**
/// The process is the host's and the Job is the one it was started for; a
/// second Job in the same repository is not its to touch.
#[tokio::test]
async fn a_commander_the_host_is_running_is_placed_scoped_to_its_job_and_raises_to_three_times() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let (fleet, app) = behind_the_door(&home, &host);
    let mine = working(&fleet, &home, "the Job it steers").await;
    let theirs = fleet
        .propose(a_proposal("a Job in the same repository"))
        .await
        .expect("a proposal");
    host.steering
        .lock()
        .expect("not poisoned")
        .push((PID, mine.as_str().to_string()));
    let mine_handle = fleet.load(&mine).await.expect("reads").handle();

    // Listed the allowlist, and not Helm's rows.
    let list = at_the_door(&app, RELAY, LISTING).await;
    assert!(list.contains("\"name\":\"raise_turn_cap\""), "{list}");
    assert!(!list.contains("\"name\":\"undo_run\""), "{list}");

    // Three times what the workflow allows: 900 of 300.
    let over = at_the_door(
        &app,
        RELAY,
        &calling(
            "raise_turn_cap",
            &format!(r#"{{"job_id":"{mine_handle}","body":{{"turn_cap":901}}}}"#),
        ),
    )
    .await;
    assert!(
        over.contains("\"isError\":true") && over.contains("900"),
        "{over}"
    );
    let raised = at_the_door(
        &app,
        RELAY,
        &calling(
            "raise_turn_cap",
            &format!(r#"{{"job_id":"{mine_handle}","body":{{"turn_cap":900}}}}"#),
        ),
    )
    .await;
    assert!(raised.contains("\"isError\":false"), "{raised}");
    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &mine_handle,
    ))
    .expect("the Job's log");
    assert!(
        log.contains("\"raised_by\"") && log.contains("commander"),
        "the raise is the Commander's, placed by the connection: {log}"
    );

    // Another Job in the same repository is refused, naming both.
    let elsewhere = at_the_door(
        &app,
        RELAY,
        &calling(
            "raise_turn_cap",
            &format!(
                r#"{{"job_id":"{}","body":{{"turn_cap":900}}}}"#,
                theirs.id().as_str()
            ),
        ),
    )
    .await;
    assert!(
        elsewhere.contains("\"isError\":true") && elsewhere.contains("and no other"),
        "{elsewhere}"
    );
    assert_eq!(
        fleet.load(theirs.id()).await.expect("reads").turn_cap(),
        None,
        "and nothing moved on it"
    );

    // A person's own agent session is not narrowed to a Job, and its raise is a
    // person's: no ceiling, whatever Job it names.
    let anyone = at_the_door(
        &app,
        SOMEONE,
        &calling(
            "raise_turn_cap",
            &format!(
                r#"{{"job_id":"{}","body":{{"turn_cap":900}}}}"#,
                theirs.id().as_str()
            ),
        ),
    )
    .await;
    assert!(anyone.contains("\"isError\":false"), "{anyone}");
    assert_eq!(
        fleet.load(theirs.id()).await.expect("reads").turn_cap(),
        Some(900)
    );
}
