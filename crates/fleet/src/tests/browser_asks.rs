//! A page's ask as a Fleet-held Needs you item: Bridge raises it, and answering or dismissing it in
//! Needs you publishes `browser.ask_answered` for the tab to settle, while **the agent is told
//! nothing and is not woken**. In memory only, and gone with Bridge.
//! `docs/practices/protocol/the-job-browser.md`, *A page's ask*.

use std::sync::{Arc, Mutex};

use api::{HostedSessions, Sessions, Subscription};
use ipc::{
    AnswerWaiting, BrowserAskAnswered, DismissWaiting, EndBrowserAsk, Event, RaiseBrowserAsk, SessionId, SessionRow,
    SetWaitingFor, WaitingActKind, WaitingItem, WaitingOption, WaitingSource,
};

use super::session_host::{rig_placing, Rig};

const ASK: &str = "tab-7";
const ITEM: &str = "browser_ask:tab-7";

fn option(label: &str) -> WaitingOption {
    WaitingOption { label: label.into(), description: None }
}

fn raising(job: &SessionId, ask_id: &str, text: &str) -> RaiseBrowserAsk {
    RaiseBrowserAsk { session_id: job.clone(), ask_id: ask_id.into(), text: text.into(), options: vec![option("Allow"), option("Block")] }
}

/// A Job with its agent running, and a client holding `/events` open as Bridge does.
async fn a_job_with_bridge_open(rig: &Rig) -> (SessionId, Subscription) {
    let bridge = rig.fleet.events().subscribe();
    let id = rig.start().await;
    rig.send(&id, "open the settings page").await;
    rig.stand_in.init(0);
    (id, bridge)
}

async fn asks(rig: &Rig, job: &SessionId) -> Vec<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("read").session;
    record.waiting_for.into_iter().filter(|one| one.source == WaitingSource::BrowserAsk).collect()
}

/// Every `browser.ask_answered` Fleet publishes from now on.
fn answered_events(rig: &Rig) -> Arc<Mutex<Vec<BrowserAskAnswered>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    rig.fleet.events().tap(move |event| {
        if let Event::BrowserAskAnswered(answered) = event {
            kept.lock().unwrap().push(answered.clone());
        }
    });
    seen
}

fn choosing(job: &SessionId, choice: Option<u32>, text: Option<&str>) -> AnswerWaiting {
    AnswerWaiting { session_id: job.clone(), item_id: ITEM.into(), choice, text: text.map(str::to_string), mode: None, fixed: None }
}

/// What the agent has been sent, and how many rows its thread holds: neither moves for a page's ask.
async fn agent_side(rig: &Rig, job: &SessionId) -> (Vec<String>, usize) {
    let sent = rig.stand_in.process(0).sent.lock().unwrap().clone();
    (sent, rig.rows(job).await.len())
}

#[tokio::test]
async fn a_raised_ask_is_listed_in_the_jobs_needs_you_with_its_options() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    let before = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("read").session;

    let noted = rig.fleet.raise_browser_ask(raising(&job, ASK, "localhost:5173 wants to use your location")).await.expect("held");
    assert_eq!((noted.session_id, noted.ask_id.as_str()), (job.clone(), ASK));

    let [item] = &asks(&rig, &job).await[..] else { panic!("one ask waits") };
    assert_eq!((item.id.as_str(), item.text.as_str()), (ITEM, "localhost:5173 wants to use your location"));
    let act = item.act.as_ref().expect("an act");
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Answer, ITEM));
    let labels: Vec<&str> = item.options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["Allow", "Block"]);
    let after = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("read").session;
    assert_eq!(after.standing, before.standing, "the Job's standing is as it was");
}

#[tokio::test]
async fn raising_the_same_ask_again_replaces_it_and_keeps_one() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    let first = asks(&rig, &job).await[0].since.clone();

    let again = RaiseBrowserAsk { options: vec![option("Allow once"), option("Block")], ..raising(&job, ASK, "Use your camera?") };
    rig.fleet.raise_browser_ask(again).await.expect("held");
    let other = raising(&job, "tab-9", "Use your microphone?");
    rig.fleet.raise_browser_ask(other).await.expect("held");

    let listed = asks(&rig, &job).await;
    let texts: Vec<&str> = listed.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(texts, ["Use your camera?", "Use your microphone?"], "tab-7's ask is replaced, and tab-9's is its own");
    assert_eq!(listed[0].options[0].label, "Allow once");
    assert_eq!(listed[0].since, first, "a replaced ask keeps the time it first waited");
}

#[tokio::test]
async fn answering_a_page_ask_publishes_the_choice_removes_it_and_sends_the_agent_nothing() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    let events = answered_events(&rig);
    let told = agent_side(&rig, &job).await;

    Arc::clone(&rig.fleet).answer_waiting(choosing(&job, Some(1), None)).await.expect("settled");

    let published = events.lock().unwrap().clone();
    assert_eq!(published, [BrowserAskAnswered { session_id: job.clone(), ask_id: ASK.into(), choice: Some(1) }]);
    assert!(asks(&rig, &job).await.is_empty(), "removed");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(agent_side(&rig, &job).await, told, "no message, no thread row, no Re:");
    let rows = rig.rows(&job).await;
    assert!(!rows.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.starts_with("Re:"))), "{rows:?}");
}

#[tokio::test]
async fn dismissing_a_page_ask_publishes_no_choice_removes_it_and_sends_the_agent_nothing() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    let events = answered_events(&rig);
    let told = agent_side(&rig, &job).await;

    let dismissal = DismissWaiting { session_id: job.clone(), item_id: ITEM.into(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismissal).await.expect("dismissed");

    let published = events.lock().unwrap().clone();
    assert_eq!(published, [BrowserAskAnswered { session_id: job.clone(), ask_id: ASK.into(), choice: None }]);
    assert!(asks(&rig, &job).await.is_empty(), "removed");
    let closed = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("read").session.closed;
    assert!(closed.is_empty(), "nothing to reopen: the tab owns the ask, {closed:?}");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(agent_side(&rig, &job).await, told);
}

#[tokio::test]
async fn ending_a_page_ask_removes_it_with_no_event_and_ending_it_again_is_fine() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    let events = answered_events(&rig);
    let told = agent_side(&rig, &job).await;

    assert_eq!(asks(&rig, &job).await.len(), 1, "it waits before the tab settles it");

    let ending = || EndBrowserAsk { session_id: job.clone(), ask_id: ASK.into() };
    rig.fleet.end_browser_ask(ending()).await.expect("ended");
    assert!(asks(&rig, &job).await.is_empty(), "removed");
    rig.fleet.end_browser_ask(ending()).await.expect("already gone is still 200");

    assert!(events.lock().unwrap().is_empty(), "the tab settled it, so there is nothing to tell it");
    assert_eq!(agent_side(&rig, &job).await, told);
    let late = Arc::clone(&rig.fleet).answer_waiting(choosing(&job, Some(0), None)).await;
    assert_eq!(late.expect_err("settled already").status(), 409);
}

#[tokio::test]
async fn words_alone_do_not_answer_a_page_ask() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    let events = answered_events(&rig);
    assert_eq!(asks(&rig, &job).await.len(), 1, "it waits before words are tried");

    let refused = Arc::clone(&rig.fleet).answer_waiting(choosing(&job, None, Some("yes, go ahead"))).await;
    let refusal = refused.expect_err("an ask that takes no words");
    assert_eq!(refusal.status(), 422);
    assert_eq!(asks(&rig, &job).await.len(), 1, "still waiting");
    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_agents_own_list_neither_holds_nor_drops_a_page_ask() {
    let rig = rig_placing(None);
    let (job, _bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");

    let emptied = SetWaitingFor { items: Vec::new(), session_id: Some(job.clone()) };
    rig.fleet.set_waiting(None, emptied).await.expect("set");

    assert_eq!(asks(&rig, &job).await.len(), 1, "waiting_for replaces the agent's items and no others");
}

#[tokio::test]
async fn with_bridge_gone_a_page_ask_is_not_listed_and_does_not_return() {
    let rig = rig_placing(None);
    let (job, bridge) = a_job_with_bridge_open(&rig).await;
    rig.fleet.raise_browser_ask(raising(&job, ASK, "Use your location?")).await.expect("held");
    assert_eq!(asks(&rig, &job).await.len(), 1);

    drop(bridge);
    assert_eq!(rig.fleet.events().watching(), 0, "no client holds /events open");
    assert!(asks(&rig, &job).await.is_empty(), "nothing could act on the answer");

    let _another = rig.fleet.events().subscribe();
    assert!(asks(&rig, &job).await.is_empty(), "a Bridge that opens later has none of the last one's tabs");
}
