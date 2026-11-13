//! The share filter, one test for each row of what crosses to his other Macs: a Job of an unshared project, or
//! of none, sends nothing and is answered as a Job that does not exist; the same Job sends once its project is
//! shared, and stops the moment it is turned off.

use api::{HostedSessions, Projects, Refusal, Sessions};
use ipc::{EditProject, Event, JobThreadChanged, NewProject, ProjectMember, ReportBug, SessionId, StartSession};

use super::session_host::{rig_placing, Rig};

/// A project of the rig's repository, unshared as every new one is.
async fn project(rig: &Rig, name: &str) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: name.to_string(), members, place: None, values: None };
    let projects = rig.fleet.add_project(new).await.expect("added").projects;
    projects.into_iter().find(|one| one.name == name).expect("the project").id
}

async fn shared(rig: &Rig, project: &str, on: bool) {
    let edit = EditProject { shared_with_macs: Some(on), ..Default::default() };
    rig.fleet.edit_project(project.to_string(), edit).await.expect("edited");
}

async fn job_in(rig: &Rig, project: Option<&str>) -> SessionId {
    let start = StartSession {
        project: project.map(str::to_string),
        manifest_id: rig.manifest.clone(),
        title: Some("Build the pane".to_string()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    rig.fleet.start_session(start).await.expect("started").id
}

fn thread_moved(job: &SessionId) -> Event {
    Event::JobThreadChanged(JobThreadChanged { session_id: job.clone(), from: "r1".to_string() })
}

/// What a refusal says, with the id it names taken out, so two refusals can be compared whole.
fn said(refused: &Refusal, named: &str) -> (u16, String, String) {
    (refused.status(), refused.error().code.clone(), refused.error().message.replace(named, "<id>"))
}

#[tokio::test]
async fn a_project_starts_unshared_and_the_switch_is_served_as_set() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;

    let listed = rig.fleet.list_projects().await.expect("listed");
    assert!(!listed.projects.iter().any(|one| one.shared_with_macs), "every project starts off: {listed:?}");

    shared(&rig, &shop, true).await;
    let listed = rig.fleet.list_projects().await.expect("listed");
    assert!(listed.projects.iter().find(|one| one.id == shop).expect("shop").shared_with_macs);

    shared(&rig, &shop, false).await;
    let listed = rig.fleet.list_projects().await.expect("listed");
    assert!(!listed.projects.iter().find(|one| one.id == shop).expect("shop").shared_with_macs);
}

#[tokio::test]
async fn a_jobs_row_thread_and_live_changes_cross_only_while_its_project_is_shared() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;

    assert!(rig.fleet.of_job_for_other_mac(&in_shop, "row").await.is_none(), "an unshared project's Job stays");
    assert!(rig.fleet.event_for_other_mac(thread_moved(&in_shop)).await.is_none(), "so does its thread");
    assert!(rig.fleet.of_job_for_other_mac(&in_none, "row").await.is_none(), "a Job in no project stays");

    shared(&rig, &shop, true).await;
    assert_eq!(rig.fleet.of_job_for_other_mac(&in_shop, "row").await.map(|one| one.sent()), Some("row"));
    let crossed = rig.fleet.event_for_other_mac(thread_moved(&in_shop)).await.expect("its thread crosses").sent();
    assert_eq!(crossed, thread_moved(&in_shop));
    assert!(rig.fleet.event_for_other_mac(thread_moved(&in_none)).await.is_none(), "a Job in no project still stays");

    shared(&rig, &shop, false).await;
    assert!(rig.fleet.event_for_other_mac(thread_moved(&in_shop)).await.is_none(), "turned off, it stops at once");
}

#[tokio::test]
async fn an_act_on_an_unshared_job_is_answered_as_an_act_on_no_job() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;
    let nobody = SessionId::carried("01NOSUCHJOB0000000000000000");

    let missing = rig.fleet.act_from_other_mac(&nobody).await.expect_err("no such Job");
    assert_eq!(missing.status(), 404);
    let unshared = rig.fleet.act_from_other_mac(&in_shop).await.expect_err("unshared");
    let unprojected = rig.fleet.act_from_other_mac(&in_none).await.expect_err("in no project");
    assert_eq!(said(&unshared, in_shop.as_str()), said(&missing, nobody.as_str()), "the refusal says nothing");
    assert_eq!(said(&unprojected, in_none.as_str()), said(&missing, nobody.as_str()), "the refusal says nothing");

    shared(&rig, &shop, true).await;
    rig.fleet.act_from_other_mac(&in_shop).await.expect("a shared Job takes the act");
}

#[tokio::test]
async fn a_job_from_another_mac_starts_only_in_a_shared_project() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;

    let unshared = rig.fleet.start_from_other_mac(Some(&shop)).expect_err("unshared");
    let missing = rig.fleet.start_from_other_mac(Some("no-such-project")).expect_err("no such project");
    assert_eq!(said(&unshared, &shop), said(&missing, "no-such-project"), "an unshared project is not named");
    assert_eq!(rig.fleet.start_from_other_mac(None).expect_err("in no project").status(), 404);

    shared(&rig, &shop, true).await;
    rig.fleet.start_from_other_mac(Some(&shop)).expect("a shared project takes the start");
}

#[tokio::test]
async fn a_message_crosses_only_between_jobs_whose_projects_are_both_shared() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let sender = job_in(&rig, Some(&shop)).await;
    let receiver = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;

    assert!(rig.fleet.message_to_other_mac(&sender, "hello").await.is_none(), "an unshared sender sends nothing");
    assert!(rig.fleet.message_to_other_mac(&in_none, "hello").await.is_none(), "nor does a Job in no project");
    assert!(rig.fleet.act_from_other_mac(&receiver).await.is_err(), "an unshared receiver cannot be reached");

    shared(&rig, &shop, true).await;
    assert!(rig.fleet.message_to_other_mac(&sender, "hello").await.is_some());
    rig.fleet.act_from_other_mac(&receiver).await.expect("a shared receiver is reached");
}

#[tokio::test]
async fn a_bug_report_follows_the_job_that_filed_it() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let quiet = project(&rig, "Quiet").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_quiet = job_in(&rig, Some(&quiet)).await;
    let in_none = job_in(&rig, None).await;
    for job in [&in_shop, &in_quiet, &in_none] {
        let report = ReportBug {
            title: "Refresh drops the lease".into(),
            what: "It quotes private work.".into(),
            steps: Vec::new(),
            errors: Vec::new(),
            logs: Vec::new(),
            session_id: Some(job.clone()),
        };
        rig.fleet.report_bug(None, report).await.expect("reported");
    }
    shared(&rig, &shop, true).await;

    let reports = rig.fleet.list_bug_reports().await.expect("listed");
    let crossing = rig.fleet.bug_reports_for_other_mac(reports.clone()).await.sent();
    let from: Vec<&str> = crossing.reports.iter().map(|one| one.session_id.as_str()).collect();
    assert_eq!(from, [in_shop.as_str()], "only the shared Job's report crosses");

    let Some(Event::BugReportsChanged(published)) = rig.fleet.event_for_other_mac(Event::BugReportsChanged(reports)).await.map(|one| one.sent())
    else {
        panic!("bug_reports.changed crosses, cut");
    };
    assert_eq!(published, crossing);
}

#[tokio::test]
async fn only_shared_projects_are_named_to_another_mac() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    project(&rig, "Quiet").await;
    shared(&rig, &shop, true).await;

    let projects = rig.fleet.list_projects().await.expect("listed");
    assert_eq!(projects.projects.len(), 2);
    let crossing = rig.fleet.projects_for_other_mac(projects.clone()).sent();
    let named: Vec<&str> = crossing.projects.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(named, [shop.as_str()]);
    assert!(crossing.loose.is_empty() && crossing.left_out.is_empty(), "tasks in no project and unread files stay");

    let Some(Event::ProjectsChanged(published)) = rig.fleet.event_for_other_mac(Event::ProjectsChanged(projects)).await.map(|one| one.sent())
    else {
        panic!("projects.changed crosses, cut");
    };
    assert_eq!(published, crossing);
}
