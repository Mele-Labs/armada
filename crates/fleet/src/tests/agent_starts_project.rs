//! The claim, written before the code: **a Job an agent starts is for its starter's project.** It takes that
//! project's values as any Job does, its repository becomes a whole member where it was not one, and a starter in
//! no project starts its Job in none. `docs/concepts/project.md`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Projects, Sessions};
use ipc::{Event, ForkFrom, ForkThisSession, MoveSession, NewProject, ProjectMember, SessionId, StartSession, ValuesSet, HANDOFF_YES};

use super::agent_starts::{begins, caller, rig, Rig};
use super::session_handoff::asked_and_answered;
use super::projects::a_second_repository;
use super::projects_kept::file;

const BRIEF: &str = "Read crates/reader and say why it drops lines.";

/// A project of the rig's repository whose review model is `project-model`, and its id.
async fn a_project(rig: &Rig) -> String {
    let values = ValuesSet { review_model: Some("project-model".into()), ..ValuesSet::default() };
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: Some(values) };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

/// The agent that makes the calls, started for `project`.
async fn an_agent_for(rig: &Rig, project: &str) -> SessionId {
    let start = StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some("The agent".into()),
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
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.send(&id, "go on").await;
    rig.stand_in.init(0);
    id
}

async fn project_of(rig: &Rig, job: &SessionId) -> Option<String> {
    rig.fleet.project_of_job(job.as_str()).await
}

#[tokio::test]
async fn a_started_job_is_for_its_starters_project_and_takes_the_values_the_project_sets() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    an_agent_for(&rig, &project).await;

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", BRIEF)).await.expect("started");

    assert_eq!(project_of(&rig, &held.session_id).await.as_deref(), Some(project.as_str()));
    assert_eq!(rig.fleet.review_model_of_job(held.session_id.as_str()).await.as_deref(), Some("project-model"), "frozen at its start");
}

#[tokio::test]
async fn a_repository_the_project_does_not_hold_becomes_a_whole_member_and_the_change_is_published() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let second = a_second_repository(&rig.fleet, &rig.home);
    an_agent_for(&rig, &project).await;
    let mut watching = rig.fleet.events().subscribe();
    let mut start = begins(&rig, "Check the other", BRIEF);
    start.manifest_id = Some(second.clone());

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), start).await.expect("started");

    assert_eq!(project_of(&rig, &held.session_id).await.as_deref(), Some(project.as_str()));
    let listed = rig.fleet.list_projects().await.expect("listed").projects;
    let members: Vec<_> = listed.iter().find(|one| one.id == project).expect("the project").members.iter().map(|one| (one.manifest_id.clone(), one.folder.clone())).collect();
    assert_eq!(members, [(rig.manifest.clone(), None), (second.clone(), None)], "whole, after the first");
    let written = std::fs::read_to_string(file(&rig.home, &project)).expect("its file");
    assert!(written.contains(second.as_str()), "the file says so: {written}");
    let published = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::ProjectsChanged(projects) = delivered.event {
                        if projects.projects.iter().any(|one| one.members.iter().any(|member| member.manifest_id == second)) {
                            return;
                        }
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await;
    assert!(published.is_ok(), "a projects.changed carries the new member");
}

#[tokio::test]
async fn a_starter_in_no_project_starts_its_job_in_none() {
    let rig = rig(None);
    a_project(&rig).await;
    rig.an_agent().await;

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", BRIEF)).await.expect("started");

    assert_eq!(project_of(&rig, &held.session_id).await, None);
    assert_eq!(rig.fleet.review_model_of_job(held.session_id.as_str()).await, None, "no project's value was taken");
}

#[tokio::test]
async fn a_project_whose_file_did_not_read_starts_the_job_in_none_and_the_start_stands() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    an_agent_for(&rig, &project).await;
    std::fs::remove_file(file(&rig.home, &project)).expect("its file");
    rig.fleet.projects_reload().await;

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", BRIEF)).await.expect("not refused");

    assert_eq!(project_of(&rig, &held.session_id).await, None);
    assert_eq!(rig.record(&held.session_id).await.id, held.session_id, "the Job is there");
}

/// The agent that has spoken, which a fork is taken of, put in `project` as one started for it is.
async fn a_speaker_in(rig: &Rig, project: &str) -> (SessionId, Vec<String>) {
    let (id, rows) = rig.an_agent_that_has_spoken().await;
    let mut store = rig.fleet.store().lock().await;
    let mut hosting = store.hosting(id.as_str()).unwrap().expect("hosted");
    hosting.project = Some(project.to_string());
    store.keep_hosting(&hosting).unwrap();
    drop(store);
    (id, rows)
}

#[tokio::test]
async fn a_fork_an_agent_makes_of_its_own_job_is_for_that_jobs_project() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    a_speaker_in(&rig, &project).await;

    let held = rig
        .fleet
        .fork_this_session(caller(), ForkThisSession { title: "The other way".into(), message: "Do it with a map.".into(), at: None })
        .await
        .expect("forked");

    assert_eq!(project_of(&rig, &held.session_id).await.as_deref(), Some(project.as_str()));
}

#[tokio::test]
async fn a_fork_the_person_makes_from_bridge_is_for_the_jobs_project() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let (old, rows) = a_speaker_in(&rig, &project).await;
    let forking = StartSession {
        project: None,
        manifest_id: rig.manifest.clone(),
        title: None,
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: Some(ForkFrom { session_id: old.clone(), at: Some(rows[0].clone()) }),
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };

    let fork = rig.fleet.start_session(forking).await.expect("forked");

    assert_eq!(project_of(&rig, &fork.id).await.as_deref(), Some(project.as_str()));
}

#[tokio::test]
async fn a_job_moved_into_another_repository_stays_in_its_project_and_the_repository_becomes_a_member() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let second = a_second_repository(&rig.fleet, &rig.home);
    let (old, _) = a_speaker_in(&rig, &project).await;

    let moved = Arc::clone(&rig.fleet).move_session(MoveSession { session_id: old, manifest_id: second.clone() }).await.expect("moved");

    assert_eq!(project_of(&rig, &moved.id).await.as_deref(), Some(project.as_str()));
    let listed = rig.fleet.list_projects().await.expect("listed").projects;
    let members: Vec<_> = listed.iter().find(|one| one.id == project).expect("the project").members.iter().map(|one| one.manifest_id.clone()).collect();
    assert_eq!(members, [rig.manifest.clone(), second], "the repository it went to joined");
}

/// A pin rather than a fail-first: the hand's copy of the old Job's settings also sets the project, so this holds
/// without the project passed at the start. It fails where neither does.
#[tokio::test]
async fn a_handoff_is_for_the_old_jobs_project() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let (old, _) = a_speaker_in(&rig, &project).await;

    asked_and_answered(&rig, &old, HANDOFF_YES).await;

    let new = rig.fleet.job_now(old.as_str()).await;
    assert_ne!(new, old.as_str(), "a fresh Job took its place");
    assert_eq!(rig.fleet.project_of_job(&new).await.as_deref(), Some(project.as_str()));
}
