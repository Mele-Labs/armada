//! Workspaces' claim: **a change runs the Checks of the manifests it touches,
//! and of the manifests that depend on what it touched, and no others.**
//!
//! Written first, in the vocabulary the claim needs: `config::gating`,
//! `config::Gate` and `Manifest::parse_workspace`. None of it exists when this
//! file is written, so the crate does not compile until A2 of the manifest
//! workspaces plan builds them. That is the red. It is not merged red.
//!
//! The repository is held as text: a root, `lib`, and `a` and `b`, which depend
//! on `lib`. Nothing is written to disk.
//!
//! | Not asserted yet | Carried by |
//! |---|---|
//! | A Check runs in its manifest's directory | A4 of the plan |
//! | The Job's `gate_manifests` is filled from the diff | A3 |
//! | Evidence records did-not-run and why | A4 |

use std::path::Path;

use config::{gating, Gate, GateWhy, Manifest};

const ROOT: &str = "version: 1\nid: root\nchecks:\n  rust:\n    run: cargo test\n";
const LIB: &str = "version: 1\nid: lib\nchecks:\n  test:\n    run: pnpm test\n";
const A: &str =
    "version: 1\nid: a\ndepends_on: [\"lib/**\"]\nchecks:\n  test:\n    run: pnpm test\n";
const B: &str =
    "version: 1\nid: b\ndepends_on: [\"lib/**\"]\nchecks:\n  test:\n    run: pnpm test\n";

struct Repository {
    root: Manifest,
    workspaces: Vec<Manifest>,
}

fn repository() -> Repository {
    let root = Manifest::parse(Path::new("armada.yml"), ROOT).expect("the root");
    let workspace = |dir: &str, text: &str| {
        Manifest::parse_workspace(Path::new(&format!("{dir}/armada.yml")), dir, text, &root)
            .unwrap_or_else(|why| panic!("{dir}: {why}"))
    };
    let workspaces = vec![workspace("lib", LIB), workspace("a", A), workspace("b", B)];
    Repository { root, workspaces }
}

fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

/// Every Check the change runs, as the manifest's id and the Check's name.
/// Sorted, because a claim about which run is not a claim about their order.
fn runs(repository: &Repository, paths: &[&str]) -> Vec<(String, String)> {
    let changed = changed(paths);
    let mut ran = Vec::new();
    for Gate { dir, .. } in gating(&repository.root, &repository.workspaces, &changed) {
        let manifest = std::iter::once(&repository.root)
            .chain(&repository.workspaces)
            .find(|manifest| manifest.dir() == dir)
            .expect("a gate names a manifest the repository holds");
        for name in manifest.checks_as_written() {
            let check = manifest.check(name).expect("a declared Check");
            if manifest.reaches(check, &changed) {
                ran.push((manifest.id().as_str().to_string(), name.clone()));
            }
        }
    }
    ran.sort();
    ran
}

fn pairs(wanted: &[(&str, &str)]) -> Vec<(String, String)> {
    wanted
        .iter()
        .map(|(id, check)| (id.to_string(), check.to_string()))
        .collect()
}

#[test]
fn a_change_in_one_workspace_runs_that_workspaces_checks_only() {
    let repository = repository();
    assert_eq!(
        runs(&repository, &["a/src/index.ts"]),
        pairs(&[("a", "test")])
    );
}

#[test]
fn a_change_in_a_shared_workspace_runs_it_and_what_depends_on_it() {
    let repository = repository();
    assert_eq!(
        runs(&repository, &["lib/src/index.ts"]),
        pairs(&[("a", "test"), ("b", "test"), ("lib", "test")])
    );
    let gates = gating(
        &repository.root,
        &repository.workspaces,
        &changed(&["lib/src/index.ts"]),
    );
    let why = |dir: &str| gates.iter().find(|gate| gate.dir == dir).map(|g| &g.why);
    assert_eq!(why("lib"), Some(&GateWhy::Owns));
    assert_eq!(why("a"), Some(&GateWhy::DependsOn("lib/**".to_string())));
}

#[test]
fn a_change_no_workspace_claims_runs_the_root_only() {
    let repository = repository();
    assert_eq!(
        runs(&repository, &["tools/release.sh"]),
        pairs(&[("root", "rust")])
    );
}

#[test]
fn every_result_names_its_manifest() {
    let repository = repository();
    let ran = runs(&repository, &["a/src/index.ts", "tools/release.sh"]);
    assert_eq!(ran, pairs(&[("a", "test"), ("root", "rust")]));
    assert!(ran.iter().all(|(id, _)| !id.is_empty()));
}
