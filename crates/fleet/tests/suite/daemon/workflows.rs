//! The workflows a case is run against.
//!
//! **Through the parser, never built by hand.** Each fixture below is either a
//! `testkit::Sketch` or the YAML a real `.armada/workflows/` file would hold,
//! and both end at `ResolvedWorkflow::resolve` — a workflow no real file could
//! have produced would prove nothing about the gate that reads it.

use std::collections::BTreeMap;

use config::Manifest;
use testkit::{Gate, Sketch};

/// Two steps: one gated on a non-empty diff, one gated on nothing.
///
/// The second is the shape the gate's own comment calls common rather than
/// edge — a step with no checks advances on evidence alone, and two of the four
/// sample workflows lean on it.
pub(super) fn two_steps() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
        },
        Sketch {
            id: "summarise",
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
        },
    ])
}

/// One workflow, keyed by its own id — what `fittings` holds when a case
/// wants nothing more than the fixture.
pub fn one(
    workflow: config::ResolvedWorkflow,
) -> BTreeMap<core_model::WorkflowId, config::ResolvedWorkflow> {
    let mut held = BTreeMap::new();
    held.insert(workflow.id().clone(), workflow);
    held
}

/// A one-step workflow with its own `workflow_id`, gated on nothing but the
/// evidence arriving — for a case that needs two workflows Fleet can tell
/// apart by id, and cannot tell apart by step id, since each step is named
/// for the workflow that declares it.
pub fn workflow_named(id: &str) -> config::ResolvedWorkflow {
    workflow_named_and(id)
}

fn workflow_named_and(id: &str) -> config::ResolvedWorkflow {
    let step_id = format!("only_in_{id}");
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nsteps:\n  - id: \
             {step_id}\n    label: \"{step_id}\"\n    evidence: {{submitted: {{type: diff}}}}\n    \
             delivers: true\n    advance_gate: auto\n"
        ),
        // The fixture names no model, so there is nothing for a roster to
        // offer. See `config::Roster`.
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

pub fn manifest() -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("a manifest that parses")
}

