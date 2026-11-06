//! The claim: **Fleet marks a task completed on one proof, a commit that names it, and says nothing to
//! the agent.** It marks no task from edits, calls or the order tasks start, and tells no one of a mark.

use api::Sessions;
use ipc::{PlanTaskState, ReportedTaskStatus, SessionFact, SessionId, SessionPlanTask, SessionRecord, SessionReport};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{edit, first, read, recorded, running, task, tasks, told};

use PlanTaskState::{Done, Open, Working};
use ReportedTaskStatus::{Completed, InProgress, Pending};

/// The plan task whose text is `text`, as the record carries it.
fn task_of(record: &SessionRecord, text: &str) -> SessionPlanTask {
    record.plan.iter().flat_map(|p| &p.groups).flat_map(|g| &g.tasks).find(|t| t.text == text).cloned().expect("a task of the plan")
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
async fn a_commit_naming_a_task_marks_it_done_with_its_proof_until_the_agents_list_changes_it() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.stand_in.says(0, vec![edit("crates/store/src/plan.rs")]);
    eventually(|| async { read(&rig, &id).await.plan.is_some_and(|plan| plan.groups[0].worked_at.is_some()) }).await;

    let record = committed(&rig, &id, "1a2b3c4", "Add the tables for the plan").await;
    let marked = task_of(&record, "Add the tables");
    assert_eq!((marked.state, marked.marked.as_deref()), (Done, Some("commit 1a2b3c4 names it")));
    // Nothing is said to the agent, at its next tool call or after.
    rig.stand_in.says(0, vec![edit("docs/one.md")]);
    eventually(|| async { read(&rig, &id).await.touches.iter().any(|one| one.path == "docs/one.md") }).await;
    assert_eq!(told(&rig, "names it"), 0, "the agent is told nothing of a mark");

    // The same list again says nothing new: Fleet's mark holds.
    let record = tasks(&rig, &id, vec![task("Add the tables", InProgress)]).await;
    assert_eq!(task_of(&record, "Add the tables").state, Done, "a list that did not change the task leaves the mark");

    // The agent changing the task is its own word again.
    let record = tasks(&rig, &id, vec![task("Add the tables", Pending)]).await;
    let back = task_of(&record, "Add the tables");
    assert_eq!((back.state, back.marked), (Open, None));
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
