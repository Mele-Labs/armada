//! Work from outside a Job's own run, as Fleet hands it to the Operator and does what it rules: a
//! report linked in the store to the open issue the call names, nothing handed while the Operator
//! is off, an issue new only after the first read of its tracker, and a Later item handed over that
//! no open issue covers, filed in the repository's beads with no call.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{FromOutside, OpenIssue};
use api::Sessions;
use ipc::{LaterHow, ReportBug, SessionId, SettleLater};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTracker, FakeTrackers};

use super::backlog::{machine, manifest};
use super::session_host::{eventually, rig_fitted, Rig};
use crate::operator::{Answering, Calling};
use crate::CallFailed;

const SCROLLBACK: &str = "Terminal scrollback is lost when the pane redraws";

async fn switched(rig: &Rig, key: &str, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn on(rig: &Rig) {
    switched(rig, "features.operator", true).await;
    switched(rig, "features.operatorDispatch", true).await;
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

/// A rig whose tracker lists issue #5, about the terminal's scrollback.
fn rig_listing() -> Rig {
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(5, SCROLLBACK)]));
    rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker)))
}

async fn reported(rig: &Rig, id: &SessionId, title: &str) -> String {
    let report = ReportBug {
        title: title.into(),
        what: "The pane redraws and the scrollback is gone.".into(),
        steps: vec!["Resize the pane".into()],
        errors: Vec::new(),
        logs: Vec::new(),
        session_id: Some(id.clone()),
    };
    rig.fleet.report_bug(None, report).await.expect("reported").report_id
}

async fn rulings(rig: &Rig) -> Vec<store::KeptRuling> {
    rig.fleet.store().lock().await.operator_rulings(10).expect("read")
}

#[tokio::test]
async fn a_report_is_handed_on_the_tick_and_linked_in_the_store_to_the_open_issue_the_call_names() {
    let rig = rig_listing();
    on(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nissue: #5\nbecause: the same fault"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start_titled(Some("Build the pane")).await;
    let report = reported(&rig, &id, "Terminal scrollback lost on redraw").await;

    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.bug_report_issue(&report).expect("read").is_some() }).await;
    assert_eq!(rig.fleet.store().lock().await.bug_report_issue(&report).expect("read").as_deref(), Some("#5"));
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in [
        "An agent reported a bug, from its own agent: \"Terminal scrollback lost on redraw\"",
        &format!("Open issues it may be: #5 \"{SCROLLBACK}\""),
        // No live Job shares its words, so existing job is not offered.
        "The ways it is offered: 1. link to an open issue; 2. new job; 3. raise\n",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    let log = rulings(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str(), log[0].issue.as_deref()), ("report", "link", Some("#5")));

    // Handed again on the next tick, it is not ruled twice.
    rig.fleet.sources_handed().await;
    assert_eq!(call.asked().len(), 1);
}

#[tokio::test]
async fn a_report_kept_while_the_operator_is_off_is_never_handed_even_once_it_is_on() {
    let rig = rig_listing();
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = rig.start_titled(Some("Build the pane")).await;
    let report = reported(&rig, &id, "Terminal scrollback lost on redraw").await;

    rig.fleet.sources_handed().await;
    on(&rig).await;
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(counted.0.load(Ordering::SeqCst), 0);
    assert!(rulings(&rig).await.is_empty());
    assert_eq!(rig.fleet.store().lock().await.bug_report_issue(&report).expect("read"), None);
}

fn listed(issues: &[(u64, &str)]) -> Vec<OpenIssue> {
    issues.iter().map(|(number, title)| FakeTracker::issue(*number, title)).collect()
}

#[tokio::test]
async fn an_issue_is_new_only_after_the_first_read_and_is_raised_with_a_read() {
    let rig = rig_listing();
    on(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nbecause: a new fault in the usage meter, and no Job works on it"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let seat = rig.fleet.operator_seat();
    let manifest = rig.manifest.as_str();

    seat.issues_read(manifest, &listed(&[(5, SCROLLBACK)]));
    rig.fleet.sources_handed().await;
    let mut assigned = FakeTracker::issue(8, "Somebody's own work");
    assigned.assignees = vec![FromOutside::verbatim("someone")];
    let mut read = listed(&[(5, SCROLLBACK), (7, "Usage meter shows yesterday's tokens")]);
    read.push(assigned);
    seat.issues_read(manifest, &read);
    rig.fleet.sources_handed().await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in ["A new issue was filed, from the tracker: \"Usage meter shows yesterday's tokens\"", "The ways it is offered: 1. raise\n"] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(!asked[0].contains(SCROLLBACK) && !asked[0].contains("Somebody's own work"), "{}", asked[0]);
    let log = rulings(&rig).await;
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str(), log[0].by.as_str()), ("issue", "raise", "operator"));
}

fn handing(id: &SessionId, which: &str) -> SettleLater {
    SettleLater { id: which.into(), how: LaterHow::Handed, issue: None, reason: None, session_id: Some(id.clone()), misread: false }
}

#[tokio::test]
async fn a_later_item_handed_over_is_refused_while_the_operator_is_off_and_filed_in_beads_with_no_call_once_it_is_on() {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let rig = machine("north", &copy).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = rig.start_titled(Some("Build the pane")).await;
    let sentence = "I will move the fixture loader into its own crate.";
    assert!(rig.fleet.later_kept(id.as_str(), &[sentence.to_string()], "2026-10-21T10:00:00.000Z").await);

    let refused = rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect_err("refused");
    assert!(format!("{refused:?}").contains("handed needs the Operator on"), "{refused:?}");

    on(&rig).await;
    rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect("handed");
    rig.fleet.sources_handed().await;

    eventually(|| async { rig.fleet.store().lock().await.later_handed().expect("read").is_empty() }).await;
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "ruled with no call");
    let log = rulings(&rig).await;
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str()), ("later", "file"));
    let open = rig.fleet.open_issues(&manifest(&rig)).await.expect("read");
    assert!(open.iter().any(|one| one.title.as_written() == sentence), "{open:#?}");
}
