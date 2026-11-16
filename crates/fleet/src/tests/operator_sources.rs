//! Work from outside a Job's own run, with the Operator's session not running: a report is told to nobody and
//! stays where it is, an issue is new only after the first read of its tracker, and a Later item handed over
//! that no open issue covers is refused whatever the Operator is, so the Job settles it itself. A red main
//! reaches the Autopilot that finds it, and a red a Job already holds never does.

use std::sync::Arc;

use adapter_traits::{FromOutside, OpenIssue};
use api::Sessions;
use ipc::{AutopilotDecision, LaterHow, ReportBug, SessionId, SettleLater};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTracker, FakeTrackers};

use super::autopilot_decisions::{rig_calling, started, SURE_OF_THE_FIRST};
use super::autopilot_decisions_repository::{jobs_titled, red_main};
use super::backlog::{machine, manifest};
use super::session_host::{rig_fitted, Rig};

const SCROLLBACK: &str = "Terminal scrollback is lost when the pane redraws";

pub(super) async fn switched(rig: &Rig, key: &str, on: bool) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Bool(on)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

pub(super) async fn on(rig: &Rig) {
    switched(rig, "features.operator", true).await;
}

/// A rig over beads, with the Operator on.
pub(super) async fn filing() -> Rig {
    let copy = Arc::new(FakeBeads::on(&FakeBeadsRemote::new()));
    let rig = machine("north", &copy).await;
    on(&rig).await;
    rig
}

/// A Job asked `words`, its first message.
pub(super) async fn asked(rig: &Rig, words: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, words).await;
    id
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
async fn a_report_is_told_to_nobody_while_the_operators_session_is_not_running_and_stays_where_it_is() {
    let rig = rig_listing();
    on(&rig).await;
    let id = rig.start_titled(Some("Build the pane")).await;
    let report = reported(&rig, &id, "Terminal scrollback lost on redraw").await;

    rig.fleet.sources_handed().await;
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(rulings(&rig).await.is_empty());
    assert_eq!(rig.fleet.store().lock().await.bug_report_issue(&report).expect("read"), None);
    assert_eq!(jobs_titled(&rig, "Terminal scrollback lost on redraw").await, 0, "no Job starts from a report");
}

fn listed(issues: &[(u64, &str)]) -> Vec<OpenIssue> {
    issues.iter().map(|(number, title)| FakeTracker::issue(*number, title)).collect()
}

#[tokio::test]
async fn an_issue_is_new_only_after_the_first_read_and_one_assigned_is_not_new() {
    let rig = rig_listing();
    on(&rig).await;
    let seat = rig.fleet.operator_seat();
    let manifest = rig.manifest.as_str();

    seat.issues_read(manifest, &listed(&[(5, SCROLLBACK)]));
    let mut assigned = FakeTracker::issue(8, "Somebody's own work");
    assigned.assignees = vec![FromOutside::verbatim("someone")];
    let mut read = listed(&[(5, SCROLLBACK), (7, "Usage meter shows yesterday's tokens")]);
    read.push(assigned);
    seat.issues_read(manifest, &read);
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(rulings(&rig).await.is_empty(), "nothing is ruled, and nothing starts");
    assert_eq!(jobs_titled(&rig, "Usage meter shows yesterday's tokens").await, 0);
}

fn handing(id: &SessionId, which: &str) -> SettleLater {
    SettleLater { id: which.into(), how: LaterHow::Handed, issue: None, reason: None, session_id: Some(id.clone()), misread: false }
}

/// A rig over beads with Job "Build the pane", which keeps one Later item, and its id.
async fn keeping_one(sentence: &str) -> (Rig, SessionId) {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let rig = machine("north", &copy).await;
    let id = rig.start_titled(Some("Build the pane")).await;
    assert!(rig.fleet.later_kept(id.as_str(), &[sentence.to_string()], "2026-10-21T10:00:00.000Z").await);
    (rig, id)
}

#[tokio::test]
async fn a_later_item_cannot_be_handed_over_with_the_operator_on_or_off_and_the_job_settles_it_itself() {
    let sentence = "I will move the fixture loader into its own crate.";
    let (rig, id) = keeping_one(sentence).await;

    let refused = rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect_err("refused off");
    assert!(format!("{refused:?}").contains("handed is not a way to settle it"), "{refused:?}");

    on(&rig).await;
    let refused = rig.fleet.settle_later(None, handing(&id, "later-1")).await.expect_err("refused on, as it is off");
    assert!(format!("{refused:?}").contains("handed is not a way to settle it"), "{refused:?}");
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(rulings(&rig).await.is_empty(), "nothing in the Log");
    let open = rig.fleet.open_issues(&manifest(&rig)).await.expect("read");
    assert!(!open.iter().any(|one| one.title.as_written() == sentence), "nothing is filed: {open:#?}");

    let dropped = SettleLater { how: LaterHow::Dropped, ..handing(&id, "later-1") };
    rig.fleet.settle_later(None, dropped).await.expect("the Job settles it itself");
}

/// The owner's ruling of 13 Nov 2026: a red main's fix Job starts at once, with no go-ahead to press.
#[tokio::test]
async fn a_red_main_starts_its_fix_at_once_with_the_operator_on_and_asks_nothing() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    switched(&rig, "features.operator", true).await;
    let commit = "b".repeat(40);
    red_main(&rig, &commit).await;
    let item = format!("red:{}:{commit}", rig.manifest.as_str());

    assert_eq!(rig.fleet.reds_fixed().await, 1);
    assert_eq!(jobs_titled(&rig, "Fix main").await, 1);
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    rig.fleet.red_item_offered(&main).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let page = rig.fleet.operator_page_wire().await.expect("page");
    assert!(!page.turn.iter().any(|one| one.item.id == item), "no go-ahead for a red a Job has");
    assert!(judge.asked().is_empty(), "no call is made for a red");
}

#[tokio::test]
async fn a_red_main_a_job_already_holds_starts_no_second_fix() {
    let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
    switched(&rig, "features.operator", true).await;
    let holder = rig.start().await;
    started(&rig, &[AutopilotDecision::Red]).await;
    red_main(&rig, &"b".repeat(40)).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    rig.fleet.recorded_session_take(&main, holder.as_str(), store::TakenHow::Took, None).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    rig.fleet.decision_pass().await.expect("passed");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(rulings(&rig).await.is_empty(), "nothing in the Log");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);
}
