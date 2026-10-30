//! `report_bug`: a job says Armada itself got something wrong, Fleet keeps it in its store on this Mac,
//! and Bridge keeps or drops it and is told by `bug_reports.changed`.

use std::time::Duration;

use api::{Next, Sessions};
use ipc::{BugReports, Event, ReportBug, SessionId};

use super::session_host::{rig_placing, Rig};

fn reporting(id: Option<&SessionId>, title: &str, what: &str) -> ReportBug {
    ReportBug {
        title: title.into(),
        what: what.into(),
        steps: vec!["Press Refresh preview".into()],
        errors: vec!["fleet.slot_unleased: no lease".into()],
        logs: vec!["/tmp/fleet.log".into()],
        session_id: id.cloned(),
    }
}

/// The next `bug_reports.changed` the stream carries.
async fn changed(watching: &mut api::Subscription) -> BugReports {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::BugReportsChanged(reports) = delivered.event {
                        return reports;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("bug_reports.changed arrives")
}

async fn reported(rig: &Rig, id: &SessionId, title: &str) -> String {
    rig.fleet.report_bug(None, reporting(Some(id), title, "It broke.")).await.expect("reported").report_id
}

async fn kept(rig: &Rig) -> Vec<ipc::BugReport> {
    rig.fleet.list_bug_reports().await.expect("listed").reports
}

#[tokio::test]
async fn report_bug_keeps_the_report_with_the_jobs_title_and_answers_its_id() {
    let rig = rig_placing(None);
    let id = rig.start_titled(Some("Build the report call")).await;

    let answered = rig.fleet.report_bug(None, reporting(Some(&id), "Refresh drops the lease", "The slot was unleased.")).await.expect("reported");

    let reports = kept(&rig).await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    let report = &reports[0];
    assert_eq!(report.id, answered.report_id);
    assert_eq!(report.session_id, id.as_str());
    assert_eq!(report.job_title, "Build the report call");
    assert_eq!((report.title.as_str(), report.what.as_str()), ("Refresh drops the lease", "The slot was unleased."));
    assert_eq!(report.steps, ["Press Refresh preview"]);
    assert_eq!(report.errors, ["fleet.slot_unleased: no lease"]);
    assert_eq!(report.logs, ["/tmp/fleet.log"]);
    assert_eq!(report.kept, None, "the owner has not kept it");
}

#[tokio::test]
async fn report_bug_refuses_an_empty_title_or_what() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    for (title, what) in [("", "It broke."), ("  ", "It broke."), ("Refresh drops the lease", ""), ("Refresh drops the lease", " \n")] {
        let refused = rig.fleet.report_bug(None, reporting(Some(&id), title, what)).await.unwrap_err();
        assert_eq!(refused.error().code, "fleet.bug_report_refused", "{title:?} {what:?}");
        assert_eq!(refused.status(), 422);
    }
    assert!(kept(&rig).await.is_empty());
}

#[tokio::test]
async fn report_bug_refuses_a_call_that_places_no_job() {
    let rig = rig_placing(None);

    let refused = rig.fleet.report_bug(None, reporting(None, "Refresh drops the lease", "It broke.")).await.unwrap_err();

    assert_eq!(refused.status(), 422, "{refused:?}");
    assert!(kept(&rig).await.is_empty());
}

#[tokio::test]
async fn report_bug_trims_lines_and_drops_blank_ones() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let sent = ReportBug {
        title: "  Refresh drops the lease \n".into(),
        what: "\tIt broke.  ".into(),
        steps: vec!["  one ".into(), "".into(), "   ".into(), "two".into()],
        errors: vec!["\n".into(), " the error ".into()],
        logs: vec![],
        session_id: Some(id),
    };

    rig.fleet.report_bug(None, sent).await.expect("reported");

    let report = kept(&rig).await.remove(0);
    assert_eq!((report.title.as_str(), report.what.as_str()), ("Refresh drops the lease", "It broke."));
    assert_eq!(report.steps, ["one", "two"]);
    assert_eq!(report.errors, ["the error"]);
    assert!(report.logs.is_empty());
    assert_eq!(report.job_title, "", "a job with no title keeps an empty one");
}

#[tokio::test]
async fn report_bug_publishes_bug_reports_changed_with_every_report() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut watching = rig.fleet.events().subscribe();

    let first = reported(&rig, &id, "First").await;
    let told = changed(&mut watching).await;
    assert_eq!(told.reports.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [first.as_str()]);

    let second = reported(&rig, &id, "Second").await;
    let told = changed(&mut watching).await;
    assert_eq!(told.reports.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [second.as_str(), first.as_str()], "all of them, newest first");
}

#[tokio::test]
async fn keep_bug_report_sets_kept_and_publishes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let report = reported(&rig, &id, "First").await;
    let mut watching = rig.fleet.events().subscribe();

    let answered = rig.fleet.keep_bug_report(report.clone()).await.expect("kept");

    let at = answered.kept.clone().expect("kept is set");
    let told = changed(&mut watching).await;
    assert_eq!(told.reports.len(), 1);
    assert_eq!(told.reports[0].kept, Some(at.clone()));
    assert_eq!(kept(&rig).await[0].kept, Some(at.clone()));

    let again = rig.fleet.keep_bug_report(report).await.expect("kept again");
    assert_eq!(again.kept, Some(at), "the first time stands");
}

#[tokio::test]
async fn drop_bug_report_deletes_a_kept_report_and_publishes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let report = reported(&rig, &id, "First").await;
    let other = reported(&rig, &id, "Second").await;
    rig.fleet.keep_bug_report(report.clone()).await.expect("kept");
    let mut watching = rig.fleet.events().subscribe();

    let answered = rig.fleet.drop_bug_report(report.clone()).await.expect("dropped");

    assert_eq!(answered.id, report);
    let told = changed(&mut watching).await;
    assert_eq!(told.reports.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [other.as_str()]);
    assert_eq!(kept(&rig).await.len(), 1);
}

#[tokio::test]
async fn keep_and_drop_refuse_a_missing_id_as_no_such_bug_report() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let report = reported(&rig, &id, "First").await;
    let mut watching = rig.fleet.events().subscribe();

    let refused = rig.fleet.keep_bug_report("nope".into()).await.unwrap_err();
    assert_eq!((refused.status(), refused.error().code.as_str()), (404, "fleet.no_such_bug_report"));
    let refused = rig.fleet.drop_bug_report("nope".into()).await.unwrap_err();
    assert_eq!((refused.status(), refused.error().code.as_str()), (404, "fleet.no_such_bug_report"));

    rig.fleet.drop_bug_report(report.clone()).await.expect("dropped");
    changed(&mut watching).await;
    let refused = rig.fleet.drop_bug_report(report).await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.no_such_bug_report", "a second drop");
    assert!(
        tokio::time::timeout(Duration::from_millis(200), changed(&mut watching)).await.is_err(),
        "a refusal publishes nothing"
    );
}
