//! The claim, written before the code: **the rail reads a step's facts, not a proxy for them.** A
//! page a step writes under `.armada/documents/` is the step's own output, not an edit before it; a
//! fake that lets a crate compile is not a test written; tasks of a group handed to its own Job are
//! not open in this one; and a Job with no code is not told to push. Each came from a notice the agent
//! disputed with the reason below.

use ipc::{BuildPhase, FileTouch, Instant, PlanTaskState, SessionBuild, SessionRecord, SessionStanding, TouchAct};

use super::session_rail::{delivers, follows, plan_step};
use crate::rail::{self, Extra, Facts};
use crate::rail_ends::End;
use crate::test_paths::{is_step_output, is_test_path};

async fn bare() -> SessionRecord {
    let rig = super::agent_starts::rig(None);
    let agent = rig.an_agent().await;
    rig.record(&agent).await
}

fn building(phase: BuildPhase) -> SessionBuild {
    SessionBuild {
        phase,
        pieces: Vec::new(),
        stops: Vec::new(),
        forecast: Vec::new(),
        landed_at: None,
        quiet_since: None,
        stalled: false,
        workers: Vec::new(),
    }
}

fn touch(path: &str, act: TouchAct) -> FileTouch {
    FileTouch { by: None, path: path.into(), act, at: Instant::carried("2026-10-16T10:00:00.000Z") }
}

#[tokio::test]
async fn a_page_a_step_writes_is_its_own_output_and_not_an_edit_before_the_step() {
    let mut record = bare().await;
    record.build = Some(building(BuildPhase::Implement));
    record.touches = vec![touch(".armada/documents/custom-margins-rollout-plan.html", TouchAct::Write)];
    assert!(!Facts::of(&record, Extra::default()).edited, "the Draft step's page is the step's work");

    record.touches.push(touch("crates/fleet/src/rail.rs", TouchAct::Edit));
    assert!(Facts::of(&record, Extra::default()).edited, "code beside it is an edit");

    record.touches = vec![touch(".armada/documents/plan.html", TouchAct::Write)];
    assert!(Facts::of(&record, Extra { committed: true, ..Extra::default() }).edited, "a commit is code");
    assert!(is_step_output(".armada/documents/a.html") && is_step_output("/repo/.armada/slots/slot-5/.armada/documents/n/a.html"));
    assert!(!is_step_output("docs/documents/a.html") && !is_step_output(".armada/documentsx/a.html"));
}

#[test]
fn a_fake_that_lets_a_crate_compile_is_not_a_test() {
    for path in ["crates/api/src/tests/fake/sessions.rs", "apps/desktop/src/renderer/src/mock/fake-context.ts"] {
        assert!(!is_test_path(path), "{path}");
    }
    assert!(is_test_path("crates/fleet/src/tests/rail.rs"), "a test beside it still is");
}

#[tokio::test]
async fn a_fake_alone_is_not_a_test_edited() {
    let mut record = bare().await;
    record.touches = vec![touch("crates/api/src/tests/fake/sessions.rs", TouchAct::Edit)];
    assert!(!Facts::of(&record, Extra::default()).test_edited);
}

fn group(id: &str, own_job: bool, states: &[PlanTaskState]) -> ipc::SessionPlanGroup {
    let tasks = states
        .iter()
        .enumerate()
        .map(|(at, state)| ipc::SessionPlanTask { id: format!("{id}-{at}"), text: format!("Task {at}"), state: *state, seen: true, marked: None, stale: None, changed_at: None })
        .collect();
    ipc::SessionPlanGroup { id: id.into(), agent: "x".into(), title: id.into(), gives: None, paths: Vec::new(), tasks, issue: None, worked_at: None, lands_in: Default::default(), own_job }
}

#[tokio::test]
async fn tasks_of_a_group_handed_to_its_own_job_are_not_open_here() {
    use PlanTaskState::{Done, Open, Working};
    let mut record = bare().await;
    record.plan = Some(ipc::SessionPlan {
        version: 1,
        summary: String::new(),
        recorded_at: Instant::carried("2026-10-16T10:00:00.000Z"),
        approved: true,
        approved_at: None,
        set_aside_at: None,
        scope: Vec::new(),
        groups: vec![group("g1", false, &[Done]), group("g2", true, &[Open, Working])],
        changes: None,
        sketch: None,
        held_by: Vec::new(),
    });
    assert!(!Facts::of(&record, Extra::default()).tasks_open, "g2 is another Job's");
    record.plan.as_mut().unwrap().groups[0].tasks[0].state = Open;
    assert!(Facts::of(&record, Extra::default()).tasks_open, "g1 is this Job's and open");
}

#[tokio::test]
async fn a_job_with_no_code_is_not_told_to_push_or_deliver() {
    let steps = [plan_step(), follows("Refine"), delivers("Deliver")];
    let mut record = bare().await;
    record.workflow_step = Some(1);
    record.standing = Some(SessionStanding::Replied);
    record.waiting_for.clear();
    record.touches.clear();
    assert!(rail::stalled(&steps, &record, Extra::default()).is_none(), "nothing was edited: there is nothing to push");

    record.touches = vec![touch(".armada/documents/uat.html", TouchAct::Write)];
    assert!(rail::stalled(&steps, &record, Extra::default()).is_none(), "a page is not code");

    record.build = Some(building(BuildPhase::Implement));
    record.touches = vec![touch("crates/fleet/src/rail.rs", TouchAct::Edit)];
    assert_eq!(rail::stalled(&steps, &record, Extra::default()).map(|(at, end)| (at, end)), Some((1, End::Pushed)), "code was edited: push it");
}
