//! Sub projects, written before the code: **the Jobs he picked are kept under his name with the features whose live Jobs
//! held one of them at the press**, refused where the name, the Jobs or their project is wrong, listed oldest first and
//! dropped on Ungroup. `docs/concepts/feature-catalog.md`, *Sub projects*.

use api::{Features, Refusal};
use ipc::{SessionId, StartSubProject, UngroupSubProject};

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, held_slot, seeded, wrote, VOTING};

fn start(project: &str, name: &str, jobs: &[&SessionId]) -> StartSubProject {
    StartSubProject { project: project.to_string(), name: name.to_string(), jobs: jobs.iter().map(|job| (*job).clone()).collect() }
}

fn code(refusal: Refusal) -> (&'static str, String) {
    match refusal {
        Refusal::Unacceptable(why) => ("422", why.code),
        Refusal::NotFound(why) => ("404", why.code),
        Refusal::IllegalMove(why) => ("409", why.code),
        other => panic!("refused otherwise: {other:?}"),
    }
}

/// A project with its features seeded and one Job at work on `voting-on-takes`.
async fn at_work(rig: &Rig) -> (String, SessionId) {
    let project = a_project(rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let job = an_agent_for(rig, &project).await;
    wrote(rig, &job, "crates/pages/voting/ranks.rs").await;
    held_slot(rig, &job, "4").await;
    (project, job)
}

#[tokio::test]
async fn a_sub_project_keeps_his_jobs_in_his_order_and_the_features_they_are_on_now() {
    let rig = rig(None);
    let (project, voting) = at_work(&rig).await;
    let idle = an_agent_for(&rig, &project).await;

    let kept = rig.fleet.start_sub_project(start(&project, "  Rank votes ", &[&idle, &voting])).await.expect("started");
    assert_eq!(kept.name, "Rank votes", "trimmed");
    assert_eq!(kept.project, project);
    assert_eq!(kept.jobs, [idle, voting], "in his order");
    assert_eq!(kept.features, [VOTING], "page-actions has no Job on it");
    assert!(!kept.id.is_empty());

    let listed = rig.fleet.list_sub_projects(Some(project.clone())).await.expect("listed");
    assert_eq!(listed.sub_projects, [kept]);
}

#[tokio::test]
async fn a_sub_project_with_no_name_or_no_jobs_is_refused_and_nothing_is_kept() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let no_name = rig.fleet.start_sub_project(start(&project, "   ", &[&job])).await.expect_err("refused");
    assert_eq!(code(no_name), ("422", "fleet.sub_project_no_name".to_string()));
    let no_jobs = rig.fleet.start_sub_project(start(&project, "Rank votes", &[])).await.expect_err("refused");
    assert_eq!(code(no_jobs), ("422", "fleet.sub_project_no_jobs".to_string()));
    assert!(rig.fleet.list_sub_projects(None).await.expect("listed").sub_projects.is_empty());
}

#[tokio::test]
async fn a_job_fleet_does_not_hold_is_refused_404() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let unknown = SessionId::carried("no-such-job");
    let refused = rig.fleet.start_sub_project(start(&project, "Rank votes", &[&job, &unknown])).await.expect_err("refused");
    assert_eq!(code(refused), ("404", "fleet.no_such_session".to_string()));
    assert!(rig.fleet.list_sub_projects(None).await.expect("listed").sub_projects.is_empty());
}

#[tokio::test]
async fn jobs_of_two_projects_are_refused_409() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let other = a_project(&rig).await;
    let elsewhere = an_agent_for(&rig, &other).await;
    let refused = rig.fleet.start_sub_project(start(&project, "Rank votes", &[&job, &elsewhere])).await.expect_err("refused");
    assert_eq!(code(refused), ("409", "fleet.sub_project_across_projects".to_string()));
    assert!(rig.fleet.list_sub_projects(None).await.expect("listed").sub_projects.is_empty());
}

#[tokio::test]
async fn sub_projects_are_listed_by_project_oldest_first_and_every_project_without_one() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let other = a_project(&rig).await;
    let elsewhere = an_agent_for(&rig, &other).await;
    let first = rig.fleet.start_sub_project(start(&project, "First", &[&job])).await.expect("started");
    let second = rig.fleet.start_sub_project(start(&project, "Second", &[&job])).await.expect("started");
    let theirs = rig.fleet.start_sub_project(start(&other, "Theirs", &[&elsewhere])).await.expect("started");
    assert!(theirs.features.is_empty(), "the other project holds no feature");

    let names = |listed: ipc::SubProjects| listed.sub_projects.into_iter().map(|one| one.name).collect::<Vec<_>>();
    assert_eq!(names(rig.fleet.list_sub_projects(Some(project.clone())).await.expect("listed")), [first.name.clone(), second.name.clone()]);
    assert_eq!(names(rig.fleet.list_sub_projects(Some(other.clone())).await.expect("listed")), [theirs.name.clone()]);
    assert_eq!(names(rig.fleet.list_sub_projects(None).await.expect("listed")), [first.name, second.name, theirs.name]);
}

#[tokio::test]
async fn ungroup_drops_the_sub_project_and_leaves_its_jobs_and_an_unknown_id_is_404() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = rig.fleet.start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");

    rig.fleet.ungroup_sub_project(UngroupSubProject { id: kept.id.clone() }).await.expect("ungrouped");
    assert!(rig.fleet.list_sub_projects(Some(project.clone())).await.expect("listed").sub_projects.is_empty());
    let library = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    let row = library.areas[0].features.iter().find(|one| one.slug == VOTING).expect("the feature");
    assert_eq!(row.jobs_now.iter().map(|one| one.session_id.clone()).collect::<Vec<_>>(), [job], "the Job is untouched");

    let again = rig.fleet.ungroup_sub_project(UngroupSubProject { id: kept.id }).await.expect_err("refused");
    assert_eq!(code(again), ("404", "fleet.no_such_sub_project".to_string()));
}
