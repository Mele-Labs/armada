//! The claim, written before the code: **an Autopilot is started for a project the way a Job is, and
//! each Job it starts is for that project.** The start is refused as `start_session` refuses a
//! project, before anything is kept. The call is the fake.

use std::sync::Arc;

use api::{Autopilots, Projects};
use config::Manifest;
use ipc::{AutopilotSettings, AutopilotWork, ManifestId, NewProject, ProjectMember};
use store::KeptBugReport;
use testkit::{FakeJudge, FakeTracker, FakeTrackers};

use super::projects::code_of;
use super::session_host::{rig_fitted, Rig};

const SURE: &str = "issue: R-AAAAAA\nworkflow: bug\nsure: yes\nbecause: plainly covered";
const UNSURE: &str = "issue: R-AAAAAA\nworkflow: bug\nsure: no\nbecause: may need a decision";

/// A rig on the armada repository answering `answer` to the picker, which offers one reported bug.
async fn rig_picking(answer: &str) -> Rig {
    let judge = Arc::new(FakeJudge::saying(answer));
    let tracker = Arc::new(FakeTracker::listing(vec![]));
    let rig = rig_fitted(move |fittings| {
        fittings.judge = judge;
        fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker));
        fittings.starting().workflows = [super::daemon::workflow_named("bug")].into_iter().map(|one| (one.id().clone(), one)).collect();
        fittings.starting().manifest = Manifest::parse(std::path::Path::new("armada.yml"), "version: 1\nid: armada\n").expect("loads");
    });
    let report = KeptBugReport {
        id: "01R00000000000000000AAAAAA".to_string(),
        session_id: "reporter".to_string(),
        job_title: String::new(),
        title: "Open one".to_string(),
        what: "It broke.".to_string(),
        steps: Vec::new(),
        errors: Vec::new(),
        logs: Vec::new(),
        at: "2026-11-03T08:00:00.000Z".to_string(),
        kept: None,
        fixed: None,
    };
    rig.fleet.store().lock().await.keep_bug_report(&report).expect("kept");
    rig
}

fn settings(project: Option<String>) -> AutopilotSettings {
    AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 1,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        weekly_cap_percent: None,
        on_autopilot: AutopilotWork { backlog: false, running: false, reports: true },
        decisions: Default::default(),
        project,
    }
}

async fn project_of(rig: &Rig, name: &str, members: &[&str]) -> String {
    let members = members.iter().map(|id| ProjectMember { manifest_id: ManifestId::carried(*id), folder: None }).collect();
    let made = rig.fleet.add_project(NewProject { name: name.to_string(), members: Some(members), place: None, values: None }).await.expect("added");
    made.projects.last().expect("made").id.clone()
}

async fn job_project(rig: &Rig, job: &str) -> Option<String> {
    rig.fleet.store().lock().await.hosting(job).expect("read").expect("hosted").project
}

#[tokio::test]
async fn an_autopilot_started_for_a_project_starts_its_jobs_for_it() {
    let rig = rig_picking(SURE).await;
    let id = project_of(&rig, "Shop", &[rig.manifest.as_str()]).await;
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(Some(id.clone()))).await.expect("started");

    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 1);

    let pick = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").picks.remove(0);
    let job = pick.session_id.expect("a Job started");
    assert_eq!(job_project(&rig, job.as_str()).await.as_deref(), Some(id.as_str()), "the Job records the project");
}

#[tokio::test]
async fn a_set_aside_pick_dispatched_later_is_for_the_project_too_and_an_autopilot_for_none_starts_jobs_for_none() {
    let rig = rig_picking(UNSURE).await;
    let id = project_of(&rig, "Shop", &[rig.manifest.as_str()]).await;
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(Some(id.clone()))).await.expect("started");
    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 0);
    let aside = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("read").picks.remove(0);

    let dispatched = Arc::clone(&rig.fleet).dispatch_set_aside(rig.manifest.clone(), aside.pick_id).await.expect("dispatched");

    let job = dispatched.picks[0].session_id.clone().expect("a Job started");
    assert_eq!(job_project(&rig, job.as_str()).await.as_deref(), Some(id.as_str()));

    let plain = rig_picking(SURE).await;
    plain.fleet.start_autopilot(plain.manifest.clone(), settings(None)).await.expect("started");
    assert_eq!(plain.fleet.autopilot_pass().await.expect("passed"), 1);
    let pick = plain.fleet.get_autopilot(plain.manifest.clone()).await.expect("read").picks.remove(0);
    assert_eq!(job_project(&plain, pick.session_id.expect("a Job").as_str()).await, None);
}

#[tokio::test]
async fn an_autopilot_for_an_unknown_project_is_a_404_and_for_one_that_lacks_the_repository_a_422_and_neither_starts() {
    let rig = rig_picking(SURE).await;
    let elsewhere = project_of(&rig, "Elsewhere", &["some-other-repository"]).await;

    let unknown = rig.fleet.start_autopilot(rig.manifest.clone(), settings(Some("nope".into()))).await.expect_err("refused");
    assert_eq!(code_of(unknown), (404, "fleet.no_such_project".to_string()));
    let outside = rig.fleet.start_autopilot(rig.manifest.clone(), settings(Some(elsewhere))).await.expect_err("refused");
    assert_eq!(code_of(outside), (422, "fleet.project_not_member".to_string()));

    assert!(rig.fleet.store().lock().await.autopilot(rig.manifest.as_str()).expect("read").is_none(), "nothing was kept");
}
