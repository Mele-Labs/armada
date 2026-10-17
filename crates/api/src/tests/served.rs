//! Every operation, over the router, with no socket.
//!
//! `tower::ServiceExt::oneshot` calls the router directly — it is a `Service`,
//! and a `Service` does not need a port to be called. What this proves is the
//! step's own claim: the daemon core is drivable with zero network, because
//! nothing between a request and the daemon requires one.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobList, JobSummary, StreamMessage, WireError};
use tower::ServiceExt;

use crate::tests::fake::{at, running, FakeDaemon, THE_STUDIO};
use crate::tests::shapes;
use crate::tests::shapes::{run_id, THE_LESSON, THE_MANIFEST};
use crate::{router, Broadcaster, Next, Served, Subscription, SERVED};

fn wired(daemon: FakeDaemon, events: Broadcaster) -> Router {
    router(Served::by(daemon, run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    (status, body)
}

/// The route table is hand-written, so nothing but a call proves a path is
/// really there. This walks [`SERVED`] and refuses a row the router does not
/// answer — the runtime-500 class of mistake the accepted cost buys.
///
/// `forget_job` is skipped here and proven on its own Job below it: it is the
/// one row on this table that really deletes what it is given, and calling it
/// in place with every other row would erase `01JOB0` out from under whichever
/// rows the table still has left to check.
#[tokio::test]
async fn every_operation_the_table_names_is_routed() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    // The id the loop substitutes, made real first. A route that is present and
    // answers "no such Job" is a 404 too, and the assertion below cannot tell
    // that apart from a route that is not there.
    running(&daemon, "01JOB0");
    // `forget_job`'s own Job, terminal before the route is asked to delete it —
    // sharing `01JOB0` would leave nothing for a route later in the table to
    // answer about.
    at(&daemon, "01FORGET", "completed_success");
    let app = wired(daemon, events);
    for route in SERVED {
        // `delete_studio` for `forget_job`'s reason: the rows after it name the
        // Studio it would take away.
        if route.operation == "forget_job" || route.operation == "delete_studio" {
            continue;
        }
        let uri = route
            .path
            .replace(":job_id", "01JOB0")
            .replace(":manifest_id", THE_MANIFEST)
            .replace(":studio_id", THE_STUDIO)
            .replace(":lesson_id", THE_LESSON)
            .replace(":reference", "214");
        let (status, _) = call(&app, route.method, &uri, "").await;
        assert_ne!(
            status,
            StatusCode::NOT_FOUND,
            "{} is in the table and not in the router: {} {}",
            route.operation,
            route.method,
            route.path
        );
        assert_ne!(
            status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{} is routed at {} but not for {}",
            route.operation,
            route.path,
            route.method
        );
    }

    let (status, _) = call(&app, "POST", &format!("/studios/{THE_STUDIO}/delete"), "").await;
    assert_eq!(status, StatusCode::OK, "delete_studio is routed");
    let (status, _) = call(&app, "POST", "/jobs/01FORGET/forget_job", "").await;
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "forget_job is in the table and not in the router: POST /jobs/:job_id/forget_job"
    );
    assert_ne!(
        status,
        StatusCode::METHOD_NOT_ALLOWED,
        "forget_job is routed but not for POST"
    );
}

/// One Job, through the operations that remain, watched on the stream — and
/// not a byte of it goes near a network.
#[tokio::test]
async fn every_operation_runs_with_no_network() {
    let events = Broadcaster::new();
    let mut watching: Subscription = events.subscribe();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    // list_jobs. A list is a shape, not an absence.
    let (status, body) = call(&app, "GET", "/jobs", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: JobList = ipc::decode("job list", &body).expect("a job list");
    assert_eq!(listed.jobs.len(), 1, "only the running Job");

    // kill_job, on the Job that is running. This is the one that ends it.
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/kill_job", "").await;
    assert_eq!(status, StatusCode::OK);
    let ended: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(ended.status.as_wire(), "killed");

    // The transition reached the stream, whole and in order.
    let Some(Next::Send(delivered)) = watching.next().await else {
        panic!("the kill published nothing");
    };
    assert_eq!(delivered.cursor.position(), 0);
    let ipc::Event::JobStateChanged(moved) = delivered.event else {
        panic!("a move, not a creation");
    };
    assert_eq!(moved.from.as_wire(), "running");
    assert_eq!(moved.to.as_wire(), "killed");
}

/// **A step's gates cross before any of them acts, over HTTP and whole.**
///
/// This is the JSON a rail is drawn from, and until it carried these two keys
/// a step gated `human_always` — where the Job halts for a person — drew as a
/// step with nothing on it. What is asserted here is the serialised body
/// rather than the struct: the defect was a field that never left Rust.
#[tokio::test]
async fn a_steps_judge_and_its_human_gate_cross_before_either_acts() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    let (status, body) = call(&app, "GET", "/jobs/01RUNNING", "").await;
    assert_eq!(status, StatusCode::OK);
    let json = String::from_utf8(body.clone()).expect("a JSON body");
    let detail: ipc::JobDetail = ipc::decode("a Job in full", &body).expect("a detail");

    let gated = &detail.steps[1];
    assert_eq!(
        gated.advance_gate.expect("the gate crosses").as_wire(),
        "human_always",
        "the step the Job stops on says so: {json}"
    );

    let judged = detail.steps[0]
        .judge_checks
        .as_ref()
        .expect("the declaration crosses");
    assert_eq!(judged[0].criteria, 2);
    assert_eq!(judged[0].panel_size, Some(3), "the owner asked by name");
    assert!(judged[0].gaming_check);
    assert!(
        gated
            .judge_checks
            .as_ref()
            .expect("an answer, not a gap")
            .is_empty(),
        "asking the Judge nothing is not the same as stopping for nobody: {json}"
    );
}

/// **The same two declarations, one moment earlier.** The rail says what a
/// running Job's step did; this is the preview a person approves a dispatch
/// from, and it is the more consequential of the two — a workflow that will
/// stop and wait at `handoff`, and spend Judge calls at `implement`, previewed
/// as neither.
#[tokio::test]
async fn a_workflow_previews_the_judge_and_the_gate_it_will_stop_at() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/workflows", "").await;
    assert_eq!(status, StatusCode::OK);
    let json = String::from_utf8(body.clone()).expect("a JSON body");
    let held: Vec<ipc::WorkflowSummary> = ipc::decode("the workflows", &body).expect("a list");
    let steps = &held[0].steps;

    assert_eq!(
        steps[0].advance_gate.as_wire(),
        "auto_if_judge_passes",
        "what the first step advances on: {json}"
    );
    assert_eq!(
        steps[0].judge_checks[0].criteria, 2,
        "and how many questions that costs"
    );
    assert_eq!(
        steps[1].advance_gate.as_wire(),
        "human_always",
        "the step the dispatch will stop and wait on, said before it is approved"
    );
    assert!(
        steps[1].judge_checks.is_empty(),
        "which is not the same as a step the Judge is asked about: {json}"
    );
    assert!(
        !json.contains("question") && !json.contains("does the diff"),
        "counts and states cross, and a criterion's wording does not: {json}"
    );
}

/// A Job already over is not killed twice. The 409 is the machine refusing the
/// move, not the id being unknown.
#[tokio::test]
async fn a_job_that_is_already_over_cannot_be_killed_again() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01DONE", "completed_success");
    let app = wired(daemon, events);
    let (status, _) = call(&app, "POST", "/jobs/01DONE/kill_job", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
}

/// A Job still running has a record `kill_job` could still end, not one
/// `forget_job` may erase — the 409 says which act was wanted.
#[tokio::test]
async fn a_job_that_is_not_yet_over_cannot_be_forgotten() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/forget_job", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    let refused: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(refused.code, "fake.not_forgettable");
}

/// The record, gone. **`get_job` is a 404 from then on** — this is the case
/// `forget_job` exists for: a Board that no longer shows a Job it cleared.
#[tokio::test]
async fn a_terminal_job_can_be_forgotten_and_then_is_really_gone() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01DONE", "completed_success");
    let app = wired(daemon, events);

    let (status, body) = call(&app, "POST", "/jobs/01DONE/forget_job", "").await;
    assert_eq!(status, StatusCode::OK);
    let forgotten: ipc::JobForgotten = ipc::decode("a forgotten Job", &body).expect("the id");
    assert_eq!(forgotten.job_id.as_str(), "01DONE");

    let (status, _) = call(&app, "GET", "/jobs/01DONE", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the row is really gone");
}

#[tokio::test]
async fn a_body_that_does_not_parse_is_the_callers_fault_and_says_so() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/report", r#"{"urgency":"whenever"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "not a 500: nothing failed");
    let error: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(error.code, "api.undecodable_request");
    assert_eq!(
        error.run_id.as_str(),
        "01RUN",
        "the emitting process, always"
    );
    assert!(!error.chain.is_empty(), "the chain is always present");
}

#[tokio::test]
async fn a_command_against_a_job_that_is_not_there_is_a_404() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "POST", "/jobs/01NOTHING/kill_job", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(
        error.job_id.map(|id| id.as_str().to_string()),
        Some("01NOTHING".to_string())
    );
}

#[tokio::test]
async fn a_move_the_machine_would_refuse_is_a_409_and_not_a_404() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01DONE", "completed_success");
    let app = wired(daemon, events);
    let uri = "/jobs/01DONE/kill_job";
    let (status, _) = call(&app, "POST", uri, "").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the Job exists; it is the move that is refused"
    );
}

/// The v1 bug that lost twenty-one Jobs, one layer out. A row the store could
/// not read must reach the wire as a row that could not be read.
#[tokio::test]
async fn a_row_that_would_not_load_is_on_the_wire_and_not_filtered_away() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone()).with_unreadable("status column holds `wedged`");
    let app = wired(daemon, events);
    let (_, body) = call(&app, "GET", "/jobs", "").await;
    let listed: JobList = ipc::decode("job list", &body).expect("a job list");
    assert!(listed.jobs.is_empty());
    assert_eq!(listed.unreadable.len(), 1);
    assert!(listed.unreadable[0].fault.contains("wedged"));
}

/// A dropped event is a message, not a silence.
#[tokio::test]
async fn a_subscriber_that_falls_behind_is_told_how_many_it_lost() {
    let events = Broadcaster::with_backlog(2);
    let mut watching = events.subscribe();
    for _ in 0..6 {
        events.publish(a_transition());
    }
    let Some(Next::Missed(dropped)) = watching.next().await else {
        panic!("the bound dropped events and said nothing — the quietly-wrong failure");
    };
    assert_eq!(dropped, 4, "six published, two held");
    // And the stream continues from what is still held, rather than ending.
    assert!(matches!(watching.next().await, Some(Next::Send(_))));
}





/// The `@` mention popup's own route. **Narrowed by `?q=`, and absent reads
/// as empty** — the instant somebody types a bare `@` there is no text after
/// it yet, and a client that sent no parameter at all must see the same
/// unnarrowed list as one that sent `q=`.
#[tokio::test]
async fn manifest_files_is_narrowed_by_q_and_answers_everything_with_none() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/manifest/files?q=main", "").await;
    assert_eq!(status, StatusCode::OK);
    let found: ipc::FilesFound = ipc::decode("files found", &body).expect("a list of paths");
    assert_eq!(found.paths, vec!["src/main.rs".to_string()]);

    let (status, body) = call(&app, "GET", "/manifest/files", "").await;
    assert_eq!(status, StatusCode::OK);
    let unnarrowed: ipc::FilesFound = ipc::decode("files found", &body).expect("a list of paths");
    assert_eq!(
        unnarrowed.paths,
        shapes::files_found(""),
        "no `q` at all reads the same as `q=`, not as a refusal"
    );
}

/// The dispatch panel's repository chip. **A pick is the field, and no pick is
/// the field left out** — Bridge types it `manifest_id?: string`, and a `null`
/// would pass for a repository the words never named.
#[tokio::test]
async fn repositories_of_request_names_a_repository_or_leaves_the_field_out() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/repositories/of_request?q=fix%20storefront%20totals", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: ipc::RequestRepository = ipc::decode("a request's repository", &body).expect("a reading");
    assert_eq!(read, shapes::request_repository("storefront"));

    let (status, body) = call(&app, "GET", "/repositories/of_request?q=make%20it%20faster", "").await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("manifest_id"), "absent, not null: {text}");
    assert!(text.contains(r#""likely":["pocket","storefront"]"#), "{text}");

    let (status, _) = call(&app, "GET", "/repositories/of_request", "").await;
    assert_eq!(status, StatusCode::OK, "no `q` reads as no words");
}

/// The Cockpit's timeline. **Each bound is a query-string field and any may be left off**, and a
/// window that was cut says so while one that was not leaves the field out, as Bridge types it
/// `cut?: boolean`.
#[tokio::test]
async fn timeline_is_windowed_by_since_until_and_limit() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let read = |body: &[u8]| -> ipc::FleetTimeline { ipc::decode("a timeline", body).expect("a timeline") };

    let (status, body) = call(&app, "GET", "/timeline", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read(&body).events.len(), 2, "no bounds is everything");
    assert!(!String::from_utf8_lossy(&body).contains("cut"), "{}", String::from_utf8_lossy(&body));

    let (_, body) = call(&app, "GET", "/timeline?since=2026-10-12T10:05:00.000Z", "").await;
    assert_eq!(read(&body).events.iter().map(|one| one.id).collect::<Vec<_>>(), [2]);

    let (_, body) = call(&app, "GET", "/timeline?until=2026-10-12T10:15:00.000Z", "").await;
    assert_eq!(read(&body).events.iter().map(|one| one.id).collect::<Vec<_>>(), [1], "until is not inclusive");

    let (_, body) = call(&app, "GET", "/timeline?limit=1", "").await;
    let cut = read(&body);
    assert_eq!(cut.events.iter().map(|one| one.id).collect::<Vec<_>>(), [2], "the newest are kept");
    assert!(cut.cut);
}

/// One bay's history. **Both the repository and the slot are asked for**, an entry's `kind` tags
/// what happened, and an optional field the entry lacks is left out and never sent as `null`.
#[tokio::test]
async fn a_bays_history_is_asked_by_repository_and_slot() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/slot_history?manifest_id=armada&slot=10", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: ipc::BayHistory = ipc::decode("a bay history", &body).expect("a bay history");
    assert_eq!((read.manifest_id.as_str(), read.slot), ("armada", 10));
    assert_eq!(read.entries.len(), 3);
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains(r#""kind":"checkout""#) && text.contains(r#""kind":"leased""#), "{text}");
    assert!(!text.contains("null") && !text.contains("cut"), "{text}");

    let (status, _) = call(&app, "GET", "/slot_history?manifest_id=armada", "").await;
    assert_ne!(status, StatusCode::OK, "a slot is asked for");
}

fn a_transition() -> ipc::Event {
    let message: StreamMessage = ipc::decode(
        "stream message",
        br#"{"message":"event","cursor":0,"event":{"kind":"job.state_changed",
            "job_id":"01JOB0","from":"queued","to":"running","actor":"fleet",
            "at":"2026-08-26T09:00:00.000Z"}}"#,
    )
    .expect("a stream message");
    match message {
        StreamMessage::Event(delivered) => delivered.event,
        _ => panic!("an event"),
    }
}
