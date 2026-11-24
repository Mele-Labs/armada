//! A project's board layout is kept in its file with the rest of its look, and a save changes nothing else of its values.
//! A project that saved none answers none, and a block kind this build does not know is kept and passed back.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::Projects;
use ipc::{BoardBlock, BoardLayout, ManifestId, NewProject, ProjectMember, SaveProjectBoard, SetProjectWorkflow};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::main_ci::MANIFEST;
use super::projects::code_of;
use super::projects_kept::file;
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use crate::daemon::Fleet;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

async fn a_rig() -> (Arc<Hosted>, TempDir, String) {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600)));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let members = Some(vec![ProjectMember { manifest_id: ManifestId::carried(&manifest), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    let project = fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    (fleet, home, project)
}

fn block(kind: &str, x: u8, y: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x, y, w: 3, h: 2, view: view.to_string() }
}

fn saved(blocks: Vec<BoardBlock>) -> SaveProjectBoard {
    SaveProjectBoard { layout: Some(BoardLayout { blocks }) }
}

#[tokio::test]
async fn a_project_that_saved_no_board_answers_none() {
    let (fleet, _home, project) = a_rig().await;
    let read = fleet.get_project_board(project.clone()).await.expect("read");
    assert_eq!((read.project, read.layout), (project, None));
}

#[tokio::test]
async fn a_saved_board_reads_back_whole_with_a_kind_this_build_does_not_know_and_is_kept_in_the_projects_file() {
    let (fleet, home, project) = a_rig().await;
    let layout = vec![block("needs", 0, 0, "list"), block("errors", 3, 0, "kanban"), block("checks", 0, 2, "table")];

    let answered = fleet.save_project_board(project.clone(), saved(layout.clone())).await.expect("saved");
    assert_eq!(answered.layout, Some(BoardLayout { blocks: layout.clone() }));

    fleet.projects_reload().await;
    let read = fleet.get_project_board(project.clone()).await.expect("read");
    assert_eq!(read.layout, Some(BoardLayout { blocks: layout }), "an unknown kind and view are passed through as saved, in order");
    let written = std::fs::read_to_string(file(&home, &project)).expect("its file");
    assert!(written.contains("errors") && written.contains("kanban"), "kept in the project's file: {written}");
}

#[tokio::test]
async fn a_save_changes_nothing_else_of_the_values_and_none_forgets_the_board() {
    let (fleet, home, project) = a_rig().await;
    fleet.set_project_workflow(project.clone(), SetProjectWorkflow { workflow: "chore".to_string(), on: false }).await.expect("a switch");
    fleet.save_project_board(project.clone(), saved(vec![block("run", 0, 0, "list")])).await.expect("saved");

    let listed = fleet.list_projects().await.expect("listed");
    let values = listed.projects.iter().find(|one| one.id == project).and_then(|one| one.values.clone()).expect("its values");
    assert_eq!(values.workflows_off, ["chore"], "the switch the board was saved beside stays");

    // A later edit of the values that carries the board back keeps it; one that leaves it out takes it away.
    let mut kept = values.clone();
    kept.review_model = Some("opus".to_string());
    fleet.edit_project(project.clone(), ipc::EditProject { values: Some(kept), ..ipc::EditProject::default() }).await.expect("edited");
    assert_eq!(fleet.get_project_board(project.clone()).await.expect("read").layout.map(|one| one.blocks.len()), Some(1));

    fleet.save_project_board(project.clone(), SaveProjectBoard { layout: None }).await.expect("forgotten");
    assert_eq!(fleet.get_project_board(project.clone()).await.expect("read").layout, None);
    let written = std::fs::read_to_string(file(&home, &project)).expect("its file");
    assert!(!written.contains("board") && written.contains("workflows_off"), "the board leaves the file and the switch stays: {written}");
}

#[tokio::test]
async fn an_empty_board_is_kept_as_an_empty_board_and_a_blank_kind_is_refused() {
    let (fleet, _home, project) = a_rig().await;
    fleet.save_project_board(project.clone(), saved(Vec::new())).await.expect("saved");
    let read = fleet.get_project_board(project.clone()).await.expect("read");
    assert_eq!(read.layout, Some(BoardLayout { blocks: Vec::new() }), "no block drawn is a layout, not none");

    let refused = fleet.save_project_board(project.clone(), saved(vec![block("  ", 0, 0, "list")])).await.expect_err("a blank kind");
    assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()));
    let refused = fleet.save_project_board("nope".to_string(), saved(Vec::new())).await.expect_err("no such project");
    assert_eq!(code_of(refused), (404, "fleet.no_such_project".to_string()));
}
