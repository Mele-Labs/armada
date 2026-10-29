//! The claim, written before the code: **a Job's task marks follow what it did, not only what the agent
//! ticked.** A task in progress goes stale when the edits go to another group's files, when 40 calls pass
//! without it being touched, or when a newer task starts; Fleet marks a task itself where it has proof,
//! and tells the agent once. `.claude/decisions/2026-10-27-fleet-keeps-the-task-list-honest.md`.

use adapter_traits::{CallDetail, DroneEvent};
use api::Sessions;
use ipc::{PlanTaskState, ReportedTaskStatus, SessionFact, SessionId, SessionPlanTask, SessionRecord, SessionReport};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{edit, first, group, plan, read, recorded, running, task, tasks, told};
use crate::session_host::Heard;

use PlanTaskState::{Done, Open, Working};
use ReportedTaskStatus::{Completed, InProgress, Pending};

/// The plan task whose text is `text`, as the record carries it.
fn task_of(record: &SessionRecord, text: &str) -> SessionPlanTask {
    record.plan.iter().flat_map(|p| &p.groups).flat_map(|g| &g.tasks).find(|t| t.text == text).cloned().expect("a task of the plan")
}

fn read_call(path: &str) -> DroneEvent {
    DroneEvent::Called { tool: "Read".into(), call: "toolu_r".into(), detail: CallDetail::of(path) }
}

/// `calls`, then an edit of `marker`, a file outside every group, which says when they were all heard.
fn heard_after(mut calls: Vec<DroneEvent>, marker: &str) -> Vec<DroneEvent> {
    calls.push(edit(marker));
    calls
}

async fn touched(rig: &Rig, id: &SessionId, path: &str) -> bool {
    read(rig, id).await.touches.iter().any(|one| one.path == path)
}

async fn committed(rig: &Rig, id: &SessionId, commit: &str, subject: &str) -> SessionRecord {
    let harness = read(rig, id).await.harness;
    let fact = SessionFact::Committed { branch: Some("armada/topic".into()), commit: commit.into(), subject: subject.into() };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("a report is kept")
}

/// A plan recorded and "Add the tables" in progress, on a hosted Job whose process is running.
async fn working(rig: &Rig) -> SessionId {
    let id = running(rig).await;
    recorded(rig, first(&id)).await;
    tasks(rig, &id, vec![task("Add the tables", InProgress)]).await;
    id
}

#[tokio::test]
async fn a_task_left_alone_for_forty_calls_goes_stale_and_the_agent_is_told_once() {
    let rig = rig_placing(None);
    let id = working(&rig).await;

    // 38 reads and an edit outside every group: 39 calls, the edit marking when they were all heard.
    rig.stand_in.says(0, heard_after((0..38).map(|_| read_call("README.md")).collect(), "docs/one.md"));
    eventually(|| async { touched(&rig, &id, "docs/one.md").await }).await;
    assert_eq!(task_of(&read(&rig, &id).await, "Add the tables").stale, None, "39 calls is not yet stale");

    rig.stand_in.says(0, vec![read_call("README.md")]);
    eventually(|| async { task_of(&read(&rig, &id).await, "Add the tables").stale.is_some() }).await;
    let stale = task_of(&read(&rig, &id).await, "Add the tables").stale.expect("stale");
    assert_eq!(stale.why, "40 calls with no update");
    eventually(|| async { told(&rig, "is in progress, but 40 calls with no update") == 1 }).await;

    rig.stand_in.says(0, heard_after((0..39).map(|_| read_call("README.md")).collect(), "docs/two.md"));
    eventually(|| async { touched(&rig, &id, "docs/two.md").await }).await;
    assert_eq!(told(&rig, "is in progress, but 40 calls with no update"), 1, "told once, however long it stays stale");

    // The agent's own list moving it is a touch: no longer stale.
    let record = tasks(&rig, &id, vec![task("Add the tables", Pending)]).await;
    assert_eq!(task_of(&record, "Add the tables").stale, None);
}

#[tokio::test]
async fn edits_in_another_groups_files_make_the_task_in_progress_stale_until_its_own_are_edited() {
    let rig = rig_placing(None);
    let id = working(&rig).await;

    rig.stand_in.says(0, vec![edit("crates/fleet/src/plan.rs")]);
    eventually(|| async { task_of(&read(&rig, &id).await, "Add the tables").stale.is_some() }).await;
    assert_eq!(task_of(&read(&rig, &id).await, "Add the tables").stale.expect("stale").why, "edits went to Fleet");

    rig.stand_in.says(0, vec![edit("crates/store/src/plan.rs")]);
    eventually(|| async { task_of(&read(&rig, &id).await, "Add the tables").stale.is_none() }).await;
}

#[tokio::test]
async fn a_commit_naming_a_task_marks_it_done_with_its_proof_until_the_agents_list_changes_it() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.stand_in.says(0, vec![edit("crates/store/src/plan.rs")]);
    eventually(|| async { read(&rig, &id).await.plan.is_some_and(|plan| plan.groups[0].worked_at.is_some()) }).await;

    let record = committed(&rig, &id, "1a2b3c4", "Add the tables for the plan").await;
    let marked = task_of(&record, "Add the tables");
    assert_eq!((marked.state, marked.marked.as_deref()), (Done, Some("commit 1a2b3c4 names it")));
    // A hosted Job is told at its next tool call, as any word of Fleet's mid-turn is.
    rig.stand_in.says(0, vec![read_call("README.md")]);
    eventually(|| async { told(&rig, "is completed: commit 1a2b3c4 names it") == 1 }).await;

    // The same list again says nothing new: Fleet's mark holds.
    let record = tasks(&rig, &id, vec![task("Add the tables", InProgress)]).await;
    assert_eq!(task_of(&record, "Add the tables").state, Done, "a list that did not change the task leaves the mark");

    // The agent changing the task is its own word again.
    let record = tasks(&rig, &id, vec![task("Add the tables", Pending)]).await;
    let back = task_of(&record, "Add the tables");
    assert_eq!((back.state, back.marked), (Open, None));
    assert_eq!(told(&rig, "is completed: commit 1a2b3c4 names it"), 1);
}

#[tokio::test]
async fn the_newest_task_in_progress_leads_and_the_older_one_goes_stale() {
    let rig = rig_placing(None);
    let id = working(&rig).await;

    let record = tasks(&rig, &id, vec![task("Add the tables", InProgress), task("Record it", InProgress)]).await;
    let older = task_of(&record, "Add the tables");
    assert_eq!(older.state, Working, "stale is never done");
    assert_eq!(older.stale.expect("stale").why, "\"Record it\" started after it");
    assert_eq!(task_of(&record, "Record it").stale, None);
}

#[tokio::test]
async fn two_subagents_each_editing_their_own_group_keep_both_tasks_in_progress() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    tasks(&rig, &id, vec![task("Add the tables", InProgress), task("Record it", InProgress)]).await;

    rig.stand_in.hears(0, Heard::Subagent { events: vec![edit("crates/store/src/plan.rs")], call: Some("toolu_77".into()) });
    rig.stand_in.hears(0, Heard::Subagent { events: vec![edit("crates/fleet/src/plan.rs")], call: Some("toolu_88".into()) });
    eventually(|| async { read(&rig, &id).await.touches.len() == 2 }).await;
    let record = read(&rig, &id).await;
    assert_eq!(task_of(&record, "Add the tables").stale, None);
    assert_eq!(task_of(&record, "Record it").stale, None);
}

#[tokio::test]
async fn an_edit_with_nothing_in_progress_marks_the_groups_first_open_task_working() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;

    rig.stand_in.says(0, vec![edit("crates/store/src/plan.rs")]);
    eventually(|| async { task_of(&read(&rig, &id).await, "Add the tables").state == Working }).await;
    let record = read(&rig, &id).await;
    assert_eq!(task_of(&record, "Add the tables").marked.as_deref(), Some("edits in crates/store/src/plan.rs"));
    assert_eq!(task_of(&record, "Add the migration").state, Open, "only the first open task");
    eventually(|| async { told(&rig, "is in progress: edits in crates/store/src/plan.rs") == 1 }).await;
}

#[tokio::test]
async fn an_edit_with_nothing_in_progress_marks_the_open_task_whose_title_names_the_file() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let named = plan(
        &id,
        vec![group(None, "Store", "rust-engineer", &["crates/store"], &[(None, "Check the guide's inventory"), (None, "PlanTables: keep the rows")])],
    );
    recorded(&rig, named).await;

    rig.stand_in.says(0, vec![edit("crates/store/src/plan_tables.rs")]);
    eventually(|| async { task_of(&read(&rig, &id).await, "PlanTables: keep the rows").state == Working }).await;
    assert_eq!(task_of(&read(&rig, &id).await, "Check the guide's inventory").state, Open, "not the first open task");
}

#[tokio::test]
async fn a_list_that_says_completed_again_takes_down_fleets_in_progress_mark() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, first(&id)).await;
    tasks(&rig, &id, vec![task("Add the tables", Completed)]).await;
    {
        let mut store = rig.fleet.store().lock().await;
        let mut plan = store.session_plan(id.as_str()).expect("read").expect("a plan");
        let marked = &mut plan.groups[0].tasks[0];
        marked.marked = Some("in_progress".into());
        marked.marked_why = Some("edits in crates/store/src/plan.rs".into());
        marked.edited = true;
        store.keep_session_plan_progress(id.as_str(), &plan).expect("kept");
    }
    assert_eq!(task_of(&read(&rig, &id).await, "Add the tables").state, Working);

    let record = tasks(&rig, &id, vec![task("Add the tables", Completed)]).await;
    let back = task_of(&record, "Add the tables");
    assert_eq!((back.state, back.marked), (Done, None));
}

#[tokio::test]
async fn a_merged_pull_request_completes_the_open_tasks_of_the_group_it_was_opened_under() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    // Opened while "Add the tables" was in progress, so it is that group's.
    let harness = read(&rig, &id).await.harness;
    let pull = ipc::AttachmentReport { kind: "pr".into(), target: "12".into(), detail: Default::default() };
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Attached { attachment: pull } })
        .await
        .expect("a report is kept");

    let said = crate::plan_keeping::merge_heard(&mut *rig.fleet.store().lock().await, id.as_str(), 12, "2026-08-26T10:00:00.000Z")
        .expect("kept");
    assert_eq!(said, "\nFleet marked these tasks of your plan completed, as #12 merged: \"Add the tables\", \"Add the migration\".");
    let record = read(&rig, &id).await;
    assert_eq!(task_of(&record, "Add the migration").marked.as_deref(), Some("#12 merged"));
    assert_eq!(task_of(&record, "Record it").state, Open, "another group's task is not the pull request's");

    let again = crate::plan_keeping::merge_heard(&mut *rig.fleet.store().lock().await, id.as_str(), 12, "2026-08-26T10:01:00.000Z")
        .expect("kept");
    assert_eq!(again, "", "a merge read again moves nothing");
}
