//! Loading a repository's workspace manifests: which load, which are left out
//! and said once, and that a repository with none is as it was.

use std::path::Path;

use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::ResolvedCheck;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;
use crate::workspaces::load;

const WORKFLOW: &str =
    "version: 1\nworkflow_id: fixture\nname: fixture\nsteps:\n  - id: implement\n    \
    label: Implement\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    \
    advance_gate: auto\n    mechanical_checks: [{ type: every_manifest_check }]\n";

const ROOT: &str = "version: 1\nid: 01LOADROOT\nchecks:\n  test:\n    run: \"true\"\n";
const GOOD: &str = "version: 1\nid: ";
/// `setup.worktrees` is the root's alone, so a workspace file saying it is refused.
const BAD: &str = "version: 1\nid: 01LOADBAD\nsetup:\n  worktrees: 3\n";

fn write(home: &TempDir, path: &str, text: &str) {
    let file = home.path().join(path);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(file, text).expect("a file");
}

/// A root, workspaces `lib` and `a` that load, and `bad` that does not.
fn scratch() -> (TempDir, Manifest) {
    let home = TempDir::new();
    write(&home, "armada.yml", ROOT);
    for (dir, id) in [("lib", "01LOADLIB"), ("a", "01LOADA")] {
        write(&home, &format!("{dir}/package.json"), "{}");
        let text = format!("{GOOD}{id}\nchecks:\n  test:\n    run: \"true\"\n");
        write(&home, &format!("{dir}/armada.yml"), &text);
    }
    write(&home, "bad/package.json", "{}");
    write(&home, "bad/armada.yml", BAD);
    let root = Manifest::load(&home.path().join("armada.yml")).expect("the root loads");
    (home, root)
}

fn ids(manifests: &[Manifest]) -> Vec<&str> {
    manifests.iter().map(|one| one.id().as_str()).collect()
}

#[test]
fn every_workspace_that_loads_is_held_and_the_one_that_does_not_is_left_out() {
    let (home, root) = scratch();
    let read = load(home.path(), &root);
    assert_eq!(ids(&read.manifests), ["01LOADA", "01LOADLIB"]);
    assert_eq!(read.manifests[0].dir(), "a");
    assert_eq!(read.refused.len(), 1);
    assert!(read.refused[0].0.ends_with("bad/armada.yml"));
}

#[test]
fn checks_resolve_over_the_root_and_every_workspace_with_their_directory() {
    let (home, root) = scratch();
    let read = load(home.path(), &root);
    let def = WorkflowDef::parse(
        Path::new("fixture.yml"),
        WORKFLOW,
        &Roster::offering_nothing(),
    )
    .expect("the workflow parses");
    let all: Vec<&Manifest> = std::iter::once(&root).chain(&read.manifests).collect();
    let workflow = ResolvedWorkflow::resolve_gated(&def, &root, &all).expect("resolves");
    let dirs: Vec<String> = workflow.frozen().steps()[0]
        .checks()
        .iter()
        .filter_map(|check| match check {
            ResolvedCheck::ManifestCheck { manifest_dir, .. } => Some(manifest_dir.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(dirs, ["", "a", "lib"]);
}

#[test]
fn a_repository_with_no_workspaces_resolves_as_it_always_did() {
    let home = TempDir::new();
    write(&home, "armada.yml", ROOT);
    write(&home, "package.json", "{}");
    let root = Manifest::load(&home.path().join("armada.yml")).expect("the root loads");
    let read = load(home.path(), &root);
    assert!(read.manifests.is_empty() && read.refused.is_empty());
    let def = WorkflowDef::parse(
        Path::new("fixture.yml"),
        WORKFLOW,
        &Roster::offering_nothing(),
    )
    .expect("the workflow parses");
    let over_all = ResolvedWorkflow::resolve_gated(&def, &root, &[&root]).expect("resolves");
    let alone = ResolvedWorkflow::resolve(&def, &root).expect("resolves");
    assert_eq!(
        format!("{:?}", over_all.frozen()),
        format!("{:?}", alone.frozen())
    );
}

async fn drained(watching: &mut api::Subscription) -> Vec<ipc::Event> {
    let mut seen = Vec::new();
    while let Ok(Some(api::Next::Send(delivered))) =
        tokio::time::timeout(std::time::Duration::from_millis(200), watching.next()).await
    {
        seen.push(delivered.event);
    }
    seen
}

/// A root re-read reads the workspaces again: a bad file is said once, as a
/// reading naming that file, and the others stay served.
#[tokio::test]
async fn a_root_reread_reloads_the_workspaces_and_says_a_bad_one_once() {
    let (home, root) = scratch();
    let fleet_home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&fleet_home, FakeWorkProduct::changed(&[])));
    let repository = home.path().to_string_lossy().to_string();
    fleet
        .repositories()
        .add(Located {
            root: repository.clone(),
            records_root: home.path().join("records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(root, Default::default())),
        })
        .expect("served");
    let served = fleet
        .repositories()
        .serving("01LOADROOT")
        .expect("the repository is served");
    assert!(served.workspaces().is_empty());

    let mut watching = fleet.events().subscribe();
    let quiet = ipc::ManifestReading {
        path: format!("{repository}/armada.yml"),
        at: ipc::Instant::from(&fleet.now()),
        moved: Vec::new(),
        at_restart: Vec::new(),
        refused: None,
    };
    fleet.reread(&repository, quiet);

    assert_eq!(ids(&served.workspaces()), ["01LOADA", "01LOADLIB"]);
    let seen = drained(&mut watching).await;
    let said: Vec<&ipc::ManifestReading> = seen
        .iter()
        .filter_map(|event| match event {
            ipc::Event::ManifestReread(reading) if reading.refused.is_some() => Some(reading),
            _ => None,
        })
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].path.ends_with("bad/armada.yml"));
}
