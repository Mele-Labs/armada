//! The projects his other Macs share, as this Fleet holds them: kept with their Mac and listed beside its own, sent to
//! Bridge by relay, never sent back out; the edits made on a copy, sent to the Mac that holds it, queued while it
//! cannot be reached and sent when it speaks; and, on the Mac that holds one, the gate and the latest edit winning.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{OtherMacs, Projects, Refusal, Subscription};
use ipc::{
    EditProject, Event, MacNamed, MoveProjectRow, NewProjectTask, OtherMacProjects, PhoneGateway, Project, ProjectAct,
    ProjectActFromMac, ProjectRow, ProjectTask, ProjectTaskState, TickProjectTask, WireError,
};

use super::session_host::{rig_placing, Rig};
use super::share::{project, shared};
use crate::phone::{OtherMacRead, PhoneSwitch};

const WORK: &str = "mac-work";

fn task_row(id: &str, title: &str, manifest: &ipc::ManifestId) -> ProjectRow {
    ProjectRow::Task {
        id: id.to_string(),
        manifest_id: manifest.clone(),
        title: title.to_string(),
        notes: String::new(),
        state: ProjectTaskState::Open,
        held_by: None,
        parent: None,
        issues: Vec::new(),
        pulls: Vec::new(),
    }
}

/// A project Work shares, as Work says it.
fn theirs(id: &str, name: &str, rows: Vec<ProjectRow>) -> Project {
    Project { id: id.to_string(), name: name.to_string(), rows, shared_with_macs: true, ..Default::default() }
}

async fn hold(rig: &Rig, projects: Vec<Project>) {
    Arc::clone(&rig.fleet).hold_other_mac_projects(OtherMacProjects { mac: WORK.into(), projects }).await.expect("held");
}

async fn listed(rig: &Rig) -> ipc::Projects {
    rig.fleet.list_projects().await.expect("listed")
}

fn titles_of(projects: &ipc::Projects, project: &str) -> Vec<String> {
    let one = projects.projects.iter().find(|one| one.id == project).expect("the project");
    one.rows
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Task { title, .. } => Some(title.clone()),
            ProjectRow::Issue { .. } => None,
        })
        .collect()
}

fn state_in(projects: &ipc::Projects, row: &str) -> ProjectTaskState {
    projects
        .projects
        .iter()
        .flat_map(|one| &one.rows)
        .find_map(|one| match one {
            ProjectRow::Task { id, state, .. } if id == row => Some(*state),
            _ => None,
        })
        .expect("the task")
}

/// Every event already sent. **Read after the call that sent them returns**, so nothing races the read.
async fn sent(watching: &mut Subscription) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(100), watching.next()).await {
        if let api::Next::Send(delivered) = next {
            events.push(delivered.event);
        }
    }
    events
}

fn last_projects(events: &[Event]) -> Option<&ipc::Projects> {
    events.iter().rev().find_map(|one| match one {
        Event::ProjectsChanged(projects) => Some(projects),
        _ => None,
    })
}

/// What a refusal says, with the id it names taken out, so two refusals can be compared whole.
fn said(refused: &Refusal, named: &str) -> (u16, String, String) {
    (refused.status(), refused.error().code.clone(), refused.error().message.replace(named, "<id>"))
}

#[tokio::test]
async fn projects_another_mac_shares_are_held_with_their_mac_told_to_bridge_and_never_sent_back_out() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    shared(&rig, &shop, true).await;
    project(&rig, "Taxes").await;
    let mut watching = rig.fleet.events().subscribe();
    let notes = theirs("p-notes", "Notes", vec![task_row("t-1", "Call the plumber", &rig.manifest)]);
    // A copy that has the id of this Mac's own shared project: only the rule that a copy never goes back keeps it in.
    let echo = theirs(&shop, "Shop on Work", Vec::new());

    hold(&rig, vec![notes.clone(), echo]).await;

    let read = listed(&rig).await;
    let names: Vec<(&str, Option<&str>)> = read.projects.iter().map(|one| (one.name.as_str(), one.mac.as_deref())).collect();
    assert_eq!(names, [("Shop", None), ("Taxes", None), ("Notes", Some(WORK)), ("Shop on Work", Some(WORK))], "this Mac's, then the copies");
    assert_eq!(titles_of(&read, "p-notes"), ["Call the plumber"]);
    let told = sent(&mut watching).await;
    let merged = last_projects(&told).expect("Bridge was told");
    assert_eq!(merged.projects.len(), 4, "the copies reach Bridge with this Mac's own");

    let out = rig.fleet.share_projects().await.expect("shared").projects;
    assert_eq!(out.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Shop"], "a copy never goes back out");
    let event = rig.fleet.share_event(Event::ProjectsChanged(merged.clone())).await.expect("crosses");
    let Event::ProjectsChanged(crossing) = event else { panic!("a projects event") };
    assert_eq!(crossing.projects.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Shop"], "nor in an event");

    // A write of this Mac's own still tells Bridge of the copies, or Bridge would lose them.
    rig.fleet
        .add_project_task(NewProjectTask { manifest_id: rig.manifest.clone(), project: Some(shop.clone()), title: "Order tape".into(), notes: String::new() })
        .await
        .expect("added");
    let told = sent(&mut watching).await;
    let merged = last_projects(&told).expect("the write was published");
    assert!(merged.projects.iter().any(|one| one.mac.as_deref() == Some(WORK) && one.id == "p-notes"), "the copy stays: {merged:?}");
    assert_eq!(titles_of(merged, &shop), ["Order tape"]);

    // The same list again changes nothing and says nothing.
    hold(&rig, vec![notes.clone()]).await;
    let _ = sent(&mut watching).await;
    hold(&rig, vec![notes]).await;
    assert!(last_projects(&sent(&mut watching).await).is_none(), "an unchanged copy is not told again");

    // A replace that leaves it out drops it, and forgetting the Mac drops what it left.
    hold(&rig, Vec::new()).await;
    assert!(listed(&rig).await.projects.iter().all(|one| one.mac.is_none()));
    hold(&rig, vec![theirs("p-back", "Back", Vec::new())]).await;
    assert!(listed(&rig).await.projects.iter().any(|one| one.id == "p-back"));
    let _ = sent(&mut watching).await;
    rig.fleet.forget_other_mac(MacNamed { mac: WORK.into() }).await.expect("forgot");
    assert!(listed(&rig).await.projects.iter().all(|one| one.mac.is_none()), "forgotten");
    let told = sent(&mut watching).await;
    assert!(last_projects(&told).expect("Bridge was told").projects.iter().all(|one| one.mac.is_none()));
}

#[tokio::test]
async fn a_copy_refuses_what_only_the_mac_that_holds_it_may_change() {
    let rig = rig_placing(None);
    let own = project(&rig, "Shop").await;
    rig.fleet
        .add_project_task(NewProjectTask { manifest_id: rig.manifest.clone(), project: Some(own.clone()), title: "Mine".into(), notes: String::new() })
        .await
        .expect("added");
    let mine = match &listed(&rig).await.projects[0].rows[0] {
        ProjectRow::Task { id, .. } => id.clone(),
        other => panic!("{other:?}"),
    };
    hold(&rig, vec![theirs("p-notes", "Notes", vec![task_row("t-1", "Call the plumber", &rig.manifest)])]).await;
    let before = listed(&rig).await;
    let held = |refused: Refusal| {
        assert_eq!((refused.status(), refused.error().code.as_str()), (409, "fleet.other_mac_project_held"));
        assert_eq!(refused.error().message, format!("Only Mac {WORK} can change this project."));
    };

    held(rig.fleet.edit_project("p-notes".into(), EditProject { name: Some("Mine now".into()), ..Default::default() }).await.expect_err("refused"));
    held(rig.fleet.edit_project("p-notes".into(), EditProject { shared_with_macs: Some(false), ..Default::default() }).await.expect_err("the switch is the holder's"));
    held(rig.fleet.delete_project("p-notes".into()).await.expect_err("refused"));
    held(rig.fleet.link_task_job("t-1".into(), ipc::LinkTaskJob { session_id: ipc::SessionId::carried("01JOB") }).await.expect_err("Job links wait for stage 2"));
    held(rig.fleet.unlink_task_job("t-1".into()).await.expect_err("refused"));
    held(Projects::move_project_task_repository(&*rig.fleet, "t-1".into(), ipc::MoveProjectTaskRepository { manifest_id: rig.manifest.clone() }).await.expect_err("refused"));
    let into_own = MoveProjectRow { project: Some(own.clone()), position: 0, parent: None };
    held(rig.fleet.move_project_row("t-1".into(), into_own).await.expect_err("a row never moves between Macs"));
    let loose = MoveProjectRow { project: Some(String::new()), position: 0, parent: None };
    held(rig.fleet.move_project_row("t-1".into(), loose).await.expect_err("nor out of a copy"));
    let into_copy = MoveProjectRow { project: Some("p-notes".into()), position: 0, parent: None };
    held(rig.fleet.move_project_row(mine, into_copy).await.expect_err("nor into one"));

    assert_eq!(listed(&rig).await, before, "nothing refused changed anything");
}

/// The Gateway as a stand-in: what Fleet sent which Mac, and what the Mac answers. `None` is a Mac that sleeps.
struct Link {
    sent: Mutex<Vec<(String, String, ProjectActFromMac)>>,
    reply: Mutex<Option<(u16, Vec<u8>)>>,
}

impl Link {
    fn new() -> Arc<Link> {
        Arc::new(Link { sent: Mutex::new(Vec::new()), reply: Mutex::new(None) })
    }

    fn answers(&self, status: u16, body: &impl serde::Serialize) {
        *self.reply.lock().expect("not poisoned") = Some((status, ipc::encode(body).expect("encodes").into_bytes()));
    }

    fn sleeps(&self) {
        *self.reply.lock().expect("not poisoned") = None;
    }

    fn calls(&self) -> Vec<ProjectActFromMac> {
        self.sent.lock().expect("not poisoned").iter().map(|(_, _, one)| one.clone()).collect()
    }
}

impl PhoneSwitch for Link {
    fn state(&self) -> PhoneGateway {
        PhoneGateway { on: true, port: 8443, listening: true, why: None }
    }

    fn set(&self, _on: bool) -> PhoneGateway {
        self.state()
    }

    fn send_other_mac(&self, mac: &str, path: &str, body: Vec<u8>) -> OtherMacRead {
        let sent: ProjectActFromMac = ipc::decode("a sent act", &body).expect("a project act");
        self.sent.lock().expect("not poisoned").push((mac.to_string(), path.to_string(), sent));
        let reply = self.reply.lock().expect("not poisoned").clone();
        Box::pin(async move { reply.ok_or_else(|| String::from("Work could not be reached")) })
    }
}

fn holder_says(rows: Vec<ProjectRow>) -> ipc::Projects {
    ipc::Projects { projects: vec![theirs("p-notes", "Notes", rows)], loose: Vec::new(), left_out: Vec::new() }
}

fn edit_of(title: &str) -> ProjectTask {
    ProjectTask { title: title.to_string(), notes: String::new() }
}

#[tokio::test]
async fn an_edit_on_a_copy_goes_to_its_mac_and_the_answer_is_the_copy_and_a_refusal_there_is_the_answer() {
    let rig = rig_placing(None);
    let link = Link::new();
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    let rows = vec![task_row("t-1", "Call the plumber", &rig.manifest), task_row("t-2", "Order tape", &rig.manifest)];
    hold(&rig, vec![theirs("p-notes", "Notes", rows.clone())]).await;

    let mut said_there = rows.clone();
    said_there[0] = task_row("t-1", "Call the plumber today", &rig.manifest);
    link.answers(200, &holder_says(said_there));
    let answered = rig.fleet.edit_project_task("t-1".into(), edit_of("Call the plumber today")).await.expect("edited on Work");

    assert_eq!(titles_of(&answered, "p-notes"), ["Call the plumber today", "Order tape"], "what Work said is the copy");
    assert_eq!(titles_of(&listed(&rig).await, "p-notes"), ["Call the plumber today", "Order tape"]);
    let calls = link.sent.lock().expect("not poisoned").clone();
    assert_eq!(calls.len(), 1);
    assert_eq!((calls[0].0.as_str(), calls[0].1.as_str()), (WORK, "/other_mac/project_act"));
    let ProjectAct::EditProjectTask { row, fields, .. } = &calls[0].2.act else { panic!("{:?}", calls[0].2) };
    assert_eq!(row, "t-1");
    assert_eq!(fields.map(|one| (one.title, one.notes)), Some((true, false)), "only the field he changed is sent");
    assert!(calls[0].2.at.is_some(), "when he made it");

    // Work refuses it: that is the answer, with its status and code, and nothing is queued.
    let theirs = WireError::raised("fleet.no_such_project_row", "there is no row t-2", ipc::RunId::carried("01THEIRRUN"));
    link.answers(404, &theirs);
    let refused = rig.fleet.tick_project_task("t-2".into(), TickProjectTask { done: true }).await.expect_err("refused there");
    assert_eq!((refused.status(), refused.error().code.as_str(), refused.error().message.as_str()), (404, "fleet.no_such_project_row", "there is no row t-2"));
    assert!(rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty(), "a refusal that was answered is not queued");
    assert_eq!(state_in(&listed(&rig).await, "t-2"), ProjectTaskState::Open);
}

#[tokio::test]
async fn edits_made_while_a_mac_cannot_be_reached_show_at_once_wait_in_order_and_are_sent_when_it_speaks() {
    let rig = rig_placing(None);
    let link = Link::new();
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    let rows = vec![task_row("t-1", "Call the plumber", &rig.manifest), task_row("t-2", "Order tape", &rig.manifest), task_row("t-3", "Old", &rig.manifest)];
    hold(&rig, vec![theirs("p-notes", "Notes", rows.clone())]).await;
    link.sleeps();
    let mut watching = rig.fleet.events().subscribe();

    // A title, a tick, a removal.
    let shown = rig.fleet.edit_project_task("t-1".into(), edit_of("Call the plumber today")).await.expect("queued");
    assert_eq!(titles_of(&shown, "p-notes"), ["Call the plumber today", "Order tape", "Old"], "shown at once");
    let calls_after_first = link.calls().len();
    let ticked = rig.fleet.tick_project_task("t-2".into(), TickProjectTask { done: true }).await.expect("queued");
    assert_eq!(state_in(&ticked, "t-2"), ProjectTaskState::Done);
    let removed = rig.fleet.remove_project_row("t-3".into()).await.expect("queued");
    assert_eq!(titles_of(&removed, "p-notes"), ["Call the plumber today", "Order tape"]);
    assert_eq!(link.calls().len(), calls_after_first, "behind what is queued, so Work is not asked again");
    assert!(last_projects(&sent(&mut watching).await).is_some(), "Bridge was told");

    // A task added while it sleeps has an id of its own, so later edits can name it.
    let added = rig
        .fleet
        .add_project_task(NewProjectTask { manifest_id: rig.manifest.clone(), project: Some("p-notes".into()), title: "Buy tape".into(), notes: String::new() })
        .await
        .expect("queued");
    let made = match added.projects[0].rows.last().expect("a row") {
        ProjectRow::Task { id, title, .. } => {
            assert_eq!(title, "Buy tape");
            id.clone()
        }
        other => panic!("{other:?}"),
    };
    let under = rig.fleet.add_project_subtask(made.clone(), edit_of("Two rolls")).await.expect("queued under the new task");
    let shown: Vec<&str> = under.projects[0]
        .rows
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Task { title, .. } => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(shown, ["Call the plumber today", "Order tape", "Buy tape", "Two rolls"]);
    let blank = rig.fleet.edit_project_task("t-1".into(), edit_of("  ")).await.expect_err("a blank title waits on nobody");
    assert_eq!(blank.status(), 422);
    let queued = rig.fleet.store().lock().await.queued_project_acts(Some(WORK)).expect("reads");
    assert_eq!(queued.len(), 5);
    assert!(queued.windows(2).all(|two| two[0].at <= two[1].at), "in the order he made them");

    // Work speaks: its projects arrive, and the queue is sent behind them, oldest first.
    let mut after = rows;
    after.remove(2);
    after[1] = task_row("t-2", "Order tape", &rig.manifest);
    let holder = holder_says(after);
    link.answers(200, &holder);
    Arc::clone(&rig.fleet)
        .hold_other_mac_projects(OtherMacProjects { mac: WORK.into(), projects: holder.projects.clone() })
        .await
        .expect("held");
    for _ in 0..100 {
        if rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(rig.fleet.store().lock().await.queued_project_acts(None).expect("reads").is_empty(), "every act answered and dropped");
    let calls = link.calls();
    let sent_acts: Vec<&ProjectAct> = calls[calls_after_first..].iter().map(|one| &one.act).collect();
    assert!(matches!(sent_acts[0], ProjectAct::EditProjectTask { .. }), "oldest first: {sent_acts:?}");
    assert!(matches!(sent_acts[1], ProjectAct::TickProjectTask { .. }));
    assert!(matches!(sent_acts[2], ProjectAct::RemoveProjectRow { .. }));
    let ProjectAct::AddProjectTask { id: Some(added_as), .. } = sent_acts[3] else { panic!("{:?}", sent_acts[3]) };
    let ProjectAct::AddProjectSubtask { row, id: Some(_), .. } = sent_acts[4] else { panic!("{:?}", sent_acts[4]) };
    assert_eq!((row, added_as), (&made, &made), "the new task is sent under the id the subtask names");
    assert!(calls[calls_after_first..].windows(2).all(|two| two[0].at <= two[1].at), "each carries when he made it");
    assert_eq!(titles_of(&listed(&rig).await, "p-notes"), ["Call the plumber", "Order tape"], "the copy is what Work last said");
}

/// An unshared project of this Mac's with one task, "Mine": the project's id and the task's.
async fn shop_with_a_task(rig: &Rig) -> (String, String) {
    let shop = project(rig, "Shop").await;
    let projects = rig
        .fleet
        .add_project_task(NewProjectTask { manifest_id: rig.manifest.clone(), project: Some(shop.clone()), title: "Mine".into(), notes: String::new() })
        .await
        .expect("added");
    let row = match &projects.projects.iter().find(|one| one.id == shop).expect("shop").rows[0] {
        ProjectRow::Task { id, .. } => id.clone(),
        other => panic!("{other:?}"),
    };
    (shop, row)
}

fn act(act: ProjectAct, at: Option<i64>) -> ProjectActFromMac {
    ProjectActFromMac { act, at }
}

#[tokio::test]
async fn an_act_from_another_mac_goes_only_into_a_shared_project_and_is_refused_as_a_row_that_does_not_exist() {
    let rig = rig_placing(None);
    let (shop, row) = shop_with_a_task(&rig).await;
    let edit = |row: &str, title: &str| act(ProjectAct::EditProjectTask { row: row.to_string(), task: edit_of(title), fields: None }, None);

    let missing = rig.fleet.project_act_from_mac(edit("nope", "x")).await.expect_err("no such row");
    let unshared = rig.fleet.project_act_from_mac(edit(&row, "Theirs")).await.expect_err("unshared");
    assert_eq!(said(&unshared, &row), said(&missing, "nope"), "refused exactly as a row that does not exist");
    assert_eq!(unshared.status(), 404);
    let new_task = |project: Option<&str>| {
        act(
            ProjectAct::AddProjectTask {
                task: NewProjectTask { manifest_id: rig.manifest.clone(), project: project.map(str::to_string), title: "t".into(), notes: String::new() },
                id: None,
            },
            None,
        )
    };
    let nowhere = rig.fleet.project_act_from_mac(new_task(Some("nope"))).await.expect_err("no such project");
    let private = rig.fleet.project_act_from_mac(new_task(Some(&shop))).await.expect_err("unshared project");
    assert_eq!(said(&private, &shop), said(&nowhere, "nope"));
    assert_eq!(rig.fleet.project_act_from_mac(new_task(None)).await.expect_err("loose").status(), 404, "a loose task is never shared");
    assert_eq!(titles_of(&listed(&rig).await, &shop), ["Mine"], "nothing refused changed anything");

    shared(&rig, &shop, true).await;
    let answered = rig.fleet.project_act_from_mac(edit(&row, "Theirs")).await.expect("a shared project takes it");
    assert_eq!(titles_of(&answered, &shop), ["Theirs"], "answers this Mac's shared projects");
    let into_none = MoveProjectRow { project: Some(String::new()), position: 0, parent: None };
    let leaves = rig.fleet.project_act_from_mac(act(ProjectAct::MoveProjectRow { row: row.clone(), to: into_none }, None)).await;
    assert_eq!(leaves.expect_err("a row leaves only for another shared project").status(), 404);
    let into_private = project(&rig, "Taxes").await;
    let to_private = MoveProjectRow { project: Some(into_private), position: 0, parent: None };
    assert_eq!(rig.fleet.project_act_from_mac(act(ProjectAct::MoveProjectRow { row, to: to_private }, None)).await.expect_err("unshared").status(), 404);

    // A row added under an id another Mac minted keeps it, and the same act again adds nothing.
    let with_id = |title: &str| {
        act(
            ProjectAct::AddProjectTask {
                task: NewProjectTask { manifest_id: rig.manifest.clone(), project: Some(shop.clone()), title: title.into(), notes: String::new() },
                id: Some("workmade01".into()),
            },
            Some(5),
        )
    };
    rig.fleet.project_act_from_mac(with_id("From Work")).await.expect("added");
    let again = rig.fleet.project_act_from_mac(with_id("From Work")).await.expect("already applied");
    assert_eq!(titles_of(&again, &shop), ["Theirs", "From Work"]);
    assert!(again.projects.iter().flat_map(|one| &one.rows).any(|one| matches!(one, ProjectRow::Task { id, .. } if id == "workmade01")));
}

#[tokio::test]
async fn an_act_made_before_a_field_was_edited_here_leaves_that_field_and_a_tick_is_only_the_persons_own() {
    let rig = rig_placing(None);
    let (shop, row) = shop_with_a_task(&rig).await;
    shared(&rig, &shop, true).await;
    let edit = |title: &str, at: i64| act(ProjectAct::EditProjectTask { row: row.clone(), task: edit_of(title), fields: None }, Some(at));

    rig.fleet.edit_project_task(row.clone(), edit_of("Here")).await.expect("edited here, now");
    // The clock of the fixture is in 2026, so an act made at 1 is older than that edit, and one made far on is newer.
    let older = rig.fleet.project_act_from_mac(edit("Work, long ago", 1)).await.expect("answered");
    assert_eq!(titles_of(&older, &shop), ["Here"], "the title was edited here since");
    let newer = rig.fleet.project_act_from_mac(edit("Work, later", i64::MAX / 2)).await.expect("answered");
    assert_eq!(titles_of(&newer, &shop), ["Work, later"]);

    // What Work holds of this Mac's tasks never ticks one: a copy is only what the holder says.
    let tick = |done: bool, at: i64| act(ProjectAct::TickProjectTask { row: row.clone(), tick: TickProjectTask { done } }, Some(at));
    assert_eq!(state_in(&rig.fleet.project_act_from_mac(tick(true, i64::MAX / 2)).await.expect("ticked"), &row), ProjectTaskState::Done);
    assert_eq!(state_in(&rig.fleet.project_act_from_mac(tick(false, 1)).await.expect("answered"), &row), ProjectTaskState::Done, "an older tick leaves it");
    let held = rig.fleet.list_projects().await.expect("listed");
    hold(&rig, held.projects.iter().cloned().map(|mut one| {
        one.id = format!("copy-{}", one.id);
        one
    }).collect()).await;
    assert_eq!(state_in(&rig.fleet.list_projects().await.expect("listed"), &row), ProjectTaskState::Done, "holding a copy changes no task here");
}
