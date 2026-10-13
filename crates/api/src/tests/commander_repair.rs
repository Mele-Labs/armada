//! A repair Drone is the Commander's alone to send, reaches the daemon naming
//! only what broke, and is not `add_job_step`. `#2170`.

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use super::commander_reach::{calling, commander_holding_one, from, shared, ANYONE, COMMANDER};
use crate::{offerable, CommanderReach, COMMANDER_ALLOWED};

const MERGE_CONFLICT: &str = r#"{"job_id":"01JOB","body":{"broke":"merge_conflict"}}"#;

#[test]
fn the_repair_is_on_a_commanders_allowlist_and_in_no_other_callers_reach() {
    assert!(COMMANDER_ALLOWED.contains(&"send_repair_drone"));
    assert!(
        !offerable().any(|row| row.operation == "send_repair_drone"),
        "not any agent's, and not Helm's: the move it ends in is signed Commander"
    );
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB"));
    assert!(reach.may("send_repair_drone"));
}

/// A step the Commander wrote would be gated as that step's, not as a repair.
#[test]
fn the_commander_is_given_a_repair_of_its_own_and_never_a_step_to_write() {
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB"));
    assert!(!COMMANDER_ALLOWED.contains(&"add_job_step"));
    assert!(!reach.may("add_job_step"));
}

#[tokio::test]
async fn a_commander_is_listed_the_repair_and_a_persons_agent_is_not() {
    let app = shared(&commander_holding_one());
    let listing = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let commander = from(&app, COMMANDER, &[], listing).await;
    assert!(commander.contains(r#""name":"send_repair_drone""#), "{commander}");
    let anyone = from(&app, ANYONE, &[], listing).await;
    assert!(!anyone.contains(r#""name":"send_repair_drone""#), "{anyone}");
}

#[tokio::test]
async fn a_repair_sent_through_the_commanders_door_reaches_the_daemon_naming_what_broke() {
    let daemon = commander_holding_one();
    let app = shared(&daemon);

    let body = from(&app, COMMANDER, &[], &calling("send_repair_drone", MERGE_CONFLICT)).await;

    assert!(!body.contains("\"isError\":true"), "{body}");
    assert_eq!(
        *daemon.repairs_sent.lock().expect("not poisoned"),
        vec![(ipc::JobId::carried("01JOB"), ipc::Breakage::MergeConflict)]
    );
}

#[tokio::test]
async fn a_repair_naming_a_fault_outside_the_four_or_none_does_not_decode() {
    let daemon = commander_holding_one();
    let app = shared(&daemon);
    for body in [
        r#"{"job_id":"01JOB","body":{"broke":"the_weather"}}"#,
        r#"{"job_id":"01JOB","body":{}}"#,
    ] {
        let answered = from(&app, COMMANDER, &[], &calling("send_repair_drone", body)).await;
        assert!(answered.contains("\"isError\":true"), "{body}: {answered}");
    }
    assert!(daemon.repairs_sent.lock().expect("not poisoned").is_empty());
}

/// **The route is not a person's.** The same bytes posted straight to the
/// surface carry no Commander and are refused, so the move it ends in is never
/// signed as anyone else's.
#[tokio::test]
async fn a_repair_posted_to_the_surface_by_anyone_else_is_refused() {
    let daemon = commander_holding_one();
    let app = shared(&daemon);
    let request = Request::builder()
        .method("POST")
        .uri("/jobs/01JOB/commander/repair")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"broke":"merge_conflict"}"#))
        .expect("a request");

    let status = app.oneshot(request).await.expect("an answer").status();

    assert_eq!(status, 409);
    assert!(daemon.repairs_sent.lock().expect("not poisoned").is_empty());
}
