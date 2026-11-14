//! The claim, written before the code: **a merge closes the `merged` gate of the group Fleet ties it
//! to, and no other: a Job started for a group is tied to it, the Job's own pull requests to its one
//! group that is not handed on, a merge from before a group was recorded closes nothing of it, and a
//! group with work still on the way stays open.** `docs/concepts/session/the-plan.md`, *Gates*.

use api::Sessions;
use ipc::{AttachmentReport, PlanGate, PlanGroupInput, SessionFact, SessionId, SessionRecord, SessionReport};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_host::{rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running, task, tasks};
use ipc::ReportedTaskStatus::{Completed, Pending};

fn gated(id: Option<&str>, title: &str, gate: PlanGate, own_job: bool) -> PlanGroupInput {
    PlanGroupInput { gate, own_job, ..group(id, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))]) }
}

async fn pull(rig: &Rig, id: &SessionId, number: &str, state: &str) {
    let harness = read(rig, id).await.harness;
    let detail = [("state".to_string(), state.to_string())].into_iter().collect();
    let fact = SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: number.into(), detail } };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("attached");
    // The clock reads milliseconds: let the next row land after this one.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
}

fn closes(record: &SessionRecord) -> Vec<(String, Option<String>)> {
    let plan = record.plan.as_ref().expect("a plan");
    plan.groups.iter().map(|group| (group.id.clone(), group.gate_closed.as_ref().map(|one| one.fact.clone()))).collect()
}

async fn closed_now(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.fleet.gates_kept(id.as_str()).await.into_iter().map(|one| one.group_id).collect()
}

#[tokio::test]
async fn a_merge_of_the_jobs_own_closes_its_one_group_not_handed_on_and_not_the_first_open_one() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = [gated(None, "Native screens", PlanGate::Merged, true), gated(None, "Deliver", PlanGate::Merged, false)];
    recorded(&rig, plan(&id, groups.to_vec())).await;

    pull(&rig, &id, "3442", "merged").await;
    assert_eq!(closed_now(&rig, &id).await, ["g2"]);
    assert_eq!(closes(&read(&rig, &id).await), [("g1".into(), None), ("g2".into(), Some("Pull request #3442 merged".into()))]);
}

#[tokio::test]
async fn a_merge_of_a_job_started_for_the_second_group_closes_only_the_second() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = [gated(None, "Share switch", PlanGate::Merged, true), gated(None, "Tunnel", PlanGate::Merged, true)];
    recorded(&rig, plan(&id, groups.to_vec())).await;

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let at = |ms: &str| format!("2099-01-01T00:00:00.{ms}Z");
    let row = |holder: &str, kind: &str, target: &str, detail: &[(&str, &str)]| KeptAttachment {
        holder: Holder::session(holder),
        kind: kind.into(),
        manifest_id: String::new(),
        target: target.into(),
        state: AttachmentState::Spent,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        since: at("000"),
        changed_at: at("100"),
    };
    {
        let mut store = rig.fleet.store().lock().await;
        store.attach(&row(id.as_str(), crate::started_jobs::STARTED_TO, "tunnel-job", &[("group", "g2")]), false).expect("linked");
        store.attach(&row("tunnel-job", "pr", "3469", &[("state", "merged")]), false).expect("attached");
    }
    assert_eq!(closed_now(&rig, &id).await, ["g2"]);
    assert_eq!(closes(&read(&rig, &id).await), [("g1".into(), None), ("g2".into(), Some("Pull request #3469 merged".into()))]);
}

#[tokio::test]
async fn a_merge_from_before_a_group_was_recorded_closes_nothing() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, plan(&id, vec![gated(None, "Build", PlanGate::None, false)])).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    pull(&rig, &id, "3342", "merged").await;

    let again = [gated(Some("g1"), "Build", PlanGate::None, false), gated(None, "Deliver", PlanGate::Merged, false)];
    recorded(&rig, plan(&id, again.to_vec())).await;
    assert!(closed_now(&rig, &id).await.is_empty(), "#3342 merged before Deliver was in the plan");
    assert_eq!(closes(&read(&rig, &id).await)[1].1, None);
}

#[tokio::test]
async fn a_merge_nothing_ties_to_an_open_group_closes_nothing() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = [gated(None, "Decisions", PlanGate::Merged, false), gated(None, "Stage 1", PlanGate::Merged, true)];
    recorded(&rig, plan(&id, groups.to_vec())).await;
    pull(&rig, &id, "3467", "merged").await;
    assert_eq!(closed_now(&rig, &id).await, ["g1"]);

    // A fix the Job opened later says nothing of Stage 1, whose work is its own Job's.
    pull(&rig, &id, "3557", "merged").await;
    assert!(closed_now(&rig, &id).await.is_empty(), "#3557 is not Stage 1's");
    assert_eq!(closes(&read(&rig, &id).await)[1].1, None);
}

#[tokio::test]
async fn a_group_landing_in_several_pull_requests_closes_on_its_last_merge_not_its_first() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, plan(&id, vec![gated(None, "Build and deliver", PlanGate::Merged, false)])).await;
    pull(&rig, &id, "3453", "merged").await;
    pull(&rig, &id, "3454", "open").await;
    assert!(closed_now(&rig, &id).await.is_empty(), "#3454 is still on the way");

    pull(&rig, &id, "3454", "merged").await;
    assert_eq!(closed_now(&rig, &id).await, ["g1"]);
    assert_eq!(closes(&read(&rig, &id).await)[0].1.as_deref(), Some("Pull request #3454 merged"));
}

#[tokio::test]
async fn with_several_groups_of_its_own_a_merge_closes_the_group_whose_work_was_done_before_it() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = [gated(None, "Routine questions", PlanGate::Merged, false), gated(None, "Red main", PlanGate::Merged, false)];
    recorded(&rig, plan(&id, groups.to_vec())).await;

    tasks(&rig, &id, vec![task("Do Routine questions", Completed), task("Do Red main", Pending)]).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    pull(&rig, &id, "3442", "merged").await;
    assert_eq!(closed_now(&rig, &id).await, ["g1"], "Red main has work still open");

    // Its work done after #3442 merged: #3442 is not its pull request.
    tasks(&rig, &id, vec![task("Do Routine questions", Completed), task("Do Red main", Completed)]).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert!(closed_now(&rig, &id).await.is_empty(), "#3442 merged before Red main's work was done");

    pull(&rig, &id, "3450", "merged").await;
    assert_eq!(closed_now(&rig, &id).await, ["g2"]);
    assert_eq!(closes(&read(&rig, &id).await)[1].1.as_deref(), Some("Pull request #3450 merged"));
}
