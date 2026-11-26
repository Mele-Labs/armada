//! The person's dispatch with the Operator's session not running: **the Job starts at once as he asked, and its
//! sends follow, so his work is never lost.** What he marks it as is a plain field and picks no way and no
//! workflow. `docs/concepts/operator.md`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{DispatchJob, DispatchKind, DispatchMessage, DispatchThen, JobDispatched, SessionId, StartSession};

use super::operator_sources::on as operator_on;
use super::session_host::{eventually, rig_placing, Rig};

const BRIEF: &str = "Scrollback is lost on redraw in src/terminal/scrollback.rs. Keep it across a resize.";
const ALSO: &str = "Also keep the colour.";

const KINDS: [DispatchKind; 3] = [DispatchKind::Idea, DispatchKind::Feature, DispatchKind::Bug];

fn called(kind: DispatchKind, rig: &Rig) -> DispatchJob {
    DispatchJob { kind: Some(kind), ..the_dispatch(rig) }
}

pub(super) fn the_dispatch(rig: &Rig) -> DispatchJob {
    let start = StartSession {
        project: None,
        manifest_id: rig.manifest.clone(),
        title: None,
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: Some(BRIEF.to_string()),
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
        workflow: None,
    };
    DispatchJob { start, kind: None, then: vec![DispatchThen::Message(DispatchMessage { text: ALSO.to_string(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })] }
}

async fn dispatched(rig: &Rig, dispatch: DispatchJob) -> JobDispatched {
    Arc::clone(&rig.fleet).dispatch_job(dispatch).await.expect("dispatched")
}

async fn thread(rig: &Rig, job: &str) -> String {
    rig.fleet.store().lock().await.session_rows(job).expect("rows").join("\n")
}

/// Every Job but `except`.
async fn jobs_but(rig: &Rig, except: &SessionId) -> Vec<String> {
    let store = rig.fleet.store().lock().await;
    let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: None, text: None }).expect("read");
    found.into_iter().map(|one| one.id).filter(|id| id != except.as_str()).collect()
}

async fn dispatch_kept(rig: &Rig) -> usize {
    rig.fleet.store().lock().await.operator_dispatches().expect("read").len()
}

#[tokio::test]
async fn with_the_operator_off_a_dispatch_is_a_start_and_its_sends() {
    let rig = rig_placing(None);
    let answer = dispatched(&rig, the_dispatch(&rig)).await;
    let record = answer.session.expect("a Job");
    assert_eq!(answer.operator, None);
    eventually(|| async { thread(&rig, record.id.as_str()).await.contains(ALSO) }).await;
}

#[tokio::test]
async fn with_the_operator_on_and_its_session_not_running_a_dispatch_starts_as_he_asked() {
    let rig = rig_placing(None);
    operator_on(&rig).await;
    let mut plain = the_dispatch(&rig);
    plain.then.clear();

    let answer = dispatched(&rig, plain).await;

    assert!(answer.session.is_some() && answer.operator.is_none() && answer.parked.is_none(), "{answer:?}");
    assert_eq!(dispatch_kept(&rig).await, 0, "nothing is held for a ruling");
}

#[tokio::test]
async fn a_dispatch_he_held_is_a_plain_held_start_whatever_the_operator_is() {
    let rig = rig_placing(None);
    operator_on(&rig).await;
    let mut dispatch = the_dispatch(&rig);
    dispatch.start.held = true;

    let answer = dispatched(&rig, dispatch).await;

    let record = answer.session.expect("a Job");
    assert!(record.hosted.and_then(|hosted| hosted.start).is_some(), "held on its start card");
}

#[tokio::test]
async fn a_dispatch_kept_when_fleet_stopped_is_started_when_fleet_starts() {
    let rig = rig_placing(None);
    let body = ipc::encode(&the_dispatch(&rig)).expect("encoded");
    rig.fleet.store().lock().await.keep_operator_dispatch("start:dispatch:01OLD", &body, "2026-11-12T08:00:00.000Z").expect("kept");
    assert_eq!(dispatch_kept(&rig).await, 1);
    let nobody = SessionId::carried("none");

    rig.fleet.dispatches_resumed().await;

    assert_eq!(dispatch_kept(&rig).await, 0);
    let started = jobs_but(&rig, &nobody).await;
    assert_eq!(started.len(), 1, "his work started");
    eventually(|| async { thread(&rig, &started[0]).await.contains(ALSO) }).await;
}

#[tokio::test]
async fn an_idea_is_started_at_once_whatever_he_calls_it() {
    let rig = rig_placing(None);
    operator_on(&rig).await;

    let answer = dispatched(&rig, called(DispatchKind::Idea, &rig)).await;

    assert!(answer.session.is_some() && answer.parked.is_none(), "{answer:?}");
    assert!(rig.fleet.parked_ideas_wire().await.expect("listed").ideas.is_empty());
}

#[tokio::test]
async fn a_kind_sets_no_workflow_and_the_job_starts_as_it_does_with_none() {
    let rig = super::session_host::rig_fitted(|fittings| {
        fittings.starting().workflows = ["bug", "feature"].into_iter().map(super::daemon::workflow_named).map(|one| (one.id().clone(), one)).collect();
    });
    let workflow_of = |job: SessionId| {
        let fleet = Arc::clone(&rig.fleet);
        async move { fleet.store().lock().await.hosting(job.as_str()).expect("read").expect("hosted").workflow }
    };

    let plain = dispatched(&rig, the_dispatch(&rig)).await.session.expect("a Job");
    let plain = workflow_of(plain.id).await;
    for kind in KINDS {
        let job = dispatched(&rig, called(kind, &rig)).await.session.expect("a Job");
        assert_eq!(workflow_of(job.id).await, plain, "{kind:?}: his mark picks no workflow");
    }
    assert_eq!(plain, None, "he named none, so the proposer reads it");
}

/// Notes from the annotation layer ride a dispatch as `then`. Posted through the router on a worker of `stack` bytes,
/// as Fleet serves it: on 21 Nov the note's send ran beneath every layer of the router and the whole start, overflowed
/// the worker's stack in `sqlite3Prepare`, and aborted Fleet with the Job made and the notes lost.
fn notes_dispatched_through_the_router_on_a_worker_of(stack: usize) {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).thread_stack_size(stack).enable_all().build().expect("a runtime");
    runtime.block_on(async {
        let rig = rig_placing(None);
        let mut dispatch = the_dispatch(&rig);
        let placeholder = SessionId::carried("placeholder".to_string());
        dispatch.then = vec![DispatchThen::Notes(vec![super::session_notes::note(&placeholder, "Other commands", None).note])];
        let body = ipc::encode(&dispatch).expect("encoded");
        let events = rig.fleet.events();
        let app = api::router(api::Served::sharing(Arc::clone(&rig.fleet), ipc::RunId::carried("01RUN"), events));
        let peer: std::net::SocketAddr = "127.0.0.1:52010".parse().expect("an address");
        let request = Request::builder()
            .method("POST")
            .uri("/sessions/dispatch")
            .header("content-type", "application/json")
            .header("x-armada-caller", "bridge")
            .extension(axum::extract::ConnectInfo(peer))
            .body(Body::from(body))
            .expect("a well-formed request");

        let response = tokio::spawn(app.oneshot(request)).await.expect("the worker kept its stack").expect("an answer");

        assert!(response.status().is_success(), "{:?}", response.status());
        let ids = jobs_but(&rig, &placeholder).await;
        let [job] = &ids[..] else { panic!("one Job: {ids:?}") };
        eventually(|| async { thread(&rig, job).await.contains(super::session_notes::SAID) }).await;
    });
}

/// A debug build's start needs about 1 MiB of its own, so 1.5 MiB holds it and its sends; with the notes' send polled
/// beneath the router and the start, as it was, this worker overflowed.
#[test]
fn a_dispatch_carrying_notes_through_the_router_fits_a_one_and_a_half_megabyte_worker() {
    notes_dispatched_through_the_router_on_a_worker_of(1536 * 1024);
}
