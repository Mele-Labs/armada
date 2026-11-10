//! Work from outside a Job's own run, as Fleet hands it to the Operator and does what it rules: a
//! report linked in the store to the open issue the call names, nothing handed while the Operator
//! is off, an issue new only after the first read of its tracker, and a Later item handed over that
//! no open issue covers is refused whatever the Operator is, so the Job settles it itself. A red main
//! reaches it through the read that finds it, and a red a Job already holds never does.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{FromOutside, OpenIssue};
use api::Sessions;
use ipc::{AutopilotDecision, LaterHow, ReportBug, SessionId, SettleLater};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTracker, FakeTrackers};

use super::autopilot_decisions::{rig_calling, started, SURE_OF_THE_FIRST};
use super::autopilot_decisions_repository::{jobs_titled, red_main};
use super::backlog::{machine, manifest};
use super::session_host::{eventually, rig_fitted, Rig};
use crate::operator::{Answering, Calling};
use crate::CallFailed;

const SCROLLBACK: &str = "Terminal scrollback is lost when the pane redraws";

pub(super) async fn switched(rig: &Rig, key: &str, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

pub(super) async fn on(rig: &Rig) {
    switched(rig, "features.operator", true).await;
}

/// A call that answers each question with the next of its answers, and keeps what it was asked.
#[derive(Default)]
pub(super) struct Answers {
    answers: Mutex<VecDeque<String>>,
    asked: Mutex<Vec<String>>,
}

impl Answers {
    pub(super) fn with(answers: &[&str]) -> Arc<Answers> {
        Arc::new(Answers { answers: Mutex::new(answers.iter().map(|one| one.to_string()).collect()), asked: Mutex::new(Vec::new()) })
    }

    pub(super) fn asked(&self) -> Vec<String> {
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

pub(super) async fn rulings(rig: &Rig) -> Vec<store::KeptRuling> {
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

/// A rig over beads with Job "Build the pane", which keeps one Later item, and its id.
async fn keeping_one(sentence: &str) -> (Rig, SessionId, Arc<Counting>) {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let rig = machine("north", &copy).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = rig.start_titled(Some("Build the pane")).await;
    assert!(rig.fleet.later_kept(id.as_str(), &[sentence.to_string()], "2026-10-21T10:00:00.000Z").await);
    (rig, id, counted)
}

#[tokio::test]
async fn a_later_item_cannot_be_handed_over_with_the_operator_on_or_off_and_the_job_settles_it_itself() {
    let sentence = "I will move the fixture loader into its own crate.";
    let (rig, id, counted) = keeping_one(sentence).await;

    let refused = rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect_err("refused off");
    assert!(format!("{refused:?}").contains("handed is not a way to settle it"), "{refused:?}");

    on(&rig).await;
    let refused = rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect_err("refused on, as it is off");
    assert!(format!("{refused:?}").contains("handed is not a way to settle it"), "{refused:?}");
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "no call");
    assert!(rulings(&rig).await.is_empty(), "nothing in the Log");
    let open = rig.fleet.open_issues(&manifest(&rig)).await.expect("read");
    assert!(!open.iter().any(|one| one.title.as_written() == sentence), "nothing is filed: {open:#?}");

    let dropped = SettleLater { how: LaterHow::Dropped, ..handing(&id, "later-1") };
    rig.fleet.settle_later(None, dropped).await.expect("the Job settles it itself");
}

#[tokio::test]
async fn a_red_main_autopilot_finds_is_asked_once_and_a_fix_starts_only_on_yes() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    switched(&rig, "features.operator", true).await;
    let call = Answers::with(&[]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    started(&rig, &[AutopilotDecision::Red]).await;
    let commit = "b".repeat(40);
    red_main(&rig, &commit).await;
    let item = format!("red:{}:{commit}", rig.manifest.as_str());
    let in_turn = || async { rig.fleet.operator_page_wire().await.expect("page").turn.into_iter().filter(|one| one.item.id == item).collect::<Vec<_>>() };

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "the Operator has it, not Autopilot's own start");
    eventually(|| async { in_turn().await.len() == 1 }).await;
    assert!(judge.asked().is_empty(), "Autopilot makes no call of its own for a red");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0, "nothing starts until he says yes");
    let log = rulings(&rig).await;
    assert!(log.iter().all(|row| row.outcome != "new_job" && row.outcome != "proposed_new_job"), "the ruling does not ask a second time: {log:#?}");

    rig.fleet.decision_pass().await.expect("passed");
    rig.fleet.decision_pass().await.expect("passed");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let waiting = in_turn().await;
    assert_eq!(waiting.len(), 1, "one ask for each red commit");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);

    let press = ipc::operator_page::OperatorAnswerPress { id: waiting[0].id.clone(), answer: ipc::operator_page::OperatorAnswer::Yes, then: None };
    let answered = rig.fleet.operator_answered(press).await.expect("answered");
    assert!(answered.started.is_some());
    assert_eq!(jobs_titled(&rig, "Fix main").await, 1);
}

#[tokio::test]
async fn a_red_main_a_job_already_holds_is_never_handed_to_the_operator() {
    let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
    switched(&rig, "features.operator", true).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let holder = rig.start().await;
    started(&rig, &[AutopilotDecision::Red]).await;
    red_main(&rig, &"b".repeat(40)).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    rig.fleet.recorded_session_take(&main, holder.as_str(), store::TakenHow::Took, None).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    rig.fleet.decision_pass().await.expect("passed");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "no call");
    assert!(rulings(&rig).await.is_empty(), "nothing in the Log");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);
}
