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
use ipc::WireError;
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, THE_STUDIO};
use crate::tests::shapes;
use crate::tests::shapes::{run_id, THE_LESSON, THE_MANIFEST};
use crate::{router, Broadcaster, Next, Served, SERVED};

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
/// `delete_studio` is skipped in the loop and proven after it: it really
/// deletes what it is given, and calling it in place would erase the Studio out
/// from under whichever rows the table still has left to check.
#[tokio::test]
async fn every_operation_the_table_names_is_routed() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    let app = wired(daemon, events);
    for route in SERVED {
        if route.operation == "delete_studio" {
            continue;
        }
        let uri = route
            .path
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
}

/// **A step's gates cross before any of them acts.** This is the preview a
/// person approves a dispatch from — a workflow that will stop and wait at
/// `handoff`, and spend Judge calls at `implement`, previewed as neither,
/// would be the defect.
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

/// A body that does not decode is the caller's fault, and a command against a
/// Studio that is not there is a 404.
#[tokio::test]
async fn a_body_that_does_not_parse_is_the_callers_fault_and_says_so() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "POST", "/studios/create", r#"{"name":5}"#).await;
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
async fn a_command_against_a_studio_that_is_not_there_is_a_404() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "POST", "/studios/01NOTHING/delete", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(error.code, "fake.no_such_studio");
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

/// The test map, read for paths that ride the query string newline-separated: a reached path is
/// grouped under its Check and member, and an unreached one is named, so the caller can say "no
/// test reaches this file" instead of running nothing.
#[tokio::test]
async fn tests_reaching_takes_paths_newline_separated_and_names_the_unreached() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/test_map/reaching?paths=src%2Fmain.rs%0Asrc%2Fnobody.rs%0A", "").await;
    assert_eq!(status, StatusCode::OK);
    let reaching: ipc::TestsReaching = ipc::decode("the tests reaching", &body).expect("a map reading");
    assert_eq!(reaching.unreached, vec!["src/nobody.rs".to_string()], "the blank last line is no path");
    assert!(reaching.holds_edges, "the map's holding rides the wire");
    assert_eq!(reaching.checks.len(), 1);
    assert_eq!((reaching.checks[0].check.as_str(), reaching.checks[0].member.as_str()), ("test", "armada"));
    let test = &reaching.checks[0].tests[0];
    assert_eq!((test.source.as_wire(), test.grain.as_wire(), test.stale), ("measured", "test", false));

    let (status, body) = call(&app, "GET", "/test_map/reaching", "").await;
    assert_eq!(status, StatusCode::OK, "no `paths` at all is an empty answer, not a refusal");
    let empty: ipc::TestsReaching = ipc::decode("the tests reaching", &body).expect("a map reading");
    assert_eq!(empty, ipc::TestsReaching { holds_edges: true, ..ipc::TestsReaching::default() }, "no path asked, and the map still holds its edges");
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
    ipc::Event::StudioDeleted(ipc::StudioDeleted {
        id: ipc::StudioId::carried("01STUDIO"),
        manifest_id: ipc::ManifestId::carried(THE_MANIFEST),
    })
}

/// `read_loadout_item` reads its row off the query string, and a kind outside the closed set is the
/// transport's 400 rather than a read.
#[tokio::test]
async fn a_loadout_item_is_asked_on_the_query_string() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(
        &app,
        "GET",
        "/sessions/loadout/item?session_id=s1&kind=skill&name=productivity%3Astart&source=plugin&from=productivity",
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let answer: ipc::LoadoutItemBody = ipc::decode("an item's body", &body).expect("a body");
    assert!(matches!(answer, ipc::LoadoutItemBody::NotRead { .. }));
    let (status, _) = call(&app, "GET", "/sessions/loadout/item?session_id=s1&kind=everything&name=x&source=kit", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// A hosted job's MCP servers read on the query string, each state its snake_case word and an absent
/// field left out; an act answers the servers after it and, for a sign-in, where to go.
#[tokio::test]
async fn a_jobs_mcp_servers_are_read_and_acted_on() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "GET", "/sessions/mcp?session_id=s1", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains(r#""state":"needs_sign_in""#) && text.contains(r#""tools":3"#), "{text}");
    assert!(!text.contains("null"), "{text}");
    let read: ipc::SessionMcp = ipc::decode("a job's servers", &body).expect("the servers");
    assert_eq!(read.servers.len(), 2);

    let (status, body) = call(&app, "POST", "/sessions/mcp/act", r#"{"session_id":"s1","server":"linear","act":"sign_in"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let acted: ipc::SessionMcpActed = ipc::decode("an act's answer", &body).expect("an act's answer");
    let sign_in = acted.sign_in.expect("a sign-in that needs the person");
    assert!(sign_in.url.starts_with("https://") && sign_in.callback_expected);

    let (status, body) = call(&app, "POST", "/sessions/mcp/act", r#"{"session_id":"s1","server":"linear","act":"turn_off"}"#).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!String::from_utf8_lossy(&body).contains(r#""sign_in":"#), "only a sign-in says where to go");

    let (status, _) = call(&app, "POST", "/sessions/mcp/act", r#"{"session_id":"s1","server":"linear","act":"restart"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "an act outside the four is the transport's 400");

    let (status, body) = call(&app, "GET", "/sessions/mcp?session_id=closed", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("fleet.session_closed"));
}
