//! Which manifests a change gates, and why.

use std::path::Path;

use crate::gating::{gating, Gate, GateWhy};
use crate::manifest::Manifest;

fn root() -> Manifest {
    Manifest::parse(Path::new("armada.yml"), "version: 1\nid: root\n").expect("root")
}

fn workspace(dir: &str, depends_on: &[&str]) -> Manifest {
    let depends = match depends_on.is_empty() {
        true => String::new(),
        false => format!("depends_on: {depends_on:?}\n"),
    };
    let text = format!("version: 1\nid: {dir}\n{depends}");
    Manifest::parse_workspace(&Path::new(dir).join("armada.yml"), dir, &text, &root())
        .unwrap_or_else(|why| panic!("{dir}: {why}"))
}

fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

fn owns(dir: &str) -> Gate {
    Gate {
        dir: dir.to_string(),
        why: GateWhy::Owns,
    }
}

fn depends(dir: &str, glob: &str) -> Gate {
    Gate {
        dir: dir.to_string(),
        why: GateWhy::DependsOn(glob.to_string()),
    }
}

#[test]
fn the_nearest_workspace_up_the_tree_owns_a_path() {
    let nested = [
        workspace("packages/a", &[]),
        workspace("packages/a/sub", &[]),
    ];
    let gates = gating(&root(), &nested, &changed(&["packages/a/sub/x.ts"]));
    assert_eq!(gates, [owns("packages/a/sub")]);
    let gates = gating(&root(), &nested, &changed(&["packages/a/x.ts"]));
    assert_eq!(gates, [owns("packages/a")]);
}

#[test]
fn a_path_no_workspace_claims_falls_to_the_root() {
    let workspaces = [workspace("packages/a", &[])];
    let gates = gating(
        &root(),
        &workspaces,
        &changed(&["tools/release.sh", "Cargo.lock"]),
    );
    assert_eq!(gates, [owns("")]);
}

#[test]
fn a_directory_that_only_starts_with_a_workspaces_name_is_not_below_it() {
    let workspaces = [workspace("packages/a", &[])];
    let gates = gating(&root(), &workspaces, &changed(&["packages/a-two/x.ts"]));
    assert_eq!(gates, [owns("")]);
}

#[test]
fn a_change_in_one_workspace_does_not_gate_the_root_or_its_siblings() {
    let workspaces = [workspace("a", &[]), workspace("b", &[])];
    assert_eq!(
        gating(&root(), &workspaces, &changed(&["a/x.ts"])),
        [owns("a")]
    );
}

#[test]
fn a_workspace_is_gated_by_a_path_matching_its_depends_on() {
    let workspaces = [
        workspace("lib", &[]),
        workspace("a", &["lib/**"]),
        workspace("b", &["lib/**"]),
    ];
    let gates = gating(&root(), &workspaces, &changed(&["lib/src/x.ts"]));
    assert_eq!(
        gates,
        [owns("lib"), depends("a", "lib/**"), depends("b", "lib/**")]
    );
}

#[test]
fn depends_on_is_not_transitive() {
    // `a` depends on `lib` and `lib` on `core`: a change in `core` gates
    // `lib`, and `a` is gated only by what it wrote itself.
    let workspaces = [
        workspace("core", &[]),
        workspace("lib", &["core/**"]),
        workspace("a", &["lib/**"]),
    ];
    let gates = gating(&root(), &workspaces, &changed(&["core/x.ts"]));
    assert_eq!(gates, [owns("core"), depends("lib", "core/**")]);
}

#[test]
fn owning_a_path_is_reported_before_depending_on_it() {
    let workspaces = [workspace("a", &["a/**", "lib/**"])];
    let gates = gating(&root(), &workspaces, &changed(&["a/x.ts", "lib/y.ts"]));
    assert_eq!(gates, [owns(""), owns("a")]);
}

#[test]
fn the_first_matching_glob_is_the_one_named() {
    let workspaces = [workspace("a", &["pnpm-lock.yaml", "lib/**"])];
    let gates = gating(
        &root(),
        &workspaces,
        &changed(&["lib/y.ts", "pnpm-lock.yaml"]),
    );
    assert_eq!(gates, [owns(""), depends("a", "pnpm-lock.yaml")]);
}

#[test]
fn a_depends_on_glob_reaches_a_path_another_workspace_owns() {
    let workspaces = [workspace("lib", &[]), workspace("a", &["lib/vendored/**"])];
    let gates = gating(&root(), &workspaces, &changed(&["lib/vendored/x.ts"]));
    assert_eq!(gates, [owns("lib"), depends("a", "lib/vendored/**")]);
}

#[test]
fn a_change_touching_several_manifests_names_the_root_first_and_each_once() {
    let workspaces = [workspace("a", &[]), workspace("b", &[])];
    let gates = gating(
        &root(),
        &workspaces,
        &changed(&["b/x.ts", "a/x.ts", "a/y.ts", "README.md"]),
    );
    assert_eq!(gates, [owns(""), owns("a"), owns("b")]);
}

#[test]
fn a_root_with_depends_on_is_gated_by_it_like_any_manifest() {
    let text = "version: 1\nid: root\ndepends_on: [\"packages/**\"]\n";
    let root = Manifest::parse(Path::new("armada.yml"), text).expect("root");
    let workspaces = [workspace("packages/a", &[])];
    let gates = gating(&root, &workspaces, &changed(&["packages/a/x.ts"]));
    assert_eq!(gates, [depends("", "packages/**"), owns("packages/a")]);
}

#[test]
fn a_repository_with_no_workspaces_is_gated_by_the_root_alone() {
    assert_eq!(
        gating(&root(), &[], &changed(&["anything/at/all"])),
        [owns("")]
    );
}

#[test]
fn an_empty_change_gates_nothing() {
    let workspaces = [workspace("a", &[])];
    assert!(gating(&root(), &workspaces, &[]).is_empty());
}

fn with_checks(dir: &str, checks: &str) -> Manifest {
    let text = format!("version: 1\nid: {dir}\nchecks:\n{checks}");
    Manifest::parse_workspace(&Path::new(dir).join("armada.yml"), dir, &text, &root())
        .unwrap_or_else(|why| panic!("{dir}: {why}"))
}

fn named(found: &[(&Manifest, &str)]) -> Vec<String> {
    found
        .iter()
        .map(|(manifest, name)| format!("{}:{name}", manifest.dir()))
        .collect()
}

/// **Only the workspace a change reaches is asked**, and only for the Checks written for that
/// moment: a `pre_pr` Check of another workspace, or a plain one, is not run for it.
#[test]
fn the_pre_pr_checks_are_those_of_the_manifests_a_change_reaches() {
    let desktop = with_checks(
        "apps/desktop",
        "  format_check:\n    run: pnpm format:check\n    runs_at: pre_pr\n  build:\n    run: pnpm build\n",
    );
    let other = with_checks(
        "apps/other",
        "  format_check:\n    run: pnpm format:check\n    runs_at: pre_pr\n",
    );
    let workspaces = [desktop, other];
    let root = root();
    let found = crate::gating::pre_pr_checks(&root, &workspaces, &changed(&["apps/desktop/src/a.ts"]));
    assert_eq!(named(&found), ["apps/desktop:format_check"]);
    let none = crate::gating::pre_pr_checks(&root, &workspaces, &changed(&["README.md"]));
    assert!(none.is_empty(), "{:?}", named(&none));
}

/// A pre-PR Check's own `when` narrows it as any Check's does, against what its manifest owns.
#[test]
fn a_pre_pr_check_keeps_its_own_when() {
    let desktop = with_checks(
        "apps/desktop",
        "  format_check:\n    run: pnpm format:check\n    runs_at: pre_pr\n    when: [\"**/*.ts\"]\n",
    );
    let workspaces = [desktop];
    let root = root();
    let found = crate::gating::pre_pr_checks(&root, &workspaces, &changed(&["apps/desktop/notes.md"]));
    assert!(found.is_empty(), "{:?}", named(&found));
    let found = crate::gating::pre_pr_checks(&root, &workspaces, &changed(&["apps/desktop/src/a.ts"]));
    assert_eq!(named(&found), ["apps/desktop:format_check"]);
}
