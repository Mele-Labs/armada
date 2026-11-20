//! The claim, written before the code: **a page the Operator's session writes under its own folder's
//! `.armada/documents/` is kept as a Job's page is, waits on him as a Job's page does, and a `start` button on it
//! that names its repository starts a Job there.** The Operator stands in no repository, so a backlog report is
//! the page it writes, and the work it proposes is started from it. **A `close` button on it closes each issue it
//! lists in that repository's tracker at his press, and tells the Operator what was closed; a tracker that cannot
//! close has the button named as one that will not run when the page is kept.**

use std::sync::Arc;

use api::HostedSessions;
use ipc::{PageAct, PressDid, PressPageAction, SessionId, SessionRow};
use testkit::{FakeTracker, FakeTrackers, IssueWritten};

use super::documents::{record_of, revisions_of, waits_on, writes};
use super::operator_session::{live, switched};
use super::page_actions::sent;
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};

/// The Operator's session with its process up, where the stand-in holds it, and its own folder.
async fn the_operator(rig: &Rig) -> (SessionId, usize, std::path::PathBuf) {
    switched(rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(rig).await.expect("on: one is started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    let at = rig.stand_in.starts().iter().position(|one| one.session == id).unwrap();
    rig.stand_in.init(at);
    let cwd = rig.fleet.store().lock().await.session(&id).expect("read").expect("a row").cwd;
    (SessionId::carried(id), at, std::path::PathBuf::from(cwd))
}

/// The Operator writes `html` as `name` in its own folder's documents, and Fleet keeps it as revision 1.
async fn its_page(rig: &Rig, id: &SessionId, at: usize, folder: &std::path::Path, name: &str, html: &str, call: &str) -> String {
    let file = folder.join(".armada/documents").join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, html).unwrap();
    let path = file.to_string_lossy().into_owned();
    writes(rig, at, &path, call, "Write", " +1").await;
    eventually(|| async { revisions_of(&record_of(rig, id).await, &path).is_some_and(|kept| kept.len() == 1) }).await;
    path
}

#[tokio::test]
async fn a_page_the_operator_writes_in_its_own_folder_is_kept_and_waits_on_him_as_a_jobs_page_does() {
    let rig = rig_placing(None);
    let (id, at, folder) = the_operator(&rig).await;
    let report = "<html><head><title>Backlog</title></head><body><h1>Backlog</h1><p>Three gaps in the native app.</p></body></html>";

    let path = its_page(&rig, &id, at, &folder, "backlog.html", report, "toolu_1").await;

    let record = record_of(&rig, &id).await;
    assert!(revisions_of(&record, &path).is_some(), "kept as a Job's page is");
    assert!(record.waiting_for.iter().all(|one| one.artifact.as_deref() != Some(path.as_str())), "a page that only informs raises no item");

    let asks = r#"<html><head><title>Backlog plan</title><meta name="armada-review" content="approve"></head><body><p>Approve the plan.</p><h2>Decision</h2></body></html>"#;
    let plan = its_page(&rig, &id, at, &folder, "plan.html", asks, "toolu_2").await;
    let record = record_of(&rig, &id).await;
    assert!(waits_on(&record, &plan), "{:?}", record.waiting_for);
}

#[tokio::test]
async fn a_start_button_naming_its_repository_starts_a_job_there_from_the_operators_page() {
    let rig = rig_placing(None);
    let (id, at, folder) = the_operator(&rig).await;
    let repository = rig.manifest.as_str();
    let button = format!(
        r#"<armada-action id="start-issues" act="start" label="Start" item="issues" repository="{repository}" brief="Build the Issues screen in the native app."></armada-action>"#
    );
    let report = format!("<html><head><title>Backlog</title></head><body><h1>Backlog</h1><table><tr><td>Issues screen</td><td>{button}</td></tr></table></body></html>");
    let path = its_page(&rig, &id, at, &folder, "backlog.html", &report, "toolu_1").await;

    let press = PressPageAction { session_id: id.clone(), artifact: path.clone(), revision: 1, action: "start-issues".into(), words: None };
    let press = Arc::clone(&rig.fleet).press_page_action(press).await.expect("pressed");

    let PressDid::Started { job } = press.did else { panic!("a Job started: {:?}", press.did) };
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the new Job").session;
    assert_eq!(record.manifest_id.as_ref().map(|one| one.as_str()), Some(repository), "in the repository the button names");
    assert!(!record.operator, "a Job, not a second Operator");
}

/// A Fleet whose repository's tracker is `tracker`.
fn rig_tracking(tracker: &Arc<FakeTracker>) -> Rig {
    let held = Arc::clone(tracker);
    rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(held)))
}

/// A backlog report whose one close button lists ARM-1 and ARM-2 in `repository`.
fn closing_report(repository: &str) -> String {
    let button = format!(r#"<armada-action id="close-done" act="close" label="Close 2 issues" repository="{repository}" issues="ARM-1 ARM-2"></armada-action>"#);
    format!("<html><head><title>Backlog</title></head><body><h1>Backlog</h1><p>Two issues the native app already covers.</p>{button}</body></html>")
}

#[tokio::test]
async fn a_close_button_closes_each_issue_it_lists_in_its_repositorys_tracker_and_tells_the_operator() {
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_tracking(&tracker);
    let (id, at, folder) = the_operator(&rig).await;
    let path = its_page(&rig, &id, at, &folder, "backlog.html", &closing_report(rig.manifest.as_str()), "toolu_1").await;
    assert!(tracker.writes().is_empty(), "nothing is closed before he presses");

    let press = PressPageAction { session_id: id.clone(), artifact: path.clone(), revision: 1, action: "close-done".into(), words: None };
    let press = Arc::clone(&rig.fleet).press_page_action(press).await.expect("pressed");

    assert_eq!(press.act, PageAct::Close);
    assert_eq!(press.did, PressDid::Closed { issues: vec!["ARM-1".into(), "ARM-2".into()] });
    let closed: Vec<String> = tracker
        .writes()
        .into_iter()
        .map(|one| match one {
            IssueWritten::Closed { reference, .. } => reference,
            other => panic!("only closes: {other:?}"),
        })
        .collect();
    assert_eq!(closed, ["ARM-1", "ARM-2"]);
    let told = sent(&rig.rows(&id).await).pop().unwrap_or_default();
    assert_eq!(told, "He pressed \"Close 2 issues\" in \"Backlog\". Closed ARM-1 and ARM-2.");
}

#[tokio::test]
async fn a_close_button_whose_tracker_cannot_close_is_named_as_one_that_will_not_run_when_the_page_is_kept() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.cannot("close", "this tracker cannot close an issue");
    let rig = rig_tracking(&tracker);
    let (id, at, folder) = the_operator(&rig).await;
    let path = its_page(&rig, &id, at, &folder, "backlog.html", &closing_report(rig.manifest.as_str()), "toolu_1").await;

    let told = || async {
        rig.rows(&id).await.into_iter().find_map(|row| match row {
            SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" => Some(text),
            _ => None,
        })
    };
    eventually(|| async { told().await.is_some() }).await;
    let told = told().await.unwrap();
    assert!(
        told.starts_with(&format!("In the page {path}, action \"Close 2 issues\" will not run: this tracker cannot close an issue.")),
        "{told}"
    );
}
