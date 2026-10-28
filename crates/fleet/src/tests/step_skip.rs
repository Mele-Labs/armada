//! The claim, written before the code: **a person can skip a step of a Job's workflow, the current one
//! or one to come, and undo it. Fleet counts the step passed, tells the agent once not to do it or ask
//! about it, takes back what the agent opened on it, and refuses the delivering step.**
//! `docs/concepts/workflow.md`, *Skipping a step*.
//!
//! Beside `session_rail`, which sits near the line count the gate reports. The first half is the rules,
//! pure. The second drives a Fleet with the same workflow and rig.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{BeginSession, DeliveryChoice, Instant, SessionId, SessionRecord, SessionRow, ShowWindow, SkipSessionStep};
use testkit::FakeJudge;

use super::session_host::{eventually, Rig};
use super::session_rail::{
    approved, approved_plan, at, delivers, follows, on_the_rail, plan_step, record, ruled_on, step, told, workflow,
};
use crate::rail::{self, Facts, RailStep, Skip, Skipped};
use crate::session_host::step_skipping::open_since;

// ---- The rules ----

fn skipping(at: &[u32]) -> Facts {
    Facts { skipped: Skipped::of(at), ..Facts::default() }
}

fn walk() -> RailStep {
    RailStep { walks: true, ..step("Walk") }
}

#[test]
fn a_skipped_current_step_moves_the_job_on_to_the_next() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(at(&steps, Facts::default()), Some(0));
    assert_eq!(at(&steps, skipping(&[0])), Some(1), "the plan step skipped, before anything is seen");
    assert_eq!(at(&steps, Facts { edited: true, ..skipping(&[0]) }), Some(1), "and once the Job edits, no plan is waited for");
    assert_eq!(at(&steps, Facts { pushed: true, edited: true, ..skipping(&[0]) }), Some(2));
}

#[test]
fn a_step_to_come_is_passed_over_when_the_job_reaches_it() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(at(&steps, skipping(&[1])), Some(0), "skipping ahead leaves the Job where it is");
    assert_eq!(at(&steps, Facts { skipped: Skipped::of(&[1]), ..approved() }), Some(2), "approval goes past it");
    let walking = [plan_step(), walk(), follows("Refine"), delivers("Deliver")];
    assert_eq!(at(&walking, approved()), Some(1), "the walk, unskipped");
    assert_eq!(at(&walking, Facts { skipped: Skipped::of(&[1]), ..approved() }), Some(2), "the walk skipped: refine at once");
}

#[test]
fn an_undo_puts_the_step_back_where_the_facts_place_it() {
    let walking = [plan_step(), walk(), follows("Refine"), delivers("Deliver")];
    let past = Facts { skipped: Skipped::of(&[1]), tasks_open: true, ..approved() };
    assert_eq!(at(&walking, past), Some(2));
    assert_eq!(at(&walking, Facts { skipped: Skipped::default(), ..past }), Some(1), "undone, the walk is current again");
}

#[test]
fn nothing_skipped_reads_as_it_always_did() {
    let steps = [step("Do it"), delivers("Hand off")];
    assert_eq!(at(&steps, Facts::default()), Some(0), "the first step where nothing is entered");
    assert!(!Skipped::default().has(0));
    assert!(Skipped::of(&[3]).has(3) && !Skipped::of(&[3]).has(2));
    assert!(!Skipped::of(&[64]).has(64), "past the bits, nothing is kept");
}

#[test]
fn fleets_own_nudge_is_never_sent_for_a_step_the_person_skipped() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(rail::skipped(&steps, &Facts { edited: true, ..Facts::default() }), [(0, Skip::Plan)], "the agent's own skip");
    assert!(rail::skipped(&steps, &Facts { edited: true, ..skipping(&[0]) }).is_empty(), "the person's");
    let pushed = Facts { pushed: true, tasks_open: true, skipped: Skipped::of(&[1]), ..approved() };
    assert!(rail::skipped(&steps, &pushed).is_empty());
}

#[test]
fn the_agent_is_told_the_step_by_its_label_and_a_first_turn_carries_every_skip() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    let said = rail::skip_told("Rail", &steps[0], true);
    for part in ["skipped", "\"Plan the change\"", "Rail", "Do not do that step", "do not ask about it"] {
        assert!(said.contains(part), "{part}: {said}");
    }
    assert!(rail::skip_told("Rail", &steps[0], false).contains("wanted again"));
    assert_eq!(rail::skips_said("Rail", &steps, &[]), None);
    let both = rail::skips_said("Rail", &steps, &[0, 1]).expect("said");
    assert!(both.contains("\"Plan the change\", \"Implement\""), "{both}");
}

fn at_second(n: u32) -> Instant {
    Instant::carried(format!("2026-10-28T10:00:{n:02}Z"))
}

#[test]
fn what_is_taken_back_is_what_the_agent_opened_on_the_step_and_has_not_closed() {
    let window = |n: u32, ended: bool| SessionRow::Window {
        id: format!("w{n}"),
        at: at_second(n),
        title: "Mock".into(),
        url: format!("http://localhost:5391/{n}"),
        variations: Vec::new(),
        steps: Vec::new(),
        origins: Vec::new(),
        ended_at: ended.then(|| at_second(n + 1)),
    };
    let draft = |n: u32| SessionRow::Draft {
        id: format!("d{n}"),
        at: at_second(n),
        title: "Release note".into(),
        text: "It stops at the end.".into(),
        before: None,
        settled: None,
        change: None,
    };
    let rows = [window(1, false), draft(2), window(5, false), window(6, true), draft(7)];
    let open = open_since(&rows, at_second(4).as_str());
    assert_eq!(open.windows, ["http://localhost:5391/5"], "before the step, and closed, are left");
    assert_eq!(open.drafts, ["d7"]);
}

// ---- Through a Fleet ----

fn rigged() -> Rig {
    ruled_on(Arc::new(FakeJudge::with_no_objection()), workflow())
}

async fn skip(rig: &Rig, id: &SessionId, step: u32, skipped: bool) -> Result<SessionRecord, Refusal> {
    Arc::clone(&rig.fleet).skip_session_step(SkipSessionStep { session_id: id.clone(), step, skipped }).await
}

async fn says_of_steps(rig: &Rig) -> Vec<String> {
    rig.fleet.timeline().settled().await;
    rig.fleet
        .fleet_timeline(ipc::TimelineAsk::default())
        .await
        .expect("a timeline")
        .events
        .into_iter()
        .filter(|one| one.kind.as_wire() == "session.step")
        .map(|one| one.says)
        .collect()
}

fn refused_with(answer: &Result<SessionRecord, Refusal>, code: &str) -> bool {
    matches!(answer, Err(Refusal::Unacceptable(why)) if why.code == code)
}

#[tokio::test]
async fn skipping_the_current_step_moves_the_job_on_tells_it_once_and_is_entered_on_the_timeline() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    eventually(|| async { record(&rig, &id).await.workflow_visits.len() == 1 }).await;

    let answered = skip(&rig, &id, 0, true).await.expect("skipped");
    assert_eq!(answered.skipped_steps, [0]);
    assert_eq!(answered.workflow_step, Some(1), "the plan step passed over");
    eventually(|| async { told(&rig, "skipped the \"Plan the change\" step") == 1 }).await;
    eventually(|| async { says_of_steps(&rig).await.contains(&"skipped: Plan the change".to_string()) }).await;

    let again = skip(&rig, &id, 0, true).await.expect("the row");
    assert_eq!(again.skipped_steps, [0], "skipping it again changes nothing");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(told(&rig, "skipped the \"Plan the change\" step"), 1, "and says nothing twice");
    assert_eq!(says_of_steps(&rig).await.iter().filter(|says| says.starts_with("skipped:")).count(), 1);
}

#[tokio::test]
async fn an_undo_puts_the_step_back_and_says_it_is_wanted_again() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    skip(&rig, &id, 0, true).await.expect("skipped");

    let undone = skip(&rig, &id, 0, false).await.expect("undone");
    assert!(undone.skipped_steps.is_empty());
    assert_eq!(undone.workflow_step, Some(0), "no plan yet, so the plan step is current again");
    eventually(|| async { told(&rig, "wanted again") == 1 }).await;
    eventually(|| async { says_of_steps(&rig).await.contains(&"skip undone: Plan the change".to_string()) }).await;
}

#[tokio::test]
async fn a_step_skipped_ahead_is_passed_over_when_the_job_reaches_it() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let ahead = skip(&rig, &id, 1, true).await.expect("skipped");
    assert_eq!(ahead.workflow_step, Some(0), "the Job stays where it is");

    approved_plan(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(2), "approval goes past Implement to Hand off");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(told(&rig, "pushed while tasks were still open"), 0, "no nudge for a step the person skipped");
}

#[tokio::test]
async fn a_window_the_agent_opened_on_the_skipped_step_is_taken_back() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    eventually(|| async { record(&rig, &id).await.workflow_visits.len() == 1 }).await;
    let page = "http://localhost:5391/?walk=reader";
    rig.fleet
        .show_window(None, ShowWindow { url: page.into(), title: None, session_id: Some(id.clone()), ..Default::default() })
        .await
        .expect("shown");

    skip(&rig, &id, 0, true).await.expect("skipped");
    let ended = rig.rows(&id).await.into_iter().any(|row| {
        matches!(row, SessionRow::Window { url, ended_at: Some(_), .. } if url == page)
    });
    assert!(ended, "the walk on the skipped step ends");
}

#[tokio::test]
async fn the_delivering_step_a_step_past_the_last_and_a_job_with_no_workflow_are_refused() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let last = skip(&rig, &id, 2, true).await;
    assert!(refused_with(&last, "fleet.session_step_not_skippable"), "{last:?}");
    let past = skip(&rig, &id, 9, true).await;
    assert!(refused_with(&past, "fleet.session_step_not_skippable"), "{past:?}");
    assert!(record(&rig, &id).await.skipped_steps.is_empty(), "nothing kept");

    let plain = rig.start().await;
    let custom = skip(&rig, &plain, 0, true).await;
    assert!(refused_with(&custom, "fleet.session_command_refused"), "{custom:?}");
}

#[tokio::test]
async fn a_job_that_has_not_run_is_told_its_skips_with_its_first_turn() {
    let rig = rigged();
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start_held().await;
    Arc::clone(&rig.fleet)
        .set_session_workflow(ipc::SetSessionWorkflow { session_id: id.clone(), workflow: Some("rail".into()) })
        .await
        .expect("set");
    rig.send_unsettled(&id, super::session_rail::REQUEST).await;

    skip(&rig, &id, 0, true).await.expect("skipped");
    assert!(rig.stand_in.starts().is_empty(), "the skip does not start the Job");

    Arc::clone(&rig.fleet)
        .begin_session(BeginSession { session_id: id.clone(), delivery: Some(DeliveryChoice::OpenPr), message: None })
        .await
        .expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, "skipped the \"Plan the change\" step") == 1 }).await;
}
