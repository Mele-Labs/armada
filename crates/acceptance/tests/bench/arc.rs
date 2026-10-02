//! The apparatus Arc's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for `focus`'s reason: Arc asks who signed each row a
//! ruling wrote, which is a different question of the same machinery. It
//! reaches [`Bench`]'s private step log because it is a child module of the one
//! that declares it, and a second copy of that log is how the two would come to
//! disagree.

use core_model::{Actor, StepId, StepTarget};
use fleet::Clock;

use super::{Bench, Run};

impl Bench {
    /// Move one step, signed by whoever the caller names. [`Bench::settled`]
    /// names [`fleet::Ruling::signed_by`], which is what `fleet::dispatch`
    /// signs the stop with.
    pub fn step_moved_by(&self, run: &mut Run, step: &StepId, to: StepTarget, by: Actor) {
        let moved = run
            .job
            .transition_step(step, to, by, self.clock.now())
            .expect("a legal step move");
        self.step_moves.borrow_mut().push(moved.event);
        run.job = moved.job;
    }
}

impl super::plan::Planned {
    /// What the plan as it stands says to one more change a person makes,
    /// **without keeping it**, for [`super::plan::Planned::judged`]'s reason.
    pub fn judged_by_person(
        &self,
        change: core_model::PlanChange,
    ) -> Result<core_model::WorkPlan, core_model::PlanRefused> {
        let entry = core_model::PlanEntry {
            change,
            by: core_model::PlanAuthor::Person,
            at: core_model::Timestamp::from_rfc3339("2026-10-02T10:59:59.000Z"),
        };
        core_model::WorkPlan::after(self.plan().as_ref(), &entry)
    }
}

/// Who signed each step move, oldest first.
pub fn step_signers(bench: &Bench) -> Vec<Actor> {
    bench
        .step_moves
        .borrow()
        .iter()
        .map(|event| event.actor())
        .collect()
}

/// Feature's shape after slice 1b: a plan step, an `implement` that works its
/// tasks a Drone each, and a `tests` step that keeps one Drone and keeps
/// `follows_plan`. Read through `config`'s own parser, as a real
/// `.armada/workflows/` file is.
pub fn feature_with_a_drone_per_task() -> core_model::FrozenWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-feature-per-task.yml"),
        r#"
version: 1
workflow_id: feature-per-task
name: feature
structure: linear
steps:
  - id: plan
    label: "Plan the change"
    evidence: {submitted: {type: plan}}
    mechanical_checks:
      - { type: plan_recorded, min_tasks: 1 }
    delivers: false
    advance_gate: auto
  - id: implement
    label: "Implement"
    follows_plan: true
    drone_per_task: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    delivers: false
    advance_gate: auto
  - id: tests
    label: "Write tests"
    follows_plan: true
    evidence: {submitted: {type: diff}}
    delivers: true
    advance_gate: auto
"#,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
        .frozen()
        .clone()
}

/// Slice 2's shape: a plan step, an `implement` that works its tasks a Drone
/// each and may go round twice on its own, and a handoff. Resolved, because
/// [`Bench`] gates against a resolved workflow.
pub fn per_task_with_two_retries() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-per-task-with-retries.yml"),
        r#"
version: 1
workflow_id: per-task-with-retries
name: feature
structure: linear
steps:
  - id: plan
    label: "Plan the change"
    evidence: {submitted: {type: plan}}
    mechanical_checks:
      - { type: plan_recorded, min_tasks: 1 }
    delivers: false
    advance_gate: auto
  - id: implement
    label: "Implement"
    follows_plan: true
    drone_per_task: true
    retry_limit: 2
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    delivers: false
    advance_gate: auto
  - id: handoff
    label: "Hand off"
    delivers: true
    advance_gate: auto
"#,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

impl Bench {
    /// [`Bench::gate`] on one run of a group: the step's attempt the run is
    /// filed under, and the group's own spent runs, which its retry budget is
    /// asked against. `fleet::settling` reads both off the store.
    pub async fn gate_on_group(
        &self,
        run: &Run,
        step: &StepId,
        submitted: &verification::Submission,
        attempt: core_model::Attempt,
        spent: core_model::Spent,
    ) -> fleet::Ruling {
        let at = fleet::AtStep::named(self.workflow.frozen(), step, &run.worktree)
            .expect("a step of the workflow")
            .on_attempt(attempt, spent);
        self.gate_at(run, at, submitted).await
    }

    /// The two step moves a group's own round makes, into `retrying` and back
    /// into `running`, signed as `fleet::dispatch` signs them.
    pub fn went_round(&self, run: &mut Run, step: &StepId, ruling: &fleet::Ruling) {
        let fleet::Ruling::HandedBack { retrying, .. } = ruling else {
            panic!("only a hand-back goes round, and got {ruling:?}");
        };
        self.step_moved_by(
            run,
            step,
            StepTarget::Retrying(*retrying),
            ruling.signed_by(),
        );
        self.step_moved(run, step, StepTarget::Running);
    }
}
