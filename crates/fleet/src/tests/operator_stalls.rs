//! A Job gone quiet with work left, as Fleet reads it from state and hands it to the Operator: one
//! it marked Stopped, and one that ended a turn after it was told its pull request failed and
//! pushed nothing since. Each arrives once however often its row is read, nothing is ruled while
//! the Operator is off, and a new Job for the rest and a raise each say what they did.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use api::{HostedSessions, Sessions};
use ipc::{PlanGroupInput, PlanTaskInput, RecordPlan, SessionId, SessionStanding};
use store::{AttachmentState, Holder, KeptAttachment};

use super::rule_acting::notices_in;
use super::session_host::{eventually, rig_placing, Rig};
use super::session_rail::record;
use super::session_stops::{aged, stopped_a_while_ago, tell_ripe};
use crate::operator::{Acting, Answering, Calling, FleetActing};
use crate::CallFailed;

async fn switched(rig: &Rig, key: &str, on: bool) {
    let value = match key {
        "features.operator" => ipc::SettingValue::Bool(on),
        _ => ipc::SettingValue::Text(if on { "on" } else { "off" }.to_string()),
    };
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(value))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn on(rig: &Rig) {
    switched(rig, "features.operator", true).await;
    switched(rig, "features.operatorDispatch", true).await;
    switched(rig, "features.operatorDelegate", true).await;
}

async fn told(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().filter(|(notice, _)| notice.as_deref() == Some("operator")).map(|(_, text)| text).collect()
}

async fn said(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().map(|(_, text)| text).collect()
}

/// A call that answers each question with the next of its answers, and keeps what it was asked.
#[derive(Default)]
struct Answers {
    answers: Mutex<VecDeque<String>>,
    asked: Mutex<Vec<String>>,
}

impl Answers {
    fn with(answers: &[&str]) -> Arc<Answers> {
        Arc::new(Answers { answers: Mutex::new(answers.iter().map(|one| one.to_string()).collect()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for Answers {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let answer = self.answers.lock().expect("not poisoned").pop_front();
        Box::pin(async move { answer.ok_or(CallFailed::SaidNothing) })
    }
}

/// A call that counts what it is asked and answers nothing.
#[derive(Default)]
struct Counting(AtomicUsize);

impl Calling for Counting {
    fn ask(&self, _question: String) -> Answering<'_> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Err(CallFailed::SaidNothing) })
    }
}

/// `id`'s plan, approved: one group of two tasks, the first of them done.
async fn planned(rig: &Rig, id: &SessionId) {
    let plan = RecordPlan {
        summary: "You get the terminal pane back.".into(),
        groups: vec![PlanGroupInput {
            id: None,
            title: "Scrollback".into(),
            agent: "rust-engineer".into(),
            paths: vec!["apps/panes/terminal.tsx".into()],
            issue: None,
            gives: None,
            lands_in: None,
            own_job: false,
            tasks: ["Find where the scrollback is dropped", "Keep the scrollback on redraw"]
                .into_iter()
                .map(|title| PlanTaskInput { id: None, title: title.into() })
                .collect(),
        }],
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    };
    rig.fleet.record_plan(None, plan, false).await.expect("recorded");
    let mut store = rig.fleet.store().lock().await;
    let mut kept = store.session_plan(id.as_str()).expect("read").expect("kept");
    kept.groups[0].tasks[0].listed = Some("completed".into());
    store.keep_session_plan_progress(id.as_str(), &kept).expect("kept");
    // Approved, as he would have before it went quiet: a plan waiting on him excuses a stop.
    let at = rig.fleet.now().as_str().to_string();
    store.approve_session_plan(id.as_str(), kept.version, &at).expect("approved");
}

/// A Job with a plan, told once to carry on, that stopped again: Fleet marks it Stopped.
async fn stopped_twice(rig: &Rig) -> SessionId {
    let id = stopped_a_while_ago(rig).await;
    planned(rig, &id).await;
    tell_ripe(rig, &id).await;
    rig.stand_in.finishes(0, "Still written.");
    eventually(|| async { record(rig, &id).await.standing == Some(SessionStanding::Stopped) }).await;
    id
}

/// `id` holds pull request 7, open, whose required check `ci` failed.
async fn holds_red(rig: &Rig, id: &SessionId) {
    let manifest = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted").manifest_id;
    let at = rig.fleet.now().as_str().to_string();
    let detail = [("state", "open"), ("checks", "failed"), ("failing", "ci"), ("auto_merge", "false"), ("queued", "false")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let pull = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "pr".into(),
        manifest_id: manifest,
        target: "7".into(),
        state: AttachmentState::Standing,
        detail,
        since: at.clone(),
        changed_at: at,
    };
    rig.fleet.store().lock().await.attach(&pull, false).expect("attached");
}

/// Fleet told `id` that pull request 7 failed at `head`, `ago` milliseconds back.
async fn told_red(rig: &Rig, id: &SessionId, head: &str, ago: i64) {
    let at = core_model::Timestamp::from_rfc3339(crate::clock::rfc3339_utc(rig.fleet.now().epoch_millis().unwrap() - ago));
    let recipient = format!("session:{}", id.as_str());
    rig.fleet.store().lock().await.keep_pull_notice(&rig.root, 7, head, "checks_failed", &recipient, &at).expect("kept");
}

/// Its last turn ended `ago` milliseconds back.
async fn turn_ended(rig: &Rig, id: &SessionId, ago: i64) {
    let mut store = rig.fleet.store().lock().await;
    let mut row = store.session(id.as_str()).unwrap().expect("a row");
    row.last_turn_at = Some(crate::clock::rfc3339_utc(rig.fleet.now().epoch_millis().unwrap() - ago));
    store.keep_session(&row).unwrap();
}

/// The pull requests of `id`'s Manifest read by a reading that began `ago` milliseconds back.
async fn pulls_read(rig: &Rig, id: &SessionId, ago: i64) {
    let manifest = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted").manifest_id;
    let at = core_model::Timestamp::from_rfc3339(crate::clock::rfc3339_utc(rig.fleet.now().epoch_millis().unwrap() - ago));
    rig.fleet.sweeping().lock().await.pulls_read.insert(manifest, at);
}

/// The row read, as a publish reads it, and the stalls it noted handed on.
async fn read_and_handed(rig: &Rig, id: &SessionId) {
    let _ = record(rig, id).await;
    rig.fleet.stalls_handed().await;
}

#[tokio::test]
async fn a_job_stopped_twice_with_tasks_left_is_shown_with_its_plan_and_has_the_rest_started_as_a_job_when_the_call_names_new_job() {
    let rig = rig_placing(None);
    let id = stopped_twice(&rig).await;
    on(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nbecause: its next task waits on nobody"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    read_and_handed(&rig, &id).await;

    eventually(|| async { said(&rig, &id).await.iter().any(|one| one.contains("The Operator started what is left of this Job")) }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in [
        "A Job stopped with work left, one of Fleet's own",
        "Its plan: You get the terminal pane back.",
        "Done: Find where the scrollback is dropped.",
        "Left: Keep the scrollback on redraw.",
        "The ways it is offered: 1. new job; 2. raise",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(told(&rig, &id).await.is_empty(), "no word goes to the quiet Job");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str(), log[0].by.as_str()), ("stopped", "new_job", "operator"));
    assert_eq!(log[0].session_id, id.as_str());
}

#[tokio::test]
async fn a_red_pull_request_the_job_ended_a_turn_on_with_no_push_arrives_once_however_often_its_row_is_read() {
    let rig = rig_placing(None);
    on(&rig).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = stopped_a_while_ago(&rig).await;
    holds_red(&rig, &id).await;

    // Told of the failure after its last turn ended: the word has not reached a turn yet.
    told_red(&rig, &id, "2222222", 60_000).await;
    read_and_handed(&rig, &id).await;
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "not yet");
    assert_ne!(record(&rig, &id).await.standing, Some(SessionStanding::Stopped), "an open pull request excuses a stop");

    // A turn since ended, and the pull request is red on a reading from before it: a push in that
    // turn shows only on the next reading.
    turn_ended(&rig, &id, 30_000).await;
    pulls_read(&rig, &id, 40_000).await;
    read_and_handed(&rig, &id).await;
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "not on a reading from before the turn ended");

    // Read again since, and still red.
    pulls_read(&rig, &id, 10_000).await;
    for _ in 0..3 {
        read_and_handed(&rig, &id).await;
    }
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { !fleet.store().lock().await.operator_rulings(10).expect("read").is_empty() }
    })
    .await;
    read_and_handed(&rig, &id).await;
    read_and_handed(&rig, &id).await;

    assert_eq!(counted.0.load(Ordering::SeqCst), 1, "one call");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrived.as_str(), log[0].arrival.as_str()), ("red", format!("red:{}:7:2222222", id.as_str()).as_str()));
}

#[tokio::test]
async fn a_red_pull_request_is_not_a_stall_while_a_subagent_is_still_working() {
    let rig = rig_placing(None);
    on(&rig).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = stopped_a_while_ago(&rig).await;
    holds_red(&rig, &id).await;
    told_red(&rig, &id, "2222222", 60_000).await;
    turn_ended(&rig, &id, 30_000).await;
    pulls_read(&rig, &id, 10_000).await;
    let at = rig.fleet.now().as_str().to_string();
    let subagent = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "subagent".into(),
        manifest_id: String::new(),
        target: "x1".into(),
        state: AttachmentState::Standing,
        detail: Default::default(),
        since: at.clone(),
        changed_at: at,
    };
    rig.fleet.store().lock().await.attach(&subagent, false).expect("attached");

    read_and_handed(&rig, &id).await;
    read_and_handed(&rig, &id).await;
    assert!(!record(&rig, &id).await.running.is_empty(), "it is working");
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "no call");
    assert!(rig.fleet.store().lock().await.operator_rulings(10).expect("read").is_empty(), "no ruling");
}

#[tokio::test]
async fn with_the_operator_off_a_stopped_job_is_ruled_on_by_nobody_and_no_call_is_made() {
    let rig = rig_placing(None);
    assert!(!rig.fleet.operator_on(), "the Operator ships off");
    switched(&rig, "features.operatorDispatch", true).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = stopped_twice(&rig).await;

    read_and_handed(&rig, &id).await;
    read_and_handed(&rig, &id).await;
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "no call");
    assert!(rig.fleet.store().lock().await.operator_rulings(10).expect("read").is_empty(), "no ruling");
    assert!(told(&rig, &id).await.is_empty());

    // The same Job with the Operator on is ruled, so the silence above is the switch's.
    switched(&rig, "features.operator", true).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { counted.0.load(Ordering::SeqCst) == 1 }).await;
}

#[tokio::test]
async fn a_new_job_takes_the_rest_and_a_raise_reaches_his_needs_you_list() {
    let rig = rig_placing(None);
    on(&rig).await;
    let id = rig.start().await;
    rig.send(&id, "Fix the terminal pane").await;
    aged(&rig, &id).await;
    let acting = FleetActing::on(&rig.fleet);

    let before = rig.fleet.store().lock().await.live_session_ids().expect("read");
    acting.start_rest(id.as_str(), "It stopped again after Fleet told it to carry on. Left: Open terminals in a tab.");
    eventually(|| async { said(&rig, &id).await.iter().any(|one| one.contains("The Operator started what is left of this Job")) }).await;
    let rest = rig.fleet.store().lock().await.live_session_ids().expect("read").into_iter().find(|one| !before.contains(one)).expect("a new Job");
    let rest = SessionId::carried(rest.as_str());
    let row = record(&rig, &rest).await;
    assert!(row.title.as_deref().is_some_and(|title| title.starts_with("Rest of ")), "{:?}", row.title);
    assert!(row.attachments.iter().any(|one| one.kind == "started_by" && one.target == id.as_str()), "nested under it");
    let brief = rig.rows(&rest).await;
    assert!(format!("{brief:?}").contains("Left: Open terminals in a tab."), "{brief:?}");
    assert_ne!(record(&rig, &id).await.state, ipc::SessionState::Ended, "the stalled Job is left to him");

    acting.raise(id.as_str(), "the failing check needs a key only the person holds");
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    eventually(|| {
        let (fleet, session) = (Arc::clone(&fleet), session.clone());
        async move {
            let calls = fleet.store().lock().await.calls_of(&session).expect("read");
            calls.iter().any(|call| call.notice == "operator" && call.text.contains("the failing check needs a key only the person holds"))
        }
    })
    .await;
}

/// A Job stopped twice, with Dispatch at Suggest and a new Job ruled, and its suggestion on his list.
async fn stall_suggested(rig: &Rig) -> (SessionId, ipc::WaitingItem) {
    stall_suggested_then(rig, &[]).await
}

/// The same, with `later` the call's answers after the ruling: to what he says about it.
async fn stall_suggested_then(rig: &Rig, later: &[&str]) -> (SessionId, ipc::WaitingItem) {
    let id = stopped_twice(rig).await;
    switched(rig, "features.operator", true).await;
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operatorDispatch".to_string(), Some(ipc::SettingValue::Text("suggest".into())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
    let ruling = "item: 1\nway: 1\nbecause: its next task waits on nobody";
    let call = Answers::with(&[&[ruling][..], later].concat());
    assert!(rig.fleet.operator_seated_with(call.clone()));
    read_and_handed(rig, &id).await;
    eventually(|| async { !suggestions(rig, &id).await.is_empty() }).await;
    let item = suggestions(rig, &id).await.remove(0);
    (id, item)
}

async fn suggestions(rig: &Rig, id: &SessionId) -> Vec<ipc::WaitingItem> {
    let read = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read");
    read.session.waiting_for.into_iter().filter(|one| one.source == ipc::WaitingSource::OperatorSuggestion).collect()
}

fn pressed(id: &SessionId, item: &ipc::WaitingItem, choice: u32) -> ipc::AnswerWaiting {
    ipc::AnswerWaiting { session_id: id.clone(), item_id: item.id.clone(), choice: Some(choice), text: None, mode: None, fixed: None }
}

#[tokio::test]
async fn with_dispatch_at_suggest_a_new_job_waits_on_one_press_which_starts_the_rest_and_a_second_finds_it_settled() {
    let rig = rig_placing(None);
    let (id, item) = stall_suggested(&rig).await;
    assert!(item.text.contains("its next task waits on nobody"), "{}", item.text);
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Do it", "Skip"]);
    assert!(said(&rig, &id).await.iter().all(|one| !one.contains("started what is left")), "nothing is started until he presses");

    Arc::clone(&rig.fleet).settle_waiting(pressed(&id, &item, 0)).await.expect("his press");

    eventually(|| async { said(&rig, &id).await.iter().any(|one| one.contains("The Operator started what is left of this Job")) }).await;
    assert!(suggestions(&rig, &id).await.is_empty(), "the row is gone");
    let log = rig.fleet.store().lock().await.operator_rulings_of(id.as_str()).expect("read");
    assert_eq!(log.last().map(|row| (row.outcome.as_str(), row.by.as_str())), Some(("new_job", "human")), "the Log says who pressed");
    assert!(Arc::clone(&rig.fleet).settle_waiting(pressed(&id, &item, 0)).await.is_err(), "the first press won");
}

#[tokio::test]
async fn skipping_what_dispatch_would_do_starts_nothing_and_dismissing_it_is_the_same() {
    let rig = rig_placing(None);
    let (id, item) = stall_suggested(&rig).await;

    Arc::clone(&rig.fleet).settle_waiting(pressed(&id, &item, 1)).await.expect("his press");

    assert!(suggestions(&rig, &id).await.is_empty(), "the row is gone");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(said(&rig, &id).await.iter().all(|one| !one.contains("started what is left")), "nothing was started");
}

#[tokio::test]
async fn words_on_a_suggestions_row_are_talk_to_the_operator_about_it_and_the_row_stays() {
    let rig = rig_placing(None);
    let (id, item) = stall_suggested_then(&rig, &["cites: none\njob: none\nstart: none\norder: none\nfirst: none\nsaid: It would start the rest as a Job."]).await;
    let arrival = rig.fleet.store().lock().await.operator_suggestions_of(id.as_str()).expect("read").remove(0).arrival;

    let words = ipc::AnswerWaiting { session_id: id.clone(), item_id: item.id.clone(), choice: None, text: Some("why this one?".into()), mode: None, fixed: None };
    Arc::clone(&rig.fleet).settle_waiting(words).await.expect("words are not refused");

    let talk = rig.fleet.operator_talk_wire().await.expect("read");
    assert_eq!(
        talk.lines.iter().map(|one| (one.from_person, one.text.as_str(), one.about.as_deref())).collect::<Vec<_>>(),
        [(true, "why this one?", Some(arrival.as_str())), (false, "It would start the rest as a Job.", Some(arrival.as_str()))]
    );
    assert_eq!(suggestions(&rig, &id).await.len(), 1, "the row stays open");
    assert!(said(&rig, &id).await.iter().all(|one| !one.contains("started what is left")), "nothing is done by words");
}
