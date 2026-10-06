//! `resolve_gated`: `every_manifest_check` over a gating set.

use core_model::ResolvedCheck;

use super::parse;
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::named;

const EVERY: &str = "version: 1\nworkflow_id: feature\nname: feature\nsteps:\n  - id: implement\n    label: Implement\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n";

const ROOT: &str =
    "version: 1\nid: root\nchecks:\n  build:\n    run: cargo build\n  test:\n    run: cargo test\n";
const A: &str = "version: 1\nid: a\nchecks:\n  test:\n    run: pnpm test\n";

fn root() -> Manifest {
    Manifest::parse(&named("armada.yml"), ROOT).expect("the root")
}

fn a(root: &Manifest) -> Manifest {
    Manifest::parse_workspace(&named("packages/a/armada.yml"), "packages/a", A, root)
        .expect("a workspace")
}

fn labelled(checks: &[ResolvedCheck]) -> Vec<(&str, &str)> {
    checks
        .iter()
        .map(|check| (check.label(), check.manifest_dir()))
        .collect()
}

#[test]
fn a_repository_with_no_workspaces_resolves_exactly_as_before() {
    let def = parse(EVERY).expect("a workflow");
    let root = root();
    let old = ResolvedWorkflow::resolve(&def, &root).expect("resolves");
    let gated = ResolvedWorkflow::resolve_gated(&def, &root, &[&root]).expect("resolves");
    assert_eq!(old.frozen(), gated.frozen());
    assert_eq!(
        labelled(old.steps()[0].checks()),
        [("build", ""), ("test", "")]
    );
}

#[test]
fn every_manifest_check_expands_per_gating_manifest_in_the_order_given() {
    let def = parse(EVERY).expect("a workflow");
    let root = root();
    let a = a(&root);
    let resolved = ResolvedWorkflow::resolve_gated(&def, &root, &[&root, &a]).expect("resolves");
    assert_eq!(
        labelled(resolved.steps()[0].checks()),
        [("build", ""), ("test", ""), ("test", "packages/a")]
    );
    assert_eq!(resolved.steps()[0].checks()[2].run(), Some("pnpm test"));
}

#[test]
fn a_gating_set_without_the_root_leaves_its_checks_out() {
    let def = parse(EVERY).expect("a workflow");
    let root = root();
    let a = a(&root);
    let resolved = ResolvedWorkflow::resolve_gated(&def, &root, &[&a]).expect("resolves");
    assert_eq!(
        labelled(resolved.steps()[0].checks()),
        [("test", "packages/a")]
    );
}

#[test]
fn an_empty_gating_set_expands_to_no_checks_and_the_step_still_says_it_asked() {
    let def = parse(EVERY).expect("a workflow");
    let root = root();
    let resolved = ResolvedWorkflow::resolve_gated(&def, &root, &[]).expect("resolves");
    assert!(resolved.steps()[0].checks().is_empty());
    assert!(resolved.steps()[0].gates_on_every_check());
}
