//! A hosted Job ending and a pull request merging, as they reach the Operator: once each, only those
//! after the tap started, for every Job whoever started it, and nothing while the Operator is off. The
//! words carry what the end may unblock, a ruling of `noted` writes only its Log row, an end settles
//! what waited on the Job's own run as Done, and the roles decide whether a Job is told or it is proposed.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{AttachmentReport, CloseSession, SessionFact, SessionId, SessionRecord, SessionReport, SessionRow};
use tokio::sync::Semaphore;

use super::backlog::manifest;
use super::operator_notes::filing;
use super::operator_sources::{rulings, switched, Answers};
use super::session_host::{eventually, Rig};
use crate::operator::{keep_the_ends_told, Answering, Arrival, Arrived, Calling, Source, Way};
use crate::CallFailed;

/// With a file way offered and no live Job near it, an end is offered: 1 file, 2 new job, 3 raise, 4 noted.
const NOTED: &str = "item: 1\nway: 4\njob: none\nshared: none\nwhat: none\nbecause: nothing follows";
/// With a Job waiting on it: 1 file, 2 existing job, 3 new job, 4 raise, 5 noted.
const NOTED_WITH_A_WAITER: &str = "item: 1\nway: 5\njob: none\nshared: none\nwhat: none\nbecause: nothing follows";

async fn end(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// What the mod tells Fleet of Job `id`.
async fn report(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let harness = record(rig, id).await.harness;
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("reported");
}

/// A pull request of Job `id` as the forge describes it, then settled spent, as the mod tells one.
async fn merge(rig: &Rig, id: &SessionId, number: &str, title: &str) {
    let detail = [("state".to_string(), "merged".to_string()), ("title".to_string(), title.to_string())].into();
    report(rig, id, SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: number.into(), detail } }).await;
    report(rig, id, SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: number.into() }, state: ipc::AttachmentState::Spent }).await;
}

async fn pause() {
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// A Job that ends, and one live Job waiting to start on it. Both are started before the call is seated.
async fn blocker_and_waiter(rig: &Rig) -> (SessionId, SessionId) {
    let blocker = rig.start_titled(Some("Build the store")).await;
    let waiter = rig.start_titled(Some("Wire the pane")).await;
    let wait = store::KeptWait {
        session_id: waiter.as_str().to_string(),
        on_session: blocker.as_str().to_string(),
        condition: "ends".to_string(),
        part: String::new(),
        base: "main".to_string(),
        needs: String::new(),
        brief: "Wire the pane to the store".to_string(),
        held: false,
        told: false,
        by: "you".to_string(),
        pull_number: None,
        pull_branch: None,
        created_at: "2026-11-06T09:00:00.000Z".to_string(),
        started_at: None,
    };
    rig.fleet.store().lock().await.keep_wait(&wait).expect("kept");
    (blocker, waiter)
}

async fn thread_of(rig: &Rig, id: &SessionId) -> String {
    let rows: Vec<SessionRow> = rig.rows(id).await;
    format!("{rows:?}")
}

#[tokio::test]
async fn operator_ends_a_job_that_ends_after_start_arrives_once_with_its_summary_and_one_ended_before_does_not() {
    let rig = filing().await;
    let before = rig.start_titled(Some("Old work")).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    end(&rig, &before).await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    // Said again, an end from before the tap is no arrival.
    rig.fleet.published_hosted(before.as_str()).await.expect("published");
    pause().await;
    assert!(call.asked().is_empty(), "{:#?}", call.asked());

    rig.fleet.store().lock().await.keep_done(job.as_str(), "2026-11-06T09:00:00.000Z", Some("Moved the pane to the store.\nIt keeps scrollback now.")).expect("kept");
    end(&rig, &job).await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in [
        "A Job ended, one of Fleet's own: \"Build the pane\"",
        "What it said it achieved: Moved the pane to the store. It keeps scrollback now.",
        "The ways it is offered: 1. file a new issue; 2. new job; 3. raise; 4. noted, nothing follows\n",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(!asked[0].contains("Old work"), "{}", asked[0]);
    let log = rulings(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrival.clone(), log[0].arrived.as_str(), log[0].outcome.as_str()), (format!("ended:{}", job.as_str()), "ended", "noted"));
    let board = rig.fleet.operator_board_wire().await.expect("read");
    assert_eq!(board.done[0].outcome, "Ended: Moved the pane to the store.");

    // Published again, it is not handed twice.
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    pause().await;
    assert_eq!(call.asked().len(), 1);
    assert_eq!(rulings(&rig).await.len(), 1);
}

#[tokio::test]
async fn operator_ends_a_pull_request_that_merges_after_start_arrives_once_and_one_merged_before_does_not() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    merge(&rig, &job, "100", "An older change").await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    pause().await;
    assert!(call.asked().is_empty(), "{:#?}", call.asked());

    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in ["A pull request merged, one of Fleet's own: \"Make the pane keep scrollback\"", "Pull request #3042 \"Make the pane keep scrollback\" merged. Job \"Build the pane\" held it."] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(!asked[0].contains("An older change"), "{}", asked[0]);
    let log = rulings(&rig).await;
    assert_eq!((log[0].arrival.clone(), log[0].arrived.as_str(), log[0].outcome.as_str()), (format!("merged:{}:3042", job.as_str()), "merged", "noted"));
    let board = rig.fleet.operator_board_wire().await.expect("read");
    assert_eq!(board.done[0].outcome, "Merged: #3042 Make the pane keep scrollback");

    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;
    pause().await;
    assert_eq!(call.asked().len(), 1);
    assert_eq!(rulings(&rig).await.len(), 1);
}

#[tokio::test]
async fn operator_ends_a_job_handed_on_for_context_is_not_an_end() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    // Its record as it ends with a successor, which `replaced_by` names.
    let mut record = record(&rig, &job).await;
    record.state = ipc::SessionState::Ended;
    let handed = ipc::Attachment {
        kind: "replaced_by".to_string(),
        manifest_id: None,
        target: "01NEXT".to_string(),
        state: ipc::AttachmentState::Standing,
        detail: BTreeMap::new(),
        since: ipc::Instant::carried("2026-11-06T09:00:00.000Z"),
        changed_at: ipc::Instant::carried("2026-11-06T09:00:00.000Z"),
    };
    let mut on = record.clone();
    on.attachments.push(handed);
    rig.fleet.events().publish(ipc::Event::SessionChanged(on));
    pause().await;
    assert!(call.asked().is_empty(), "{:#?}", call.asked());

    // The same record with nothing handed on is an end, so the quiet above was the rule and not the test.
    rig.fleet.events().publish(ipc::Event::SessionChanged(record));
    eventually(|| async { !call.asked().is_empty() }).await;
}

#[tokio::test]
async fn operator_ends_a_waiting_start_leaves_next_and_is_done_as_ended_when_its_job_ends() {
    let rig = filing().await;
    let first = rig.start_titled(Some("Keeps the call busy")).await;
    let waiting = rig.start_titled(Some("Waits on the call")).await;
    let call = Arc::new(Blocked { gate: Semaphore::new(0), asked: Mutex::new(Vec::new()) });
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    let manifest = manifest(&rig).as_str().to_string();
    let seat = rig.fleet.operator_seat();
    seat.hand(start(&first, &manifest, "start:first", "Keeps the call busy"), Duration::from_secs(30));
    eventually(|| async { !call.asked.lock().expect("not poisoned").is_empty() }).await;
    seat.hand(start(&waiting, &manifest, &format!("start:{}", waiting.as_str()), "Waits on the call"), Duration::from_secs(30));
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.operator_board_wire().await.expect("read").next.len() == 1 }
    })
    .await;

    end(&rig, &waiting).await;

    let id = format!("start:{}", waiting.as_str());
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        let id = id.clone();
        async move { fleet.operator_board_wire().await.expect("read").done.iter().any(|one| one.id == id) }
    })
    .await;
    let board = rig.fleet.operator_board_wire().await.expect("read");
    assert!(board.next.iter().all(|one| one.id != id), "{:?}", board.next);
    assert_eq!(board.done.iter().find(|one| one.id == id).map(|one| one.outcome.as_str()), Some("Ended"));
    assert_eq!(board.now.as_ref().map(|one| one.id.as_str()), Some("start:first"), "the call out goes on");
}

#[tokio::test]
async fn operator_ends_the_words_name_a_job_waiting_to_start_on_the_ended_job_and_noted_writes_only_a_log_row() {
    let rig = filing().await;
    let (blocker, waiter) = blocker_and_waiter(&rig).await;
    let call = Answers::with(&[NOTED_WITH_A_WAITER]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    end(&rig, &blocker).await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    let said = format!("Waiting to start on it: Job {} \"Wire the pane\" waits to start until it ends.", waiter.as_str());
    assert!(asked[0].contains(&said), "{said}:\n{}", asked[0]);
    assert!(asked[0].contains("2. existing job"), "{}", asked[0]);
    pause().await;
    let log = rulings(&rig).await;
    assert_eq!(log.iter().map(|one| one.outcome.as_str()).collect::<Vec<_>>(), ["noted"]);
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty());
    assert!(!thread_of(&rig, &waiter).await.contains("The Operator tells you"));
}

fn tell_the_waiter(waiter: &SessionId) -> String {
    format!("item: 1\nway: 2\njob: {}\nshared: none\nwhat: none\nbecause: what it waited on ended", waiter.as_str())
}

#[tokio::test]
async fn operator_ends_an_existing_job_on_an_end_is_kept_as_a_suggestion_at_suggest() {
    let rig = filing().await;
    let (blocker, waiter) = blocker_and_waiter(&rig).await;
    let save = ipc::SaveSettings { changes: BTreeMap::from([("features.operatorDelegate".to_string(), Some(ipc::SettingValue::Text("suggest".into())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
    let call = Answers::with(&[tell_the_waiter(&waiter).as_str()]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    end(&rig, &blocker).await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let kept = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read");
    assert_eq!(kept[0].text, "The Operator would tell \"Wire the pane\" that \"Build the store\" ended: what it waited on ended");
    pause().await;
    assert!(!thread_of(&rig, &waiter).await.contains("The Operator tells you"), "nothing is told until he presses");
}

#[tokio::test]
async fn operator_ends_an_existing_job_on_an_end_tells_the_job_at_on() {
    let rig = filing().await;
    let (blocker, waiter) = blocker_and_waiter(&rig).await;
    let call = Answers::with(&[tell_the_waiter(&waiter).as_str()]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    end(&rig, &blocker).await;

    let thread = || async { thread_of(&rig, &waiter).await };
    eventually(|| async { thread().await.contains("The Operator tells you what may unblock your work") }).await;
    assert!(thread().await.contains("Job \\\"Build the store\\\" ended."), "{}", thread().await);
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty());
}

#[tokio::test]
async fn operator_ends_nothing_arrives_while_the_operator_is_off_and_nothing_is_replayed_when_it_comes_on() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    switched(&rig, "features.operator", false).await;

    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;
    end(&rig, &job).await;
    pause().await;
    assert!(call.asked().is_empty());

    switched(&rig, "features.operator", true).await;
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    pause().await;
    assert!(call.asked().is_empty(), "{:#?}", call.asked());
    assert!(rulings(&rig).await.is_empty());
}

/// A call that is out until a test lets it go, and answers nothing.
struct Blocked {
    gate: Semaphore,
    asked: Mutex<Vec<String>>,
}

impl Calling for Blocked {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        Box::pin(async move {
            self.gate.acquire().await.expect("open").forget();
            Err(CallFailed::SaidNothing)
        })
    }
}

/// A Job's start, as it waits for a ruling.
fn start(job: &SessionId, manifest: &str, id: &str, title: &str) -> Arrival {
    Arrival {
        id: id.to_string(),
        arrived: Arrived::Start,
        source: Source::Dispatch,
        session_id: job.as_str().to_string(),
        manifest_id: manifest.to_string(),
        subject: title.to_string(),
        words: "The words".to_string(),
        paths: Vec::new(),
        issues: Vec::new(),
        group: None,
        ways: vec![Way::NewJob, Way::Raise],
        answers: Vec::new(),
        rule: None,
        open: Vec::new(),
        ties: Vec::new(),
    }
}
