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
