//! The claim, written before the code: **a Job can change its own copy of its workflow, lifecycles and rules,
//! the change reaches that Job at once and no other, the lanes mark it and the timeline says who made it, and
//! keeping it for every Job is a proposal that waits on the owner's Approve.**
//! `docs/practices/protocol.md`, *A Job's own copy*.
//!
//! A rule or a lifecycle a Job changed is kept for every Job one piece at a time, as a file in Kit within the
//! Job's repository, and waits on the owner's Approve on the Job's draft; nothing is written before.
//!
//! The first half is the rail's bookkeeping, pure. The second drives a Fleet with the `rail` workflow planted
//! as a file, so the Job's copy has a text to start from.

use std::sync::Arc;
use std::time::Duration;

use api::{Authoring, HostedSessions, Lifecycles, Next, Refusal};
use ipc::{
    AnswerWaiting, ChangeJobCopy, ChangeOrigin, ChangeSessionSteps, ChangeState, ChangedBy, CopyPieceKind, Event, JobCopy, KeepJobPiece,
    KeepJobWorkflow, PieceScope, ProposeWorkflowChange, SessionId, SetSessionWorkflow, SkipSessionStep, StepChange, WaitingSource,
    WorkflowId,
};
use store::{KeptPiece, PieceKind};
use testkit::FakeJudge;

use super::session_host::Rig;
use super::session_rail::{at, delivers, follows, on_the_rail, plan_step, record, ruled_on, step, workflow};
use super::workflow_page::read;
use crate::rail::RailStep;
use crate::session_host::job_copying::rail_remapped;

// ---- The rail's bookkeeping ----

fn ids(of: &[&str]) -> Vec<RailStep> {
    of.iter().map(|id| RailStep { id: (*id).to_string(), ..step(id) }).collect()
}

#[test]
fn what_the_rail_kept_by_index_follows_a_step_to_where_an_edit_put_it() {
    let before = ids(&["plan", "implement", "handoff"]);
    let after = ids(&["plan", "dispatch", "implement", "handoff"]);
    let mut kept = store::KeptRail {
        step: Some(1),
        nudged: vec![1],
        judged: vec![0, 1],
        stalled: vec![2],
        visits: vec![(0, "a".into()), (1, "b".into())],
        skipped: vec![1],
    };
    rail_remapped(&mut kept, &before, &after);
    assert_eq!(kept.step, Some(2), "the step it stood on is the same step");
    assert_eq!(kept.skipped, [2], "and a skip stays with the step it was for");
    assert_eq!((kept.nudged, kept.judged, kept.stalled), (vec![2], vec![0, 2], vec![3]));
    assert_eq!(kept.visits, [(0, "a".to_string()), (2, "b".to_string())]);
}

#[test]
fn a_step_the_edit_took_out_is_forgotten_and_the_rest_close_up() {
    let before = ids(&["plan", "implement", "handoff"]);
    let after = ids(&["plan", "handoff"]);
    let mut kept = store::KeptRail { step: Some(1), skipped: vec![1, 2], visits: vec![(1, "b".into()), (2, "c".into())], ..Default::default() };
    rail_remapped(&mut kept, &before, &after);
    assert_eq!(kept.step, None, "it stood on the step that went");
    assert_eq!(kept.skipped, [1], "the delivering step is now the second");
    assert_eq!(kept.visits, [(1, "c".to_string())]);
}

#[test]
fn the_fixture_steps_are_what_the_tests_below_expect() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(at(&steps, crate::rail::Facts::default()), Some(0));
}

// ---- Through a Fleet ----

/// The `rail` workflow as the file the repository holds.
const RAIL: &str = "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
    - id: plan\n    label: \"Plan the change\"\n    instructions: \"Record the plan before editing.\"\n    evidence: {submitted: {type: plan}}\n    \
    mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
    - id: implement\n    label: \"Implement\"\n    follows_plan: true\n    retry_limit: 2\n    \
    evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - { type: diff_nonempty }\n    delivers: false\n    \
    advance_gate: auto_if_judge_passes\n    judge_checks:\n      -\n        criteria:\n          \
    - criterion_id: c1\n            question: \"Does the reader stop at the end?\"\n            on_refusal: refuse\n  \
    - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";

/// A Fleet whose repository holds `rail` as a file, so a Job can copy it.
pub(super) fn rigged() -> Rig {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), workflow());
    let served = rig.fleet.repositories().serving(rig.manifest.as_str()).expect("served");
    served.catalogued(read(vec![config::Written::in_repository(super::workflow_page::at("repo", "rail.json"), RAIL.to_string())]));
    rig
}

pub(super) fn dispatch() -> StepChange {
    StepChange::Add {
        after: Some("plan".into()),
        id: None,
        label: "Dispatch".into(),
        about: Some("Starts a Job for each fix".into()),
        instructions: Some("Start one Job per fix and note its id.".into()),
        ends_on: Vec::new(),
    }
}

pub(super) async fn change(rig: &Rig, id: &SessionId, change: StepChange) -> Result<JobCopy, Refusal> {
    rig.fleet.steps_changed(ChangeSessionSteps { session_id: id.clone(), change }).await
}

fn refused_with<T: std::fmt::Debug>(answer: &Result<T, Refusal>, code: &str) -> bool {
    matches!(answer, Err(Refusal::Unacceptable(why)) if why.code == code)
}

fn step_ids(copy: &JobCopy) -> Vec<&str> {
    copy.steps.iter().map(|one| one.id.as_str()).collect()
}

/// A second Job on the same workflow.
async fn another_on_the_rail(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("rail".into()) })
        .await
        .expect("set");
    id
}

async fn says_of_copy(rig: &Rig) -> Vec<(String, Option<String>)> {
    rig.fleet.timeline().settled().await;
    rig.fleet
        .fleet_timeline(ipc::TimelineAsk::default())
        .await
        .expect("a timeline")
        .events
        .into_iter()
        .filter(|one| one.kind.as_wire() == "session.copy")
        .map(|one| (one.says, one.detail))
        .collect()
}

#[tokio::test]
async fn a_step_added_to_a_jobs_copy_reaches_that_job_and_no_other() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let other = another_on_the_rail(&rig).await;

    let copy = change(&rig, &id, dispatch()).await.expect("added");
    assert!(copy.own, "the Job holds its own copy now");
    assert_eq!(step_ids(&copy), ["plan", "dispatch", "implement", "handoff"]);
    assert_eq!(copy.steps[1].instructions.as_deref(), Some("Start one Job per fix and note its id."));
    assert_eq!(copy.changes.len(), 1);
    assert_eq!((copy.changes[0].lane.as_str(), copy.changes[0].item.as_str()), ("workflow", "dispatch"));
    assert_eq!(copy.changes[0].text, "Added after Plan the change by you");
    assert_eq!(copy.changes[0].by, ChangedBy::You);

    let held = record(&rig, &id).await;
    let own = held.workflow_copy.expect("the row carries the copy");
    assert_eq!(own.steps.len(), 4, "the rail and the lanes read the Job's steps");
    assert!(record(&rig, &other).await.workflow_copy.is_none(), "another Job on the workflow keeps the repository's");
    let file = rig.fleet.repositories().serving(rig.manifest.as_str()).expect("served").files();
    assert!(file.iter().filter(|one| one.id().as_str() == "rail").all(|one| !one.text().contains("dispatch")), "and the repository's file is as it was");

    let lanes = Arc::clone(&rig.fleet).get_job_lifecycles(None, Some(id.clone())).await.expect("lanes");
    assert_eq!(lanes.changes.len(), 1, "the lanes mark the change");
    assert_eq!(says_of_copy(&rig).await, [("step added: Dispatch".to_string(), Some("by you".to_string()))]);
}

#[tokio::test]
async fn a_skip_stays_on_its_step_when_one_is_added_before_it() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    Arc::clone(&rig.fleet).skip_session_step(SkipSessionStep { session_id: id.clone(), step: 1, skipped: true }).await.expect("skipped");
    change(&rig, &id, dispatch()).await.expect("added");
    assert_eq!(record(&rig, &id).await.skipped_steps, [2], "Implement is the third step now, and still skipped");
    let copy = rig.fleet.job_copy_read(id.as_str()).await.expect("read");
    assert!(copy.steps[2].skipped && !copy.steps[1].skipped);
}

#[tokio::test]
async fn a_step_is_reworded_moved_and_dropped_and_each_is_marked() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let reworded = StepChange::Reword { step: "implement".into(), label: None, about: None, instructions: Some("Build it small.".into()) };
    let copy = change(&rig, &id, reworded).await.expect("reworded");
    assert_eq!(copy.steps[1].instructions.as_deref(), Some("Build it small."));
    assert_eq!(copy.changes[0].text, "Reworded by you");

    change(&rig, &id, dispatch()).await.expect("added");
    let moved = StepChange::Move { step: "dispatch".into(), after: Some("implement".into()) };
    let copy = change(&rig, &id, moved).await.expect("moved");
    assert_eq!(step_ids(&copy), ["plan", "implement", "dispatch", "handoff"]);
    let before_the_plan = change(&rig, &id, StepChange::Move { step: "implement".into(), after: None }).await;
    assert!(refused_with(&before_the_plan, "fleet.job_workflow_unfit"), "a step that follows the plan cannot come first: {before_the_plan:?}");

    let copy = change(&rig, &id, StepChange::Drop { step: "dispatch".into() }).await.expect("dropped");
    assert_eq!(step_ids(&copy), ["plan", "implement", "handoff"]);
    assert!(copy.changes.iter().any(|mark| mark.item == "dispatch" && mark.text == "Dropped by you"));
}

#[tokio::test]
async fn an_edit_the_workflow_cannot_take_is_refused_and_the_job_keeps_what_it_had() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let delivering = change(&rig, &id, StepChange::Drop { step: "handoff".into() }).await;
    assert!(refused_with(&delivering, "fleet.job_copy_step_refused"), "{delivering:?}");
    let missing = change(&rig, &id, StepChange::Drop { step: "lunch".into() }).await;
    assert!(refused_with(&missing, "fleet.job_copy_step_refused"), "{missing:?}");
    let twice = change(&rig, &id, added("Again", Some("plan"), &[])).await;
    assert!(refused_with(&twice, "fleet.job_copy_step_refused"), "{twice:?}");
    let ends = change(&rig, &id, added("Wait", None, &["job: nonsense"])).await;
    assert!(refused_with(&ends, "fleet.job_workflow_unfit"), "{ends:?}");
    assert!(record(&rig, &id).await.workflow_copy.is_none(), "nothing was kept");

    let plain_job = rig.start().await;
    let custom = change(&rig, &plain_job, dispatch()).await;
    assert!(refused_with(&custom, "fleet.session_command_refused"), "a Job with no workflow has no steps: {custom:?}");
}

/// A step added first, named `id` where one is given, ending on `ends`.
fn added(label: &str, id: Option<&str>, ends: &[&str]) -> StepChange {
    StepChange::Add {
        after: None,
        id: id.map(str::to_string),
        label: label.into(),
        about: None,
        instructions: None,
        ends_on: ends.iter().map(|end| end.to_string()).collect(),
    }
}

#[tokio::test]
async fn a_call_through_the_door_that_places_no_job_is_refused() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let door = ipc::Via::from_wire("door");
    let asked = api::carrying(door, change(&rig, &id, dispatch())).await;
    assert!(refused_with(&asked, "fleet.job_copy_unplaced"), "{asked:?}");
    assert!(record(&rig, &id).await.workflow_copy.is_none());
}

#[tokio::test]
async fn switching_workflow_gives_up_the_copy_and_its_marks() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    change(&rig, &id, dispatch()).await.expect("added");
    Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None }).await.expect("cleared");
    let held = record(&rig, &id).await;
    assert!(held.workflow_copy.is_none() && held.workflow.is_none());
    assert!(rig.fleet.job_copy_read(id.as_str()).await.expect("read").changes.is_empty());
}

#[tokio::test]
async fn keeping_a_jobs_workflow_for_every_job_waits_on_the_owner_and_writes_nothing() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let refused = Arc::clone(&rig.fleet).keep_job_workflow(KeepJobWorkflow { session_id: id.clone(), source: None }).await;
    assert!(matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.job_holds_no_copy"), "{refused:?}");

    change(&rig, &id, dispatch()).await.expect("added");
    let kept = Arc::clone(&rig.fleet).keep_job_workflow(KeepJobWorkflow { session_id: id.clone(), source: None }).await.expect("kept");
    assert_eq!(kept.origin, ChangeOrigin::Job);
    assert_eq!(kept.state, ChangeState::Ready);
    assert_eq!((kept.workflow_id.as_str(), kept.source.as_str()), ("rail", "repository"));
    assert!(kept.after.as_deref().is_some_and(|after| after.contains("dispatch")));
    assert_eq!(kept.before, RAIL, "the file as it is, which an Approve is checked against");

    let listed = rig.fleet.list_workflow_changes(WorkflowId::carried("rail"), "repository".into(), None).await.expect("listed");
    assert_eq!(listed.changes.len(), 1);
    let rows = rig.rows(&id).await;
    assert!(rows.iter().any(|row| matches!(row, ipc::SessionRow::Draft { title, change: Some(_), before: Some(_), .. } if title == "Workflow rail")), "the owner's call is on the Job");

    let sent_back = rig.fleet.workflow_change_sent_back(ipc::SendBackWorkflowChange { change: kept.id, notes: "no".into() }).await;
    assert!(matches!(&sent_back, Err(Refusal::IllegalMove(_))), "a Job's proposal is approved or dismissed: {sent_back:?}");
}

#[tokio::test]
async fn a_job_proposes_new_text_for_a_workflow_that_exists_and_it_waits_on_the_owner() {
    let rig = rigged();
    let reworded = RAIL.replace("Record the plan before editing.", "Record the plan, then wait.");
    let propose = |workflow: &str, definition: String| ProposeWorkflowChange {
        workflow_id: WorkflowId::carried(workflow),
        source: "repository".into(),
        definition,
    };
    let kept = rig.fleet.propose_workflow_change(propose("rail", reworded.clone()), None).await.expect("proposed");
    assert_eq!((kept.origin, kept.state), (ChangeOrigin::Job, ChangeState::Ready));
    assert_eq!(kept.after.as_deref(), Some(reworded.trim()));

    let same = rig.fleet.propose_workflow_change(propose("rail", RAIL.to_string()), None).await;
    assert!(refused_with(&same, "fleet.workflow_change_unchanged"), "{same:?}");
    let other_id = rig.fleet.propose_workflow_change(propose("rail", RAIL.replace("workflow_id: rail", "workflow_id: other")), None).await;
    assert!(other_id.is_err(), "the id must stay what it was: {other_id:?}");
    let nowhere = rig.fleet.propose_workflow_change(propose("nothing", reworded), None).await;
    assert!(nowhere.is_err(), "a workflow the place does not hold is not proposed a change: {nowhere:?}");
}

// ---- Lifecycles and rules ----

const RULE: &str = "version: 1
rule_id: deploy_watch
name: Watch the deploy
scope: everywhere
when: { step_starts: plan }
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
autonomy: acts
on_failure: { retries: 3, then: tell_agent }
";

/// The rule put in the copy the Job froze when it started.
async fn frozen_with_a_rule(rig: &Rig, id: &SessionId) {
    let piece = KeptPiece { kind: PieceKind::Rule, piece_id: "deploy_watch".into(), path: "/repo/.armada/rules/deploy_watch.yml".into(), text: RULE.into() };
    assert!(rig.fleet.store().lock().await.keep_job_copy_piece(id.as_str(), &piece).expect("kept"), "a Job freezes a copy when it starts");
}

fn rule_change(id: &SessionId, text: Option<&str>) -> ChangeJobCopy {
    ChangeJobCopy { session_id: id.clone(), kind: CopyPieceKind::Rule, id: "deploy_watch".into(), text: text.map(str::to_string) }
}

#[tokio::test]
async fn a_rule_in_a_jobs_copy_is_changed_dropped_and_marked_on_the_lanes() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    let nothing = rig.fleet.piece_changed(rule_change(&id, None)).await;
    assert!(refused_with(&nothing, "fleet.job_copy_piece_refused"), "no rule of that id to drop: {nothing:?}");

    frozen_with_a_rule(&rig, &id).await;
    let lanes = Arc::clone(&rig.fleet).get_job_lifecycles(None, Some(id.clone())).await.expect("lanes");
    assert_eq!(lanes.rules.len(), 1, "the lanes read the Job's rules");

    let louder = RULE.replace("retries: 3", "retries: 1");
    let copy = rig.fleet.piece_changed(rule_change(&id, Some(&louder))).await.expect("changed");
    assert!(copy.rules[0].text.contains("retries: 1"));
    assert_eq!((copy.changes[0].lane.as_str(), copy.changes[0].item.as_str(), copy.changes[0].text.as_str()), ("rule", "deploy_watch", "Changed by you"));

    let other = RULE.replace("rule_id: deploy_watch", "rule_id: another");
    let wrong = rig.fleet.piece_changed(rule_change(&id, Some(&other))).await;
    assert!(refused_with(&wrong, "fleet.job_copy_piece_refused"), "a text that declares another id is refused: {wrong:?}");
    let broken = rig.fleet.piece_changed(rule_change(&id, Some("not a rule"))).await;
    assert!(refused_with(&broken, "fleet.job_copy_piece_refused"), "{broken:?}");

    assert!(!copy.changes[0].dropped, "a rule the copy holds is not dropped");
    let copy = rig.fleet.piece_changed(rule_change(&id, None)).await.expect("dropped");
    assert!(copy.rules.is_empty());
    assert!(copy.changes.iter().any(|one| one.lane == "rule" && one.item == "deploy_watch" && one.dropped), "its mark says it was dropped: {:?}", copy.changes);
    let lanes = Arc::clone(&rig.fleet).get_job_lifecycles(None, Some(id.clone())).await.expect("lanes");
    assert!(lanes.changes.iter().any(|one| one.item == "deploy_watch" && one.dropped), "and so do the lanes");
    let again = rig.fleet.piece_changed(rule_change(&id, None)).await;
    assert!(refused_with(&again, "fleet.job_copy_piece_refused"), "{again:?}");
    assert!(says_of_copy(&rig).await.iter().any(|(says, by)| says == "rule dropped: deploy_watch" && by.as_deref() == Some("by you")));
}

#[tokio::test]
async fn a_piece_given_whole_with_no_id_takes_the_id_it_declares() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    frozen_with_a_rule(&rig, &id).await;
    let added = RULE.replace("rule_id: deploy_watch", "rule_id: ask_for_review");
    let blank = ChangeJobCopy { session_id: id.clone(), kind: CopyPieceKind::Rule, id: "  ".into(), text: Some(added) };
    let copy = rig.fleet.piece_changed(blank).await.expect("added under the id it declares");
    assert!(copy.rules.iter().any(|rule| rule.id == "ask_for_review"), "{:?}", copy.rules);
    assert_eq!(copy.changes.last().map(|one| (one.lane.as_str(), one.item.as_str(), one.text.as_str())), Some(("rule", "ask_for_review", "Added by you")));
    let nothing = ChangeJobCopy { session_id: id.clone(), kind: CopyPieceKind::Rule, id: String::new(), text: None };
    assert!(refused_with(&rig.fleet.piece_changed(nothing).await, "fleet.job_copy_piece_refused"), "a drop still names its piece");
    let unreadable = ChangeJobCopy { session_id: id, kind: CopyPieceKind::Rule, id: String::new(), text: Some("not a rule".into()) };
    assert!(refused_with(&rig.fleet.piece_changed(unreadable).await, "fleet.job_copy_piece_refused"), "and a file that does not read says so");
}

// ---- Keep for every Job, for a rule or a lifecycle ----

const TICKET: &str = "version: 1
lifecycle_id: ticket
name: Ticket
scope: everywhere
states:
  - { id: todo, label: To do, read: { tracker: [] } }
  - { id: closed, label: Closed, read: { tracker: [] } }
transitions:
  - { from: todo, to: closed }
ends: [closed]
";

fn keep_piece(id: &SessionId, kind: CopyPieceKind, piece: &str) -> KeepJobPiece {
    KeepJobPiece { session_id: id.clone(), kind, id: piece.into() }
}

/// What Kit holds in its folder for one kind of piece.
fn in_kit(rig: &Rig, folder: &str) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(rig.fleet.kit_pieces(folder)).map(|found| found.flatten().map(|one| one.path()).collect()).unwrap_or_default();
    files.sort();
    files
}

/// The proposal on the Job's Needs-you list.
async fn proposal_waiting(rig: &Rig, id: &SessionId) -> Option<ipc::WaitingItem> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.waiting_for.into_iter().find(|one| one.source == WaitingSource::Draft)
}

async fn answered(rig: &Rig, id: &SessionId, item: &str, choice: u32) -> Result<ipc::SessionRecord, Refusal> {
    Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting { session_id: id.clone(), item_id: item.to_string(), choice: Some(choice), text: None, mode: None, fixed: None })
        .await
}

/// A Job holding `deploy_watch` that it changed, so there is something of its own to keep.
async fn changed_a_rule(rig: &Rig, id: &SessionId) {
    frozen_with_a_rule(rig, id).await;
    rig.fleet.piece_changed(rule_change(id, Some(&RULE.replace("retries: 3", "retries: 1")))).await.expect("changed");
}

#[tokio::test]
async fn keeping_a_changed_rule_for_every_job_waits_on_the_owner_and_writes_nothing_until_he_approves() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    frozen_with_a_rule(&rig, &id).await;
    let untouched = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await;
    assert!(refused_with(&untouched, "fleet.job_copy_piece_refused"), "as it was frozen there is nothing of the Job's to keep: {untouched:?}");
    let unknown = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "nothing")).await;
    assert!(refused_with(&unknown, "fleet.job_copy_piece_refused"), "{unknown:?}");

    rig.fleet.piece_changed(rule_change(&id, Some(&RULE.replace("retries: 3", "retries: 1")))).await.expect("changed");
    let kept = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    assert_eq!(kept.scope, PieceScope::Within { id: rig.manifest.as_str().to_string() }, "at the repository's own level");
    assert!(!kept.replaces, "Kit holds none yet");
    assert!(in_kit(&rig, "rules").is_empty(), "nothing is written before he approves");

    let item = proposal_waiting(&rig, &id).await.expect("the Job's draft asks him");
    assert_eq!(item.text, "Rule deploy_watch");
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Approve", "Dismiss"]);
    let held = rig.fleet.store().lock().await.workflow_change(kept.change).unwrap().expect("held");
    assert_eq!((held.piece.as_str(), held.source.as_str(), held.state.as_str()), ("rule", "kit", store::CHANGE_READY));
    assert!(held.after_text.as_deref().is_some_and(|text| text.contains("retries: 1")), "the Job's change, not the frozen rule");

    let listed = rig.fleet.list_workflow_changes(WorkflowId::carried("deploy_watch"), "kit".into(), None).await.expect("listed");
    assert!(listed.changes.is_empty(), "a workflow's page never lists a piece");
    let approved = rig.fleet.workflow_change_approved(ipc::WorkflowChangeRef { change: kept.change }).await;
    assert!(matches!(&approved, Err(Refusal::NotFound(_))), "a piece is approved from the Job's draft: {approved:?}");

    answered(&rig, &id, &item.id, 0).await.expect("approved");
    let files = in_kit(&rig, "rules");
    assert_eq!(files.len(), 1, "Kit holds the rule now: {files:?}");
    let written = config::parse_rule(&files[0], &std::fs::read_to_string(&files[0]).expect("read")).expect("a rule its parser reads");
    assert_eq!(written.id().as_str(), "deploy_watch");
    assert_eq!(written.scope(), &core_model::LifecycleScope::Within(core_model::ScopeId::new(rig.manifest.as_str()).expect("a scope")));
    assert_eq!(written.on_failure().retries.times(), 1);
    assert_eq!(rig.fleet.store().lock().await.workflow_change(kept.change).unwrap().unwrap().state, store::CHANGE_APPROVED);
    assert!(proposal_waiting(&rig, &id).await.is_none(), "the item leaves the list");
}

#[tokio::test]
async fn dismissing_the_proposal_of_a_piece_writes_nothing() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    let kept = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    let item = proposal_waiting(&rig, &id).await.expect("waits").id;
    answered(&rig, &id, &item, 1).await.expect("dismissed");
    assert!(in_kit(&rig, "rules").is_empty());
    assert_eq!(rig.fleet.store().lock().await.workflow_change(kept.change).unwrap().unwrap().state, store::CHANGE_DISMISSED);
    assert!(proposal_waiting(&rig, &id).await.is_none());
}

/// The repository `keep_drafts.changed` named, once for each time the stream carries it in the next moments.
async fn keep_draft_events(watching: &mut api::Subscription) -> Vec<String> {
    let mut found = Vec::new();
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => {
                if let Event::KeepDraftsChanged(told) = delivered.event {
                    found.push(told.manifest_id.as_str().to_string());
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn a_pieces_keep_is_listed_for_its_repository_until_it_is_approved_and_each_move_is_told() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    let mut watching = rig.fleet.events().subscribe();
    assert!(rig.fleet.list_keep_drafts(None).await.expect("listed").drafts.is_empty(), "nothing is kept yet");

    Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    let listed = rig.fleet.list_keep_drafts(None).await.expect("listed");
    assert_eq!(listed.manifest_id, rig.manifest);
    assert_eq!(listed.drafts.len(), 1, "{listed:?}");
    assert_eq!((listed.drafts[0].kind, listed.drafts[0].id.as_str()), (CopyPieceKind::Rule, "deploy_watch"));
    assert!(listed.drafts[0].text.contains("retries: 1"), "the file Approve would save: {}", listed.drafts[0].text);
    assert_eq!(keep_draft_events(&mut watching).await, [rig.manifest.as_str()], "the page is told when one opens");

    let item = proposal_waiting(&rig, &id).await.expect("waits").id;
    answered(&rig, &id, &item, 0).await.expect("approved");
    assert!(rig.fleet.list_keep_drafts(None).await.expect("listed").drafts.is_empty(), "an approved draft is no longer waiting");
    assert_eq!(keep_draft_events(&mut watching).await, [rig.manifest.as_str()], "and the page is told when it goes");
}

#[tokio::test]
async fn a_pieces_keep_leaves_the_list_when_it_is_dismissed_and_the_page_is_told() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    let mut watching = rig.fleet.events().subscribe();
    Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    assert_eq!(rig.fleet.list_keep_drafts(None).await.expect("listed").drafts.len(), 1);
    keep_draft_events(&mut watching).await;

    let item = proposal_waiting(&rig, &id).await.expect("waits").id;
    answered(&rig, &id, &item, 1).await.expect("dismissed");
    assert!(rig.fleet.list_keep_drafts(None).await.expect("listed").drafts.is_empty());
    assert_eq!(keep_draft_events(&mut watching).await, [rig.manifest.as_str()]);
}

#[tokio::test]
async fn a_dropped_rule_is_not_kept() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    rig.fleet.piece_changed(rule_change(&id, None)).await.expect("dropped");
    let refused = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await;
    assert!(refused_with(&refused, "fleet.job_copy_piece_refused"), "{refused:?}");
    assert!(proposal_waiting(&rig, &id).await.is_none(), "nothing waits on him");
}

#[tokio::test]
async fn a_rule_kit_already_holds_is_replaced_only_if_kit_still_holds_what_the_proposal_began_from() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    let within = RULE.replace("scope: everywhere", &format!("scope: {}", rig.manifest.as_str()));
    let held = rig.fleet.kit_pieces("rules").join("deploy_watch.yml");
    std::fs::create_dir_all(rig.fleet.kit_pieces("rules")).expect("folder");
    std::fs::write(&held, &within).expect("planted");

    let kept = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    assert!(kept.replaces, "Kit holds a file for it already");
    assert_eq!(std::fs::read_to_string(&held).expect("read"), within, "and it is as it was");
    let item = proposal_waiting(&rig, &id).await.expect("waits").id;

    std::fs::write(&held, within.replace("retries: 3", "retries: 2")).expect("edited meanwhile");
    let stale = answered(&rig, &id, &item, 0).await;
    assert!(matches!(&stale, Err(Refusal::IllegalMove(why)) if why.code == "fleet.workflow_changed_since"), "{stale:?}");
    assert!(std::fs::read_to_string(&held).expect("read").contains("retries: 2"), "his edit is not overwritten");

    std::fs::write(&held, &within).expect("put back");
    answered(&rig, &id, &item, 0).await.expect("approved");
    assert!(std::fs::read_to_string(&held).expect("read").contains("retries: 1"), "the file that declared it is replaced in place");
    assert_eq!(in_kit(&rig, "rules").len(), 1);
}

#[tokio::test]
async fn keeping_what_kit_already_holds_is_refused() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    changed_a_rule(&rig, &id).await;
    Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await.expect("kept");
    let item = proposal_waiting(&rig, &id).await.expect("waits").id;
    answered(&rig, &id, &item, 0).await.expect("approved");
    let again = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Rule, "deploy_watch")).await;
    assert!(refused_with(&again, "fleet.job_copy_piece_refused"), "Kit holds this exactly now: {again:?}");
}

#[tokio::test]
async fn a_lifecycle_a_job_added_is_kept_within_its_repository_as_a_rule_is() {
    let rig = rigged();
    let id = on_the_rail(&rig).await;
    frozen_with_a_rule(&rig, &id).await;
    let added = ChangeJobCopy { session_id: id.clone(), kind: CopyPieceKind::Lifecycle, id: String::new(), text: Some(TICKET.into()) };
    rig.fleet.piece_changed(added).await.expect("added");

    let kept = Arc::clone(&rig.fleet).keep_job_piece(keep_piece(&id, CopyPieceKind::Lifecycle, "ticket")).await.expect("kept");
    assert_eq!((kept.kind, kept.id.as_str()), (CopyPieceKind::Lifecycle, "ticket"));
    assert_eq!(kept.scope, PieceScope::Within { id: rig.manifest.as_str().to_string() });
    assert!(in_kit(&rig, "lifecycles").is_empty());
    let item = proposal_waiting(&rig, &id).await.expect("waits");
    assert_eq!(item.text, "Lifecycle ticket");
    answered(&rig, &id, &item.id, 0).await.expect("approved");
    let files = in_kit(&rig, "lifecycles");
    assert_eq!(files.len(), 1, "{files:?}");
    let written = config::parse_lifecycle(&files[0], &std::fs::read_to_string(&files[0]).expect("read")).expect("a lifecycle its parser reads");
    assert_eq!(written.scope(), &core_model::LifecycleScope::Within(core_model::ScopeId::new(rig.manifest.as_str()).expect("a scope")));
}
