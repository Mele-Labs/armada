//! The workflow whose product is other Jobs, driven off the file that ships.
//!
//! **This is `#215`'s claim as slice 6 left it**: a person approves one Job
//! naming a milestone, its plan proposes a wave of real Jobs, and one press at
//! the plan's gate releases all of them. What a test cannot reach is the merge
//! and a Drone deciding anything — the pieces here are asked for by the fixture
//! rather than decided by a model, which is `bug_job.rs`'s own division and for
//! the same reason.
//!
//! **Read off disk rather than restated**, exactly as `tests::looping` reads
//! `design-plan.json`. Every mechanic underneath was already proved against
//! fixtures in `tests::sub_dispatch` and `tests::looping`; what is asserted here
//! is that the definition a person actually dispatches wires them together —
//! the grant on the step a person answers, the press, the stand-down, the
//! return, and the loop coming round to propose the next wave.
//!
//! # Why the fixture writes the artifacts
//!
//! Both steps declare `artifact_exists`, and the check reads the file's size. A
//! fake Drone writes nothing, so the worktree is seeded before each gate. That
//! is the same seam `tests::looping::wrote_the_plan` uses and it is not a
//! weakening: what the check proves is that Fleet opens the declared path, and
//! a test in which nothing is ever written proves it by never running it.

use core_model::{
    Approach, EvidenceType, JobId, JobStatus, NewTask, Origin, PlanChange, StepId, StepState,
};
use ipc::mcp::DispatchJob;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::resume::Redirection;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, manifest, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The definition this repository ships, resolved against the fixture Manifest
/// — which it needs nothing from, because it declares no Manifest Check at all.
///
/// **That is a property of the workflow rather than an accident of the
/// fixture.** A workflow whose product is Jobs has no diff to build and no
/// suite to run, so it resolves against any repository's Manifest, including
/// one that declares nothing.
///
/// The roster is the adapter's, because `plan` names a Judge model and a
/// definition naming a model the adapter does not offer is refused at parse.
pub(super) fn epic() -> config::ResolvedWorkflow {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.armada/workflows/epic.json");
    let text = std::fs::read_to_string(&path).expect("the shipped definition is there");
    let roster = config::Roster::of(adapters::HeadlessAgent::models());
    let def = config::WorkflowDef::parse(&path, &text, &roster)
        .unwrap_or_else(|refused| panic!("the shipped epic did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the shipped epic did not resolve: {refused}"))
}

/// A one-step workflow the children run under, so that a wave needs nothing of
/// the epic's own definition.
pub(super) fn a_piece() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-piece\nname: fixture-piece\n\
         steps:\n  - id: do_it\n    label: \"Do it\"\n    \
         evidence: {submitted: {type: facts_note}}\n    delivers: true\n    advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .expect("the child workflow parses");
    config::ResolvedWorkflow::resolve(&def, &manifest()).expect("it resolves")
}

/// A Fleet holding the epic and something for its children to run.
///
/// **The Judge answers rather than failing**, which the default fittings' Judge
/// does not: `plan` carries two criteria, so a Fleet that left the Judge cold
/// would stop at the first gate for a reason that is not this workflow's.
pub(super) fn a_fleet_running_epics(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["docs/plan.md"]));
    fittings.starting().workflows = [epic(), a_piece()]
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = std::sync::Arc::new(FakeJudge::with_no_objection());
    Fleet::assembled(fittings)
}

pub(super) fn planning() -> StepId {
    StepId::new("plan")
}

pub(super) fn rolling_up() -> StepId {
    StepId::new("roll_up")
}

/// A file at a path inside a Job's worktree, with something in it — which is
/// the whole of what `artifact_exists` reads. Before the Job's first dispatch,
/// the slot that dispatch will lease.
pub(super) fn wrote(home: &TempDir, job: &core_model::Job, at: &str, text: &str) {
    let spec = crate::tests::daemon::spec_held(home, job).expect("a legal spec");
    let tree = match spec.slot() {
        Some(_) => std::path::PathBuf::from(spec.worktree_path()),
        None => crate::tests::daemon::first_slot(home),
    };
    let path = tree.join(at);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a place for the artifact");
    std::fs::write(&path, text).expect("the artifact is written");
}

pub(super) fn document(claim: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Document,
        claimed: Claimed(claim),
        shown_by: ShownBy("the file this part delivered"),
        not_claimed: NotClaimed(""),
        review: None,
    }
}

pub(super) fn note(claim: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::FactsNote,
        claimed: Claimed(claim),
        shown_by: ShownBy("the file this part delivered"),
        not_claimed: NotClaimed(""),
        review: None,
    }
}

pub(super) fn asking(title: &str) -> DispatchJob {
    DispatchJob {
        title: title.to_string(),
        workflow: "fixture-piece".to_string(),
        brief: "what this piece is, from the parent that read the epic".to_string(),
        acceptance_criteria: vec!["it does the thing".to_string()],
        after: Vec::new(),
    }
}

pub(super) fn said(words: &str) -> Redirection {
    Redirection::saying(words).expect("a note with something in it")
}

/// A Job on the epic, approved, with a Drone on `plan` and a plan in its
/// worktree.
pub(super) async fn planning_a_wave(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_running_epics(home);
    let job = fleet
        .propose(a_proposal_for("run the Throughput milestone", "epic"))
        .await
        .expect("a Job at the approval gate");
    let id = job.id().clone();
    worktree_directory(home, &job);
    wrote(
        home,
        &job,
        ".armada/artifacts/plan.md",
        "# Wave 1\n\nOne piece.\n",
    );
    dispatched(&fleet, &id).await.expect("it dispatches");
    (fleet, id)
}

/// One recording of the wave's plan — `plan` keeps its split, `plan.md`, and
/// records the plan beside it, `#1006`, so `plan_recorded` gates its advance
/// exactly as it gates a step whose whole product is the plan.
fn a_wave_plan() -> PlanChange {
    PlanChange::Recorded {
        approach: Approach::new("Dispatch the one piece the split names").expect("an approach"),
        tasks: vec![NewTask::new("Port the parser", "", &[], "").expect("a title")],
    }
}

/// The plan is recorded, submitted, and the Job stands at the gate a person
/// answers.
pub(super) async fn presented_the_plan(fleet: &Fixture, job: &JobId) {
    fleet
        .change_plan(job, &a_wave_plan())
        .await
        .expect("the planning step may record the wave's plan");
    submitted_by_the_one(fleet, document("The split is drawn."))
        .await
        .expect("the plan is reported");
    fleet.turn().await.expect("the plan's gate runs");
}

/// **The press.** A person answers `plan` by naming every Job it proposed, and
/// the parent stands after it, waiting on them, holding no slot.
pub(super) async fn released_the_wave(fleet: &Fixture, job: &JobId, wave: &[&JobId]) {
    let body = ipc::ApproveWave {
        jobs: wave.iter().map(|one| ipc::JobId::from(*one)).collect(),
    };
    fleet
        .approve_wave(job, &body)
        .await
        .expect("a person approves the plan and its wave");
}

/// One child of the wave proposed, the plan presented, and the wave released.
pub(super) async fn a_wave_of_one(fleet: &Fixture, job: &JobId, home: &TempDir) -> JobId {
    let child = fleet
        .sub_dispatch(job, &asking("port the parser"))
        .await
        .expect("the plan proposes it");
    worktree_directory(home, &fleet.load(&child).await.expect("the child"));
    presented_the_plan(fleet, job).await;
    released_the_wave(fleet, job, &[&child]).await;
    child
}

/// The released child admitted, worked and finished, and the parent admitted
/// again on the step after the plan.
pub(super) async fn the_wave_ran(fleet: &Fixture) {
    fleet.turn().await.expect("the child is admitted");
    submitted_by_the_one(fleet, note("The piece is done."))
        .await
        .expect("the child reports");
    fleet.turn().await.expect("the child finishes");
    fleet.turn().await.expect("the parent is admitted again");
}

/// **The definition a person dispatches loads.** Parsing is not resolving and
/// resolving is not freezing, and the third is what a Job actually runs — a
/// definition that resolved and could not be frozen would fail at Job creation,
/// where the request is already made.
#[test]
fn the_shipped_epic_parses_resolves_and_freezes() {
    let workflow = epic();
    let frozen = workflow.frozen();
    let ids: Vec<&str> = frozen
        .steps()
        .iter()
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["plan", "roll_up"],
        "plan, which proposes the wave a person releases, and roll up",
    );
}

/// **The grant is on the step a person answers, and withheld after it.** The
/// plan's Drone proposes; what it proposes waits at its gate, stamped with the
/// pass, and `roll_up` may create nothing.
#[tokio::test]
async fn the_plan_proposes_the_wave_and_the_roll_up_may_create_nothing() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;

    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("the planning step proposes Jobs");
    let proposed = fleet.load(&child).await.expect("the child reads back");
    assert_eq!(proposed.status(), JobStatus::AwaitingApproval);
    assert_eq!(proposed.origin(), Origin::SubDispatched);
    let by = proposed.dispatched_by().expect("a child names its parent");
    assert_eq!(by.step_id.as_ref().map(StepId::as_str), Some("plan"));
    assert_eq!(by.pass, Some(1));

    worktree_directory(&home, &proposed);
    presented_the_plan(&fleet, &job).await;
    released_the_wave(&fleet, &job, &[&child]).await;
    the_wave_ran(&fleet).await;
    let standing = fleet.load(&job).await.expect("the Job reads back");
    assert_eq!(standing.current_step_id(), Some(&rolling_up()));

    let refused = fleet.sub_dispatch(&job, &asking("too late")).await;
    assert!(
        refused.is_err(),
        "the roll-up has no tool and no call: {refused:?}",
    );
}

/// **The whole claim in one run.** The wave is proposed and read at the gate,
/// approving the plan alone is refused while it waits, one press releases it,
/// the parent stands down rather than holding a slot its children need, and a
/// fresh Drone is on `roll_up` once they are done.
#[tokio::test]
async fn one_press_releases_the_wave_and_the_parent_returns_to_roll_it_up() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("the wave is proposed");
    worktree_directory(&home, &fleet.load(&child).await.expect("the child"));
    presented_the_plan(&fleet, &job).await;

    let held = fleet.load(&job).await.expect("the parent reads back");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(held.current_step_id(), Some(&planning()));
    let alone = fleet.approve_review(&job).await;
    assert!(
        alone.is_err_and(|why| why.to_string().contains("approve the wave")),
        "approving the plan alone would leave its Jobs waiting",
    );
    let partial = fleet
        .approve_wave(&job, &ipc::ApproveWave { jobs: Vec::new() })
        .await;
    assert!(
        partial.is_err(),
        "a press that names none of the wave releases none of it"
    );
    assert_eq!(
        fleet.load(&child).await.expect("the child").status(),
        JobStatus::AwaitingApproval,
        "and nothing moved"
    );

    released_the_wave(&fleet, &job, &[&child]).await;
    assert_eq!(
        fleet.load(&child).await.expect("the child").status(),
        JobStatus::Queued,
        "released by the one press, with nobody approving it alone"
    );
    let waiting = fleet.load(&job).await.expect("the parent reads back");
    assert_eq!(
        waiting.status(),
        JobStatus::Queued,
        "a parent waiting on the Jobs it proposed is queued, not running",
    );
    assert_eq!(
        waiting.current_step_id(),
        Some(&planning()),
        "standing after its plan"
    );
    assert!(
        !fleet.working_on().await.contains(&job),
        "and it is holding no slot, which is what makes the wait not a deadlock",
    );

    the_wave_ran(&fleet).await;
    let back = fleet.load(&job).await.expect("the parent reads back");
    assert_eq!(back.status(), JobStatus::Running);
    assert_eq!(
        back.current_step_id(),
        Some(&rolling_up()),
        "the step after the one that proposed, which is where the report is written",
    );
}

/// **The loop closes on the shipped file.** A person reading the roll-up asks
/// for another wave, the Job re-enters `plan`, and what it proposes there is
/// stamped as the second pass.
///
/// The cap is five and this spends one of them, so what is asserted here is the
/// return; `tests::looping::two_passes_and_then_the_cap_is_spent` is where the
/// arithmetic is pinned.
#[tokio::test]
async fn a_roll_up_that_asks_for_another_wave_re_enters_the_plan_and_proposes_wave_two() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    a_wave_of_one(&fleet, &job, &home).await;
    the_wave_ran(&fleet).await;

    wrote(
        &home,
        &fleet.load(&job).await.expect("the Job"),
        ".armada/artifacts/roll-up.md",
        "# Wave 1\n\nOne Job, and it landed.\n",
    );
    submitted_by_the_one(&fleet, document("What the wave did."))
        .await
        .expect("the roll-up is reported");
    fleet.turn().await.expect("the human gate runs");

    let held = fleet.load(&job).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(held.current_step_id(), Some(&rolling_up()));

    let round = fleet
        .request_changes(
            &job,
            &said("the next wave takes the two this one made ready"),
        )
        .await
        .expect("another wave is asked for");
    assert_eq!(
        round.current_step_id(),
        Some(&planning()),
        "the loop returns to the step that plans and proposes",
    );
    assert_eq!(
        round.step(&planning()).map(|step| step.state()),
        Some(StepState::Running),
    );

    let next = fleet
        .sub_dispatch(&job, &asking("take the next two"))
        .await
        .expect("the second pass proposes");
    let next = fleet.load(&next).await.expect("it reads back");
    assert_eq!(next.status(), JobStatus::AwaitingApproval);
    assert_eq!(
        next.dispatched_by().and_then(|by| by.pass),
        Some(2),
        "the wave a person reads it in"
    );
}

/// **The plan step is told that what it names becomes Jobs a person reads.**
/// A step definition has no field for prose, so this block is `fleet::terms`'
/// and it is keyed off the step's grant.
///
/// **The two reasons a piece waits are both named.** Only one of them survives
/// leaving the plan as a dependency edge; the other is held apart by the
/// drawing and by nothing else in Fleet, so a plan that does not tell them
/// apart is one whose reader cannot either.
#[test]
fn the_plan_is_told_that_what_it_proposes_waits_for_a_person() {
    let turn = turn_at(&planning());
    assert!(turn.contains("WHAT THIS PART PROPOSES"), "{turn}");
    assert!(
        turn.contains("its own worktree, its own agent and its own spend"),
        "the cost of a piece is the half a plan is written without: {turn}",
    );
    assert!(
        turn.contains("before approving") || turn.contains("approving them together"),
        "nothing runs until a person approves the wave: {turn}",
    );
    assert!(
        turn.contains("would write the same files"),
        "a sequencing edge is not a dependency edge: {turn}",
    );
    assert!(
        !turn.contains("WHAT THIS PART DECIDES"),
        "one block or the other, never both: {turn}",
    );
}

/// The roll-up gets neither, because it creates nothing and nothing after it
/// does. **Every other step of every other workflow is this
/// case**, which is why the block is conditional at all.
#[test]
fn the_roll_up_is_told_nothing_about_creating_jobs() {
    let turn = turn_at(&rolling_up());
    assert!(!turn.contains("WHAT THIS PART PROPOSES"), "{turn}");
    assert!(!turn.contains("WHAT THIS PART DECIDES"), "{turn}");
}

/// The opening turn a Drone on one step of the epic is given.
///
/// The Job is `tests::briefing`'s, which runs a different workflow — the brief
/// takes the two separately, and what is under test here is the workflow half.
fn turn_at(step: &StepId) -> String {
    crate::briefing::first_turn(
        &crate::tests::briefing::a_job(),
        epic().frozen(),
        step,
        &crate::crossing::Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string()
}
