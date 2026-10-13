//! A Commander's acts are its own Job's, and a raise and a restart are signed as
//! the connection that asked. `commander_reach` has the list and the door they
//! stand on. `#2166`.

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use super::commander_reach::{calling, commander_holding_one, from, shared, COMMANDER, REDIRECT};
use crate::tests::fake::running;
use crate::Steerer;

/// **A Commander steers one Job.** Every act it is allowed, naming another Job
/// by id, is refused with a line naming both, and reaches no handler.
#[tokio::test]
async fn an_act_naming_another_job_is_refused_and_one_naming_its_own_is_not() {
    let daemon = commander_holding_one();
    running(&daemon, "02JOB");
    let app = shared(&daemon);

    for (tool, arguments) in [
        ("redirect_drone", r#"{"job_id":"02JOB","body":{"instruction":"go on"}}"#),
        ("restart_step", r#"{"job_id":"02JOB","body":{}}"#),
        ("raise_turn_cap", r#"{"job_id":"02JOB","body":{"turn_cap":900}}"#),
        ("raise_cost_cap", r#"{"job_id":"02JOB","body":{"cost_cap_micros":9000000}}"#),
        ("restart_task", r#"{"job_id":"02JOB","task_id":"T1"}"#),
        ("drop_task", r#"{"job_id":"02JOB","body":{"task":"T1","reason":"r"}}"#),
    ] {
        let body = from(&app, COMMANDER, &[], &calling(tool, arguments)).await;
        assert!(body.contains("\"isError\":true"), "{tool}: {body}");
        assert!(
            body.contains("steers Job `01JOB` and no other") && body.contains("`02JOB`"),
            "{tool}: {body}"
        );
    }
    assert!(daemon.redirected_by.lock().expect("not poisoned").is_empty());

    // Its own Job reaches the handler (the fake then finds no Drone on it,
    // which is the fake's refusal and not the scope's).
    let own = from(&app, COMMANDER, &[], &calling("redirect_drone", REDIRECT)).await;
    assert!(!own.contains("and no other"), "{own}");
    assert_eq!(
        *daemon.redirected_by.lock().expect("not poisoned"),
        vec![Steerer::Commander]
    );
}

/// **Reads are not narrowed**, and a person's own agent session is not narrowed
/// at all: the scope belongs to the Commander's connection.
#[tokio::test]
async fn a_read_of_another_job_and_a_persons_act_on_it_are_not_refused() {
    let daemon = commander_holding_one();
    running(&daemon, "02JOB");
    let app = shared(&daemon);

    let read = from(&app, COMMANDER, &[], &calling("get_job", r#"{"job_id":"02JOB"}"#)).await;
    assert!(!read.contains("no other"), "{read}");

    let request = Request::builder()
        .method("POST")
        .uri("/jobs/02JOB/redirect")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"instruction":"go on"}"#))
        .expect("a request");
    let status = app.clone().oneshot(request).await.expect("an answer").status();
    assert_ne!(status, 403, "a person's call names any Job");
}

/// **A raise is placed by the connection** (`#2165` took the field out of the
/// body, and `#2166` is the connection that sets it): a Commander's reads as
/// the Commander's, Helm's as Helm's, and one through no door Fleet placed as a
/// person's. And a restart is signed the same way.
#[tokio::test]
async fn a_raise_and_a_restart_are_signed_as_the_connection_that_asked() {
    let daemon = commander_holding_one();
    *daemon.helm_on.lock().expect("not poisoned") = Some((52012, |_| true));
    let app = shared(&daemon);

    for port in [COMMANDER, 52012] {
        for (tool, arguments) in [
            ("raise_turn_cap", r#"{"job_id":"1","body":{"turn_cap":500}}"#),
            ("raise_cost_cap", r#"{"job_id":"1","body":{"cost_cap_micros":9000000}}"#),
            ("restart_step", r#"{"job_id":"1","body":{"note":"read the failing Check first"}}"#),
        ] {
            let body = from(&app, port, &[], &calling(tool, arguments)).await;
            assert!(body.contains("\"isError\":false"), "{tool}: {body}");
        }
    }
    let direct = |uri: &'static str, body: &'static str| {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .expect("a request")
    };
    for (uri, body) in [
        ("/jobs/1/raise_turn_cap", r#"{"turn_cap":500}"#),
        ("/jobs/1/raise_cost_cap", r#"{"cost_cap_micros":9000000}"#),
        ("/jobs/1/restart_step", r#"{"note":"read the failing Check first"}"#),
    ] {
        app.clone().oneshot(direct(uri, body)).await.expect("an answer");
    }

    use ipc::RaisedBy::{Commander, Helm, Person};
    assert_eq!(
        *daemon.raised_by.lock().expect("not poisoned"),
        vec![Commander, Commander, Helm, Helm, Person, Person]
    );
    assert_eq!(
        *daemon.restarted_by.lock().expect("not poisoned"),
        vec![Steerer::Commander, Steerer::Helm, Steerer::Person]
    );
}
