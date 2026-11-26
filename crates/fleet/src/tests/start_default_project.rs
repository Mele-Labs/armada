//! The claim, written before the code: **a start that names no project is for the one project that holds its
//! repository, where exactly one does.** Two holders, or none, leave it in none; a named project wins.

use api::{HostedSessions, Projects};
use ipc::{NewProject, ProjectMember, StartSession};

use super::agent_starts::{rig, Rig};

async fn a_project(rig: &Rig, name: &str) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: name.to_string(), members, place: None, values: None };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

async fn started_in(rig: &Rig, project: Option<String>) -> Option<String> {
    let start = StartSession {
        project,
        manifest_id: rig.manifest.clone(),
        title: Some("A job".into()),
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
        workflow: None,
    };
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.fleet.project_of_job(id.as_str()).await
}

#[tokio::test]
async fn a_start_with_no_project_is_in_the_one_project_that_holds_its_repository() {
    let rig = rig(None);
    let only = a_project(&rig, "Shop").await;
    assert_eq!(started_in(&rig, None).await.as_deref(), Some(only.as_str()));
    assert_eq!(started_in(&rig, Some("  ".into())).await.as_deref(), Some(only.as_str()));
}

#[tokio::test]
async fn a_start_with_no_project_is_in_none_where_two_projects_or_none_hold_its_repository() {
    let rig = rig(None);
    assert_eq!(started_in(&rig, None).await, None);
    a_project(&rig, "Shop").await;
    a_project(&rig, "Yard").await;
    assert_eq!(started_in(&rig, None).await, None);
}

#[tokio::test]
async fn a_project_the_start_names_wins() {
    let rig = rig(None);
    a_project(&rig, "Shop").await;
    let named = a_project(&rig, "Yard").await;
    assert_eq!(started_in(&rig, Some(named.clone())).await.as_deref(), Some(named.as_str()));
}
