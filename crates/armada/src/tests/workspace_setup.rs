//! Setup reads the workspaces below the root: each that loads is held and
//! gates, a file that does not is left out and named, and a repository with
//! none resolves as it always did.

use core_model::ResolvedCheck;

use crate::setup::Setup;
use crate::tests::setup::roster;
use crate::tests::TempDir;

const SWEEPING: &str = "version: 1\nworkflow_id: sweeping\nname: sweeping\nsteps:\n  \
     - id: only\n    label: \"Only step\"\n    evidence: {submitted: {type: diff}}\n    \
     delivers: true\n    advance_gate: auto\n    mechanical_checks:\n      \
     - { type: every_manifest_check }\n";

fn a_monorepo() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: 01WSROOT\nchecks:\n  test:\n    run: root test\n",
    );
    dir.write(".armada/workflows/sweeping.yml", SWEEPING);
    dir.write("lib/package.json", "{}");
    dir.write(
        "lib/armada.yml",
        "version: 1\nid: 01WSLIB\nchecks:\n  test:\n    run: lib test\n",
    );
    dir.write("bad/package.json", "{}");
    dir.write(
        "bad/armada.yml",
        "version: 1\nid: 01WSBAD\nsetup:\n  worktrees: 3\n",
    );
    dir
}

fn directories(setup: &Setup) -> Vec<(String, String)> {
    let workflow = setup
        .workflows()
        .values()
        .find(|workflow| workflow.id().as_str() == "sweeping")
        .expect("the repository's own definition");
    workflow.steps()[0]
        .checks()
        .iter()
        .filter_map(|check| match check {
            ResolvedCheck::ManifestCheck {
                manifest_dir, run, ..
            } => Some((manifest_dir.clone(), run.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_workspace_that_loads_is_held_and_its_checks_carry_its_directory() {
    let dir = a_monorepo();
    let setup =
        Setup::at(dir.path(), TempDir::new().path(), &roster()).expect("the repository serves");
    let held: Vec<&str> = setup.workspaces().iter().map(|one| one.dir()).collect();
    assert_eq!(held, ["lib"]);
    assert_eq!(
        directories(&setup),
        [
            (String::new(), String::from("root test")),
            (String::from("lib"), String::from("lib test")),
        ]
    );
}

#[test]
fn a_workspace_file_that_will_not_load_is_left_out_and_named() {
    let dir = a_monorepo();
    let setup =
        Setup::at(dir.path(), TempDir::new().path(), &roster()).expect("the repository serves");
    let refused = setup.workspaces_refused();
    assert_eq!(refused.len(), 1);
    assert!(
        refused[0].0.ends_with("bad/armada.yml"),
        "{:?}",
        refused[0].0
    );
}

#[test]
fn a_repository_with_no_workspaces_holds_none_and_checks_carry_no_directory() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: 01WSROOT\nchecks:\n  test:\n    run: root test\n",
    );
    dir.write(".armada/workflows/sweeping.yml", SWEEPING);
    let setup =
        Setup::at(dir.path(), TempDir::new().path(), &roster()).expect("the repository serves");
    assert!(setup.workspaces().is_empty() && setup.workspaces_refused().is_empty());
    assert_eq!(
        directories(&setup),
        [(String::new(), String::from("root test"))]
    );
}
