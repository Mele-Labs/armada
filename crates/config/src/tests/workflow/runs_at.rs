//! `checks.<name>.runs_at`, read off a Manifest and placed by a workflow. #849.

use core_model::{ResolvedCheck, ResolvedStep, RunsAt, StepId};

use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, refusals, refused};
use crate::workflow::WorkflowDef;

/// A repository whose end-to-end Check runs only before handoff and whose
/// story build stays out of a Drone's run.
const DECLARED: &str = r#"
version: 1
id: shop
checks:
  build:
    run: cargo build
  storybook:
    run: pnpm build-storybook
    runs_at: gate
  e2e:
    run: pnpm e2e
    runs_at: handoff
"#;

/// A workflow shaped like the `feature` Armada once shipped: one sweeping step before the one that delivers.
const FEATURE: &str = "version: 1\nworkflow_id: feature\nname: feature\nsteps:\n\
    - id: propose\n  label: Propose\n  delivers: false\n  advance_gate: auto\n\
    - id: refine\n  label: Refine\n  retry_limit: 2\n  delivers: false\n  advance_gate: auto\n  \
    mechanical_checks: [{ type: every_manifest_check }, { type: diff_nonempty }]\n\
    - id: deliver\n  label: Deliver\n  delivers: true\n  advance_gate: human_always\n";

fn manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), DECLARED).expect("the fixture manifest")
}

fn resolved(text: &str) -> ResolvedWorkflow {
    let def = WorkflowDef::parse(
        &named("workflows/feature.json"),
        text,
        &crate::Roster::of(["haiku"]),
    )
    .expect("the workflow parses");
    ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared")
}

fn step<'a>(workflow: &'a ResolvedWorkflow, id: &str) -> &'a ResolvedStep {
    workflow.frozen().step(&StepId::new(id)).expect("the step")
}

fn names(checks: &[ResolvedCheck]) -> Vec<&str> {
    checks.iter().filter_map(ResolvedCheck::name).collect()
}

#[test]
fn each_value_is_read_and_absent_is_everywhere() {
    let read = manifest();
    assert_eq!(read.check("build").unwrap().runs_at(), RunsAt::Everywhere);
    assert_eq!(read.check("storybook").unwrap().runs_at(), RunsAt::Gate);
    assert_eq!(read.check("e2e").unwrap().runs_at(), RunsAt::Handoff);
}

#[test]
fn a_word_outside_the_three_is_refused_where_it_is_written() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    runs_at: sometimes\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.runs_at"),
        "{refused_here:?}"
    );
}

/// **The first line of the issue's definition of done.** A step that retries
/// and is not the last to sweep does not gate on the end-to-end Check. The
/// `feature` shape has one sweeping step, so this reads a workflow with two.
#[test]
fn an_earlier_sweeping_step_does_not_gate_on_a_handoff_check() {
    let text = "version: 1\nworkflow_id: two\nname: two\nsteps:\n\
                - id: first\n  label: First\n  retry_limit: 2\n  delivers: false\n  \
                advance_gate: auto\n  mechanical_checks: [{ type: every_manifest_check }]\n\
                - id: second\n  label: Second\n  delivers: false\n  advance_gate: auto\n  \
                mechanical_checks: [{ type: every_manifest_check }]\n";
    let workflow = resolved(text);
    let first = step(&workflow, "first");
    assert_eq!(names(first.checks()), ["build", "storybook"]);
    assert!(first.retry_limit() > 0, "the step retries");
    assert_eq!(
        workflow.frozen().held_for_handoff(first.id()),
        ["e2e"],
        "and the step says where it went"
    );
}

/// **The second line.** The last step gating on every Check before the one
/// that delivers takes it, once.
#[test]
fn the_step_before_handoff_is_the_one_that_runs_it() {
    let feature = resolved(FEATURE);
    assert_eq!(
        names(step(&feature, "refine").checks()),
        ["build", "storybook", "e2e"]
    );
    assert!(feature
        .frozen()
        .held_for_handoff(&StepId::new("refine"))
        .is_empty());
    let taking: usize = feature
        .steps()
        .iter()
        .flat_map(ResolvedStep::checks)
        .filter(|check| check.name() == Some("e2e"))
        .count();
    assert_eq!(taking, 1, "one step, never more");
}

/// A workflow that delivers nothing still runs a handoff Check somewhere: on
/// its last step gating on every Check.
#[test]
fn a_workflow_that_delivers_nothing_runs_it_on_its_last_sweeping_step() {
    let text = "version: 1\nworkflow_id: read\nname: read\nsteps:\n\
                - id: first\n  label: First\n  delivers: false\n  advance_gate: auto\n  \
                mechanical_checks: [{ type: every_manifest_check }]\n\
                - id: second\n  label: Second\n  delivers: false\n  advance_gate: auto\n  \
                mechanical_checks: [{ type: every_manifest_check }]\n";
    let workflow = resolved(text);
    assert_eq!(
        names(step(&workflow, "first").checks()),
        ["build", "storybook"]
    );
    assert_eq!(
        names(step(&workflow, "second").checks()),
        ["build", "storybook", "e2e"]
    );
}

/// A repository whose formatter check belongs to the moment a pull request opens.
const PRE_PR: &str = r#"
version: 1
id: shop
checks:
  build:
    run: cargo build
  format_check:
    run: pnpm format:check
    runs_at: pre_pr
"#;

#[test]
fn pre_pr_is_read_and_is_never_asked_mid_step() {
    let read = Manifest::parse(&named("armada.yml"), PRE_PR).expect("the fixture manifest");
    let declared = read.check("format_check").unwrap().runs_at();
    assert_eq!(declared, RunsAt::PrePr);
    assert_eq!(declared.as_wire(), "pre_pr");
    assert_eq!(RunsAt::from_wire("pre_pr"), Some(RunsAt::PrePr));
    assert!(!declared.mid_step());
}

/// **`every_manifest_check` never lifts one**, on any step: a pre-PR Check runs
/// where a pull request opens and nowhere else.
#[test]
fn no_step_gating_on_every_check_lifts_a_pre_pr_check() {
    let manifest = Manifest::parse(&named("armada.yml"), PRE_PR).expect("the fixture manifest");
    let def = WorkflowDef::parse(
        &named("workflows/feature.json"),
        FEATURE,
        &crate::Roster::of(["haiku"]),
    )
    .expect("the workflow parses");
    let workflow = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    let lifted: Vec<&str> = workflow
        .steps()
        .iter()
        .flat_map(ResolvedStep::checks)
        .filter_map(ResolvedCheck::name)
        .collect();
    assert!(lifted.contains(&"build"), "{lifted:?}");
    assert!(!lifted.contains(&"format_check"), "{lifted:?}");
}
