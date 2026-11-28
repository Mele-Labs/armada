//! A teaching at the `project` reach: the Operator has no repository, so a note taught from its thread goes to one
//! project, is kept under the project's id, and is told to every Job in a repository the project holds.

use std::sync::Arc;

use api::{Logbook, Projects, Refusal, Teaching};
use ipc::{LogbookKind, LogbookSection, NewProject, ProjectMember, SaveTeaching, TeachingNote, TeachingProposal, TeachingReach};

use super::projects::a_second_repository;
use super::session_host::{rig_placing, Rig};
use super::tmp::TempDir;

const TAUGHT: &str = "Run the shop's smoke test before any release.";

/// A project that holds the rig's repository, and its id.
async fn a_project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

fn taught(project: Option<&str>) -> SaveTeaching {
    SaveTeaching {
        proposal: TeachingProposal {
            note: Some(TeachingNote { section: LogbookSection::Repo, kind: LogbookKind::Convention, text: TAUGHT.to_string() }),
            ..TeachingProposal::default()
        },
        reach: TeachingReach::Project,
        repository: None,
        project: project.map(str::to_string),
        session_id: None,
        cite: None,
        source: None,
    }
}

fn code(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

#[tokio::test]
async fn a_project_teaching_is_kept_as_a_project_entry_under_the_projects_id() {
    let rig = rig_placing(None);
    let project = a_project(&rig).await;

    let saved = Arc::clone(&rig.fleet).save_teaching(taught(Some(&project))).await.expect("saved");

    let entry_id = saved.entry.expect("an entry");
    let listed = rig.fleet.get_logbook(Some(rig.root.clone()), None, None).await.expect("read").entries;
    let entry = listed.iter().find(|one| one.id == entry_id).expect("the project's entry is in its repository's Logbook");
    assert_eq!((entry.section, entry.project.as_deref(), entry.repository.as_deref()), (LogbookSection::Project, Some(project.as_str()), None));
    assert_eq!(entry.text, TAUGHT);
}

#[tokio::test]
async fn a_job_in_a_repository_the_project_holds_is_told_it_and_one_in_another_repository_is_not() {
    let rig = rig_placing(None);
    let project = a_project(&rig).await;
    let home = TempDir::new();
    a_second_repository(&rig.fleet, &home);
    let elsewhere = home.path().join("second").to_string_lossy().to_string();
    Arc::clone(&rig.fleet).save_teaching(taught(Some(&project))).await.expect("saved");

    let held = rig.fleet.logbook_note_of(Some(&rig.root)).await.unwrap_or_default();
    let unheld = rig.fleet.logbook_note_of(Some(&elsewhere)).await.unwrap_or_default();

    assert!(held.contains(TAUGHT), "{held}");
    assert!(!unheld.contains(TAUGHT), "{unheld}");
    let listed = rig.fleet.get_logbook(Some(elsewhere), None, None).await.expect("read").entries;
    assert!(listed.iter().all(|one| one.text != TAUGHT), "the other repository's Logbook does not list it");
}

#[tokio::test]
async fn a_project_teaching_with_no_project_or_one_fleet_does_not_hold_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let project = a_project(&rig).await;

    let unnamed = Arc::clone(&rig.fleet).save_teaching(taught(None)).await.expect_err("no project");
    let unheld = Arc::clone(&rig.fleet).save_teaching(taught(Some("no-such-project"))).await.expect_err("not held");
    let mut more = taught(Some(&project));
    more.proposal.words = Some(ipc::TeachingWords { name: "Key".into(), pattern: r"[A-Z]+-\d+".into(), example: "ARM-1".into(), address: None });
    let beyond = Arc::clone(&rig.fleet).save_teaching(more).await.expect_err("only a note");

    assert_eq!(code(unnamed), (422, "fleet.teaching_input_refused".to_string()));
    assert_eq!(code(unheld), (404, "fleet.no_such_project".to_string()));
    assert_eq!(code(beyond), (422, "fleet.teaching_input_refused".to_string()));
    let listed = rig.fleet.get_logbook(Some(rig.root.clone()), None, None).await.expect("read").entries;
    assert!(listed.iter().all(|one| one.text != TAUGHT), "nothing was kept");
}
