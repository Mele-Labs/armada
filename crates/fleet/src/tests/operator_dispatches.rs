//! The claim, written before the code: **with the Operator on and seated, the person's dispatch goes to it
//! before any Job exists.** It starts a Job, folds the work into a live Job, or asks him on its card; where no
//! live Job is near, the Operator is off or a call fails, the Job starts at once, and his work is never lost.
//! `docs/concepts/operator.md`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next};
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorTurnItem};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::{DispatchJob, DispatchMessage, DispatchThen, Event, JobDispatched, SessionId, StartSession};
use store::KeptTouch;

use super::operator_sources::{switched, Answers};
use super::session_host::{eventually, rig_placing, Rig};
use crate::operator::keep_the_raises;

const FILE: &str = "src/terminal/scrollback.rs";
const BRIEF: &str = "Scrollback is lost on redraw in src/terminal/scrollback.rs. Keep it across a resize.";
const ALSO: &str = "Also keep the colour.";
const LIVE: &str = "Fix the scrollback";

/// A live Job titled `LIVE` that has written `FILE`.
async fn a_live_job(rig: &Rig) -> SessionId {
    let job = rig.start_titled(Some(LIVE)).await;
    let touch = KeptTouch { by: None, path: FILE.into(), act: "edit".into(), at: "2026-11-10T08:00:00.000Z".into() };
    rig.fleet.store().lock().await.keep_session_touch(job.as_str(), &touch, 50).expect("kept");
    job
}

/// The Operator seated and on, with the answers its call gives.
async fn seated(rig: &Rig, answers: &[&str]) -> Arc<Answers> {
    let call = Answers::with(answers);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    switched(rig, "features.operator", true).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    call
}

/// The ruling on item 1 that takes way `way`, offered as 1 new job, 2 existing job, 3 raise.
fn ruling(way: u8, live: Option<&SessionId>) -> String {
    match live {
        Some(job) => format!("item: 1\nway: {way}\njob: {}\nshared: {FILE}\nbecause: the same file", job.as_str()),
        None => format!("item: 1\nway: {way}\nbecause: nothing else covers it"),
    }
}

fn the_dispatch(rig: &Rig) -> DispatchJob {
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
    };
    DispatchJob { start, then: vec![DispatchThen::Message(DispatchMessage { text: ALSO.to_string(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })] }
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

async fn told(watching: &mut api::Subscription) -> OperatorDispatched {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await {
            Ok(Some(Next::Send(delivered))) => {
                if let Event::OperatorDispatched(one) = delivered.event {
                    return one;
                }
            }
            other => panic!("no operator.dispatched came: {other:?}"),
        }
    }
}

/// The go-ahead no Job holds, once the Operator has raised it.
async fn card(rig: &Rig) -> OperatorTurnItem {
    for _ in 0..500 {
        let page = rig.fleet.operator_page_wire().await.expect("read");
        if let Some(one) = page.turn.into_iter().find(|one| one.item.id.starts_with("suggestion:")) {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no go-ahead is waiting");
}

fn press(item: &OperatorTurnItem, answer: OperatorAnswer) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer, then: None, option: None, project: None }
}

#[tokio::test]
async fn a_dispatch_no_live_job_is_near_starts_at_once_with_no_call_and_is_told() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[]).await;
    let mut watching = rig.fleet.events().subscribe();

    let answer = dispatched(&rig, the_dispatch(&rig)).await;

    let record = answer.session.expect("a Job");
    assert_eq!(answer.operator, None);
    assert!(call.asked().is_empty(), "no live Job is near, so no call");
    let event = told(&mut watching).await;
    assert_eq!((event.way, event.session_id.as_ref(), event.manifest_id.as_str()), (DispatchWay::Started, Some(&record.id), rig.manifest.as_str()));
    eventually(|| async { thread(&rig, record.id.as_str()).await.contains(ALSO) }).await;
    assert_eq!(dispatch_kept(&rig).await, 0);
}

#[tokio::test]
async fn a_dispatch_the_operator_rules_a_new_job_starts_the_job_and_sends_what_followed_it() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let call = seated(&rig, &[&ruling(1, None)]).await;
    let mut watching = rig.fleet.events().subscribe();

    let answer = dispatched(&rig, the_dispatch(&rig)).await;

    let id = answer.operator.expect("held by the Operator");
    assert!(answer.session.is_none());
    let event = told(&mut watching).await;
    assert_eq!((event.way, event.dispatch_id.as_str()), (DispatchWay::Started, id.as_str()));
    let started = event.session_id.expect("names the Job");
    assert_ne!(started, live);
    assert_eq!(call.asked().len(), 1);
    eventually(|| async { thread(&rig, started.as_str()).await.contains(ALSO) }).await;
    assert_eq!(dispatch_kept(&rig).await, 0, "the kept body is gone");
    let page = rig.fleet.operator_page_wire().await.expect("read");
    assert!(page.turn.iter().all(|one| !one.item.id.starts_with("suggestion:")), "it asked him nothing");
}

#[tokio::test]
async fn a_dispatch_the_operator_rules_an_existing_job_folds_the_work_into_it_and_starts_none() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    seated(&rig, &[&ruling(2, Some(&live))]).await;
    let mut watching = rig.fleet.events().subscribe();

    let answer = dispatched(&rig, the_dispatch(&rig)).await;

    assert!(answer.operator.is_some());
    let event = told(&mut watching).await;
    assert_eq!((event.way, event.session_id.as_ref(), event.title.as_deref()), (DispatchWay::Handed, Some(&live), Some(LIVE)));
    eventually(|| async { thread(&rig, live.as_str()).await.contains(ALSO) }).await;
    let said = thread(&rig, live.as_str()).await;
    assert!(said.contains("Keep it across a resize"), "its words went to the live Job: {said}");
    assert!(jobs_but(&rig, &live).await.is_empty(), "no Job was started");
    assert_eq!(dispatch_kept(&rig).await, 0);
}

#[tokio::test]
async fn a_raise_asks_him_hand_to_or_keep_both_and_hand_to_folds_it_into_the_live_job() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    seated(&rig, &[&ruling(3, None)]).await;
    let mut watching = rig.fleet.events().subscribe();

    let answer = dispatched(&rig, the_dispatch(&rig)).await;

    assert!(answer.operator.is_some());
    let asked = told(&mut watching).await;
    assert_eq!((asked.way, asked.session_id), (DispatchWay::Asked, None));
    let one = card(&rig).await;
    let labels: Vec<&str> = one.item.options.iter().map(|option| option.label.as_str()).collect();
    assert_eq!(labels, [format!("Hand to {LIVE}").as_str(), "Keep both"]);
    assert_eq!(dispatch_kept(&rig).await, 1, "kept while he decides");
    assert!(jobs_but(&rig, &live).await.is_empty(), "nothing started yet");

    rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes)).await.expect("answered");

    let handed = told(&mut watching).await;
    assert_eq!((handed.way, handed.session_id.as_ref()), (DispatchWay::Handed, Some(&live)));
    eventually(|| async { thread(&rig, live.as_str()).await.contains(ALSO) }).await;
    assert!(jobs_but(&rig, &live).await.is_empty());
    assert_eq!(dispatch_kept(&rig).await, 0);
}

#[tokio::test]
async fn keep_both_starts_the_job_as_it_was_asked() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    seated(&rig, &[&ruling(3, None)]).await;
    let mut watching = rig.fleet.events().subscribe();
    dispatched(&rig, the_dispatch(&rig)).await;
    told(&mut watching).await;
    let one = card(&rig).await;

    rig.fleet.operator_answered(press(&one, OperatorAnswer::No)).await.expect("answered");

    let started = told(&mut watching).await;
    assert_eq!(started.way, DispatchWay::Started);
    let job = started.session_id.expect("names the Job");
    assert_ne!(job, live);
    eventually(|| async { thread(&rig, job.as_str()).await.contains(ALSO) }).await;
    assert_eq!(dispatch_kept(&rig).await, 0);
}

#[tokio::test]
async fn a_call_that_fails_starts_the_job_at_once_so_his_work_is_not_lost() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    seated(&rig, &[]).await;
    let mut watching = rig.fleet.events().subscribe();

    let answer = dispatched(&rig, the_dispatch(&rig)).await;

    assert!(answer.operator.is_some());
    let event = told(&mut watching).await;
    assert_eq!(event.way, DispatchWay::Started);
    let job = event.session_id.expect("names the Job");
    assert_ne!(job, live);
    eventually(|| async { thread(&rig, job.as_str()).await.contains(ALSO) }).await;
    assert_eq!(dispatch_kept(&rig).await, 0);
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
async fn a_dispatch_he_held_is_a_plain_held_start_whatever_the_operator_is() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
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
