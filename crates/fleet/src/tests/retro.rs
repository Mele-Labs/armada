//! A Job's retro, end to end over the router: the door each act came through,
//! a Drone's note on what got in its way, and the one retro written once the
//! Job ends. `docs/concepts/retro.md`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobHistory, JobRetro, JobSummary, Lessons, Movement, RetroState, RunId};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::tests::daemon::{fittings, worktree_directory_named};
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;

/// Who is asking: Bridge names itself, and anything else does not.
#[derive(Clone, Copy)]
pub(super) enum From {
    Bridge,
    Anyone,
}

pub(super) async fn sent(
    app: &Router,
    method: &str,
    uri: &str,
    body: &str,
    from: From,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:51000"
                .parse::<std::net::SocketAddr>()
                .expect("a loopback address"),
        ));
    if let From::Bridge = from {
        request = request.header(api::CALLER_HEADER, api::BRIDGE);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.to_string()))
                .expect("a request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    (status, body)
}

/// A Fleet whose model calls `judge` answers, with one Job approved from
/// Bridge and its Drone on the first step.
pub(super) async fn running(
    home: &TempDir,
    judge: Arc<FakeJudge>,
) -> (Arc<Fixture>, Router, String) {
    let mut fitted = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.judge = judge;
    // A second workflow to choose from, which only a Job proposed from a
    // request reads: the fixture's own Job names the one it runs.
    let bug = crate::tests::daemon::workflow_named("bug");
    fitted.starting().workflows.insert(bug.id().clone(), bug);
    let fleet = Arc::new(Fleet::assembled(fitted));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    let (status, body) = sent(&app, "POST", "/jobs", A_PROPOSAL, From::Bridge).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("a Job", &body).expect("a JobSummary");
    worktree_directory_named(home, &proposed.handle);
    let id = proposed.id.as_str().to_string();
    let (status, _) = sent(
        &app,
        "POST",
        &format!("/jobs/{id}/approve_dispatch"),
        "",
        From::Bridge,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    fleet.turn().await.expect("the turn puts a Drone on it");
    (fleet, app, id)
}

pub(super) async fn retro_of(app: &Router, id: &str) -> JobRetro {
    let (status, body) = sent(app, "GET", &format!("/jobs/{id}/retro"), "", From::Bridge).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    ipc::decode("a retro", &body).expect("a JobRetro")
}

pub(super) async fn killed(app: &Router, id: &str, from: From) {
    let (status, body) = sent(app, "POST", &format!("/jobs/{id}/kill_job"), "", from).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}

/// **A person's press in Bridge and an agent's request are told apart on the
/// timeline.** The approval came from Bridge; the kill came from a request
/// that named nobody, which is what an agent's `curl` is, and it is signed
/// `helm` with the door it came through.
#[tokio::test]
async fn a_press_in_bridge_and_an_agents_request_read_apart_on_the_timeline() {
    let home = TempDir::new();
    let (_fleet, app, id) = running(&home, Arc::new(FakeJudge::saying("{\"items\":[]}"))).await;

    killed(&app, &id, From::Anyone).await;

    let (_, body) = sent(&app, "GET", &format!("/jobs/{id}/events"), "", From::Bridge).await;
    let history: JobHistory = ipc::decode("a history", &body).expect("a JobHistory");
    let signed: Vec<(&str, &str, Option<&str>)> = history
        .moves
        .iter()
        .filter_map(|row| match &row.moved {
            Movement::Status(to) => Some((
                to.to.as_wire(),
                row.actor.as_wire(),
                row.via.map(|via| via.as_wire()),
            )),
            _ => None,
        })
        .filter(|(to, ..)| matches!(*to, "queued" | "killed"))
        .collect();
    assert_eq!(
        signed,
        vec![
            ("queued", "human", Some("bridge")),
            ("killed", "helm", Some("http")),
        ]
    );
}

/// **What a Drone says got in its way reaches the record**, and a submission
/// that says nothing leaves nothing.
#[tokio::test]
async fn what_got_in_a_drones_way_reaches_the_retro_record() {
    let home = TempDir::new();
    let (fleet, app, id) = running(&home, Arc::new(FakeJudge::saying("{\"items\":[]}"))).await;

    let call = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"submit_evidence",
        "arguments":{"claimed":"The reader stops one line later.","shown_by":"src/log.rs",
                     "not_claimed":"","in_the_way":"the asked run log was cut at 200 lines"}}}"#;
    let (status, body) = sent(&app, "POST", api::MCP_PATH, call, From::Anyone).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !String::from_utf8_lossy(&body).contains("\"isError\":true"),
        "{}",
        String::from_utf8_lossy(&body)
    );
    fleet.turn().await.expect("the gate runs");

    let retro = retro_of(&app, &id).await;
    let notes: Vec<&str> = retro
        .record
        .notes
        .iter()
        .map(|note| note.said.as_str())
        .collect();
    assert_eq!(notes, vec!["the asked run log was cut at 200 lines"]);
    assert_eq!(retro.state, RetroState::Pending, "the Job has not ended");
}

/// **An ended Job gets one retro, held to its record, and it is listed.** An
/// item citing a row the record does not hold loses that citation, and one
/// citing nothing the record holds is dropped.
#[tokio::test]
async fn an_ended_job_gets_one_retro_and_its_items_are_listed() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        "Here it is:\n```json\n{\"items\":[\
         {\"who\":\"owner\",\"lands_in\":\"armada\",\
          \"title\":\"The Job waited on its approval\",\
          \"what\":\"The Job waited on its approval.\",\
          \"fix\":\"Show the wait on the Board.\",\
          \"evidence\":[\"act:1\",\"act:99\"]},\
         {\"who\":\"drone\",\"lands_in\":\"kit\",\"title\":\"Invented\",\
          \"what\":\"Invented.\",\"fix\":\"Invent less.\",\
          \"evidence\":[\"refusal:7\"]}]}\n```",
    ));
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;

    let written = fleet.reflect_next().await.expect("written");
    assert_eq!(
        written.map(|job| job.as_str().to_string()),
        Some(id.clone())
    );
    assert_eq!(
        fleet.reflect_next().await.expect("read"),
        None,
        "one retro per Job, and no call for one already written"
    );

    let retro = retro_of(&app, &id).await;
    assert_eq!(retro.state, RetroState::Written);
    let [item] = retro.items.as_slice() else {
        panic!("one item survives: {:?}", retro.items);
    };
    assert_eq!(item.who.as_wire(), "owner");
    assert_eq!(item.evidence, vec!["act:1".to_string()]);
    assert_eq!(item.id, format!("{id}-0"));
    assert_eq!(
        item.title.as_deref(),
        Some("The Job waited on its approval")
    );
    assert_eq!(
        item.what.as_deref(),
        Some("The Job waited on its approval.")
    );
    assert_eq!(item.fix.as_deref(), Some("Show the wait on the Board."));
    assert_eq!(item.statement, "The Job waited on its approval.");
    assert_eq!(
        judge.models(),
        vec!["the-retro-model".to_string()],
        "a retro is written on its own model, never the Judge's"
    );
    let asked = judge.asked();
    let [question] = asked.as_slice() else {
        panic!("one call: {asked:?}");
    };
    assert!(question.contains("-----BEGIN RECORD-----"), "{question}");
    assert!(
        question.contains("\"via\":\"http\""),
        "the door reaches the model: {question}"
    );

    let (_, body) = sent(&app, "GET", "/lessons", "", From::Bridge).await;
    let lessons: Lessons = ipc::decode("the lessons", &body).expect("Lessons");
    let [lesson] = lessons.lessons.as_slice() else {
        panic!("one lesson: {:?}", lessons.lessons);
    };
    assert_eq!(lesson.job_id.as_str(), id);
    assert!(lesson.handle.starts_with('1'), "{}", lesson.handle);
    assert_eq!(lesson.statement, "The Job waited on its approval.");
    assert_eq!(lesson.id, format!("{id}-0"));
    assert_eq!(
        lesson.title.as_deref(),
        Some("The Job waited on its approval")
    );
    assert_eq!(lesson.state.as_wire(), "open");
    assert_eq!(lesson.job_proposed, None);
}

/// **A call that fails is kept as failed and not made again**: a quota that is
/// gone does not come back on the next turn.
#[tokio::test]
async fn a_retro_whose_call_fails_is_kept_as_failed_and_not_asked_again() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::that_fails("the quota"));
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Bridge).await;

    fleet.reflect_next().await.expect("kept");
    assert_eq!(fleet.reflect_next().await.expect("read"), None);

    let retro = retro_of(&app, &id).await;
    assert_eq!(retro.state, RetroState::Failed);
    assert!(retro.why.is_some_and(|why| why.contains("the call failed")));
    assert_eq!(judge.asked().len(), 1);
}

/// **A Job stopped at the gate is owed nothing to look back on**, and no call
/// is made for it.
#[tokio::test]
async fn a_job_no_drone_ran_on_is_skipped_without_a_call() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("{\"items\":[]}"));
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.judge = judge.clone();
    let fleet = Arc::new(Fleet::assembled(fitted));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    let (_, body) = sent(&app, "POST", "/jobs", A_PROPOSAL, From::Bridge).await;
    let proposed: JobSummary = ipc::decode("a Job", &body).expect("a JobSummary");
    killed(&app, proposed.id.as_str(), From::Bridge).await;

    fleet.reflect_next().await.expect("kept");

    let retro = retro_of(&app, proposed.id.as_str()).await;
    assert_eq!(retro.state, RetroState::Skipped);
    assert!(judge.asked().is_empty());
}

/// **Every item says where its fix lands, or is dropped**: one naming no place,
/// or a place that is not one of the three, is left out the way one citing
/// nothing is, and the items beside it are kept. `?lands_in=` narrows the
/// Lessons to one place.
#[tokio::test]
async fn an_item_that_names_no_place_its_fix_lands_is_dropped_and_lessons_narrow_by_it() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        "{\"items\":[\
         {\"who\":\"owner\",\"lands_in\":\"armada\",\"title\":\"Shown running\",\
          \"what\":\"Shown running.\",\"fix\":\"Stop showing it.\",\
          \"evidence\":[\"act:1\"]},\
         {\"who\":\"drone\",\"lands_in\":\"kit\",\"title\":\"Refused grep\",\
          \"what\":\"Refused grep.\",\"fix\":\"Allow grep.\",\
          \"evidence\":[\"act:1\"]},\
         {\"who\":\"owner\",\"title\":\"No place\",\"what\":\"Named no place.\",\
          \"fix\":\"Name one.\",\"evidence\":[\"act:1\"]},\
         {\"who\":\"fleet\",\"lands_in\":\"bridge\",\"title\":\"Wrong place\",\
          \"what\":\"Named a wrong one.\",\"fix\":\"Name a right one.\",\
          \"evidence\":[\"act:1\"]}]}",
    ));
    let (fleet, app, id) = running(&home, judge).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let retro = retro_of(&app, &id).await;
    assert_eq!(retro.state, RetroState::Written);
    let kept: Vec<(&str, Option<&str>)> = retro
        .items
        .iter()
        .map(|item| {
            (
                item.statement.as_str(),
                item.lands_in.map(|lands| lands.as_wire()),
            )
        })
        .collect();
    assert_eq!(
        kept,
        vec![
            ("Shown running.", Some("armada")),
            ("Refused grep.", Some("kit")),
        ]
    );

    let (status, body) = sent(&app, "GET", "/lessons?lands_in=kit", "", From::Bridge).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let lessons: Lessons = ipc::decode("the lessons", &body).expect("Lessons");
    let narrowed: Vec<&str> = lessons
        .lessons
        .iter()
        .map(|lesson| lesson.statement.as_str())
        .collect();
    assert_eq!(narrowed, vec!["Refused grep."]);

    let (_, body) = sent(&app, "GET", "/lessons", "", From::Bridge).await;
    let lessons: Lessons = ipc::decode("the lessons", &body).expect("Lessons");
    assert_eq!(lessons.lessons.len(), 2, "absent is all three");

    let (status, _) = sent(&app, "GET", "/lessons?lands_in=bridge", "", From::Bridge).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a place that is not one");
}

/// **Items are read one at a time, and only a whole one is kept.** A `fix` with
/// a dash, a missing `title`, a `what` that is blank, an item citing nothing
/// the record holds and a stray string in the list are each dropped alone, and
/// the one good item beside them is kept with every part.
#[tokio::test]
async fn an_item_with_a_dash_or_a_missing_part_is_dropped_and_the_good_one_kept() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        "{\"items\":[\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"A dash in the fix\",\
          \"what\":\"The gate ran against a stale main.\",\
          \"fix\":\"Fetch main \\u2014 then measure.\",\"evidence\":[\"act:1\"]},\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"A dash in the what\",\
          \"what\":\"The gate ran against a stale main -- twice.\",\
          \"fix\":\"Fetch main first.\",\"evidence\":[\"act:1\"]},\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\
          \"what\":\"No title came with it.\",\"fix\":\"Add one.\",\"evidence\":[\"act:1\"]},\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"Blank what\",\"what\":\"  \",\
          \"fix\":\"Say it.\",\"evidence\":[\"act:1\"]},\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"Cites nothing\",\
          \"what\":\"Nothing backs this.\",\"fix\":\"Cite a row.\",\"evidence\":[]},\
         \"a stray string\",\
         {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"Gate measured a stale main\",\
          \"what\":\"Two upstream commits counted as the Drone's work.\",\
          \"fix\":\"Fetch main before the gate measures.\",\"evidence\":[\"act:1\"]}]}",
    ));
    let (fleet, app, id) = running(&home, judge).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let retro = retro_of(&app, &id).await;
    assert_eq!(retro.state, RetroState::Written, "{:?}", retro.why);
    let [item] = retro.items.as_slice() else {
        panic!("one item survives: {:?}", retro.items);
    };
    assert_eq!(item.title.as_deref(), Some("Gate measured a stale main"));
    assert_eq!(
        item.what.as_deref(),
        Some("Two upstream commits counted as the Drone's work.")
    );
    assert_eq!(
        item.fix.as_deref(),
        Some("Fetch main before the gate measures.")
    );
}

/// The instructions say what the owner decided: no cause the record does not
/// show, Armada's own faults to `fleet` and `armada`, and a plain style with no
/// dashes. And a retro is not asked about a file the Drone never named without
/// the fact that says so.
#[tokio::test]
async fn the_retro_call_is_told_to_name_no_cause_the_record_does_not_show() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("{\"items\":[]}"));
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let asked = judge.asked();
    let [question] = asked.as_slice() else {
        panic!("one call: {asked:?}");
    };
    for told in [
        "Name a cause only where the record shows one",
        "named_in_drone_calls: false",
        "`who: fleet`, `lands_in: armada`",
        "No dashes of any kind",
        "`title`",
        "`fix`",
    ] {
        assert!(question.contains(told), "{told}: {question}");
    }
}
