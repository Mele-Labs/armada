//! What the agent is told of the workflow it follows: the plan's groups, with ids, gates and each
//! step's instructions, so the plan it records is the workflow.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::{StepGate, WorkflowId};

use crate::session_host::proposing::workflow_words;

fn feature() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: feature\nname: feature\nsteps:\n  \
         - id: propose\n    label: Propose\n    delivers: false\n    advance_gate: auto\n    gate: plan_approved\n    \
         instructions: |\n      Say what you will build.\n      Ask what is unclear.\n  \
         - id: walk\n    label: Walk\n    delivers: false\n    advance_gate: auto\n    gate: walk_approved\n  \
         - id: land\n    label: Land\n    delivers: true\n    advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = config::Manifest::parse(std::path::Path::new("fixture-armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\n")
        .expect("the fixture manifest parses");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"));
    [(resolved.id().clone(), resolved)].into_iter().collect()
}

#[test]
fn a_steps_id_gate_and_instructions_reach_the_text_one_line_a_group() {
    let held = feature();
    let id = held.keys().next().expect("one workflow").as_str().to_string();
    let words = workflow_words(&held, Some(&id), false).expect("a workflow Fleet holds is told");
    assert_eq!(
        words,
        "Fleet: this Job follows the feature workflow. Record your plan with these groups, in this order, each with its id and gate, and your own tasks under them:\n\
         1. propose, \"Propose\", gate plan_approved: Say what you will build. Ask what is unclear.\n\
         2. walk, \"Walk\", gate walk_approved\n\
         3. land, \"Land\", gate none"
    );
}

#[test]
fn a_change_made_to_a_running_job_says_now_follows() {
    let held = feature();
    let id = held.keys().next().expect("one workflow").as_str().to_string();
    let words = workflow_words(&held, Some(&id), true).expect("told");
    assert!(words.starts_with("Fleet: this Job now follows the feature workflow. Record your plan"), "{words}");
}

#[test]
fn a_job_that_follows_none_is_told_only_when_it_had_one() {
    let held = feature();
    assert_eq!(workflow_words(&held, None, false), None);
    assert_eq!(
        workflow_words(&held, None, true).as_deref(),
        Some("Fleet: this Job no longer follows a workflow. It is a custom Job.")
    );
}

/// config and core-model cannot depend on ipc, so their gate words are spelled twice. This is the
/// one place that sees both, and it holds the lists equal in words and in order.
#[test]
fn the_step_gate_words_are_the_plan_gate_words() {
    let step: Vec<&str> = StepGate::ALL.iter().map(|gate| gate.word()).collect();
    let plan: Vec<&str> = ipc::PlanGate::ALL.iter().map(|gate| gate.word()).collect();
    assert_eq!(step, plan);
}
