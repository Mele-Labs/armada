//! Which manifests gate a Job in a repository that has workspaces, and which of
//! them freeze it. A repository with none records nothing, as it always did.

use config::Manifest;
use core_model::{
    Facts, GateManifest, GateOutcome, Job, JobId, JobNumber, ManifestId, ModelName, NewJob,
    NotRunReason, StepId, StepSeed, Timestamp, Title, TopLevelOrigin, Ulid, Urgency,
};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::gating::gate_manifests;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::gate::workflow;
use crate::tests::tmp::TempDir;

fn parsed(path: &str, dir: Option<&str>, text: &str, root: Option<&Manifest>) -> Manifest {
    let path = std::path::Path::new(path);
    match (dir, root) {
        (Some(dir), Some(root)) => Manifest::parse_workspace(path, dir, text, root),
        _ => Manifest::parse(path, text),
    }
    .expect("the manifest parses")
}

fn root() -> Manifest {
    parsed("armada.yml", None, "version: 1\nid: 01GATINGROOT\n", None)
}

fn workspaces(root: &Manifest, a_says: &str) -> Vec<Manifest> {
    let a = format!("version: 1\nid: 01GATINGA\n{a_says}");
    vec![
        parsed("packages/a/armada.yml", Some("packages/a"), &a, Some(root)),
        parsed(
            "packages/b/armada.yml",
            Some("packages/b"),
            "version: 1\nid: 01GATINGB\ndepends_on: [\"packages/a/**\"]\n",
            Some(root),
        ),
    ]
}

fn served(
    fleet: &Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>,
    home: &TempDir,
    a_says: &str,
) -> crate::repositories::Served {
    let root = root();
    let set_up =
        SetUp::of(root.clone(), Default::default()).with_workspaces(workspaces(&root, a_says));
    fleet
        .repositories()
        .add(Located {
            root: home.path().join("mono").to_string_lossy().to_string(),
            records_root: home
                .path()
                .join("mono-records")
                .to_string_lossy()
                .to_string(),
            set_up: Some(set_up),
        })
        .expect("served");
    fleet
        .repositories()
        .serving("01GATINGROOT")
        .expect("the repository is served")
}

fn ids(gates: &[GateManifest]) -> Vec<&str> {
    gates.iter().map(|gate| gate.manifest_id.as_str()).collect()
}

fn strings(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

#[test]
fn a_repository_with_no_workspaces_records_no_gates() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let served = fleet
        .repositories()
        .first()
        .expect("the fixture repository");
    assert!(gate_manifests(&served, None).is_empty());
    assert!(gate_manifests(&served, Some(&strings(&["src/lib.rs"]))).is_empty());
}

#[test]
fn paths_not_yet_known_gate_every_manifest() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let served = served(&fleet, &home, "");
    let gates = gate_manifests(&served, None);
    assert_eq!(ids(&gates), ["01GATINGROOT", "01GATINGA", "01GATINGB"]);
}

#[test]
fn known_paths_gate_the_owner_and_what_depends_on_it() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let served = served(&fleet, &home, "");
    let gates = gate_manifests(&served, Some(&strings(&["packages/a/x.ts"])));
    assert_eq!(ids(&gates), ["01GATINGA", "01GATINGB"]);
    let gates = gate_manifests(&served, Some(&strings(&["crates/x.rs"])));
    assert_eq!(ids(&gates), ["01GATINGROOT"]);
}

/// **An empty diff gates nothing**, decided 6 Oct 2026.
#[test]
fn determined_to_write_nothing_gates_nothing() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let served = served(&fleet, &home, "");
    assert!(gate_manifests(&served, Some(&[])).is_empty());
}

fn owned_by_the_root_and_gated_by(gate: &str) -> Job {
    let created_at = Timestamp::from_rfc3339("2026-09-13T09:00:00.000Z");
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01TESTWORKSPACEGATE0001")),
            title: Title::new("a change gated by a workspace").expect("a title"),
            workflow: workflow("/usr/bin/true").frozen().clone(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01GATINGROOT")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("the-configured-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: vec![StepSeed {
                step_id: StepId::new("implement"),
                ordinal: 0,
            }],
            dependencies: Vec::new(),
            gate_manifests: vec![GateManifest {
                manifest_id: ManifestId::carried(Ulid::carried(gate)),
                outcome: GateOutcome::DidNotRun(NotRunReason::PathConditionUnmet),
            }],
            write_targets: None,
            subject: None,
            redispatched_from: None,
            proposal_id: None,
            number: JobNumber::carried(1),
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        created_at,
    )
}

#[test]
fn a_frozen_workspace_freezes_the_job_it_gates_and_an_open_one_does_not() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    served(&fleet, &home, "freeze: true\n");
    let held = fleet.frozen_by(&owned_by_the_root_and_gated_by("01GATINGA"));
    assert_eq!(
        held.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        ["01GATINGA"]
    );
    assert!(fleet
        .frozen_by(&owned_by_the_root_and_gated_by("01GATINGB"))
        .is_empty());
}

/// A write target naming a workspace's directory is owned by that workspace.
#[test]
fn a_write_target_naming_a_workspace_directory_is_owned_by_it() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let served = served(&fleet, &home, "");
    for named in ["packages/a", "packages/a/", "/packages/a"] {
        let gates = gate_manifests(&served, Some(&strings(&[named])));
        assert_eq!(ids(&gates), ["01GATINGA", "01GATINGB"], "{named}");
    }
    let gates = gate_manifests(&served, Some(&strings(&["packages/ab"])));
    assert_eq!(
        ids(&gates),
        ["01GATINGROOT"],
        "a sibling that shares a prefix"
    );
}
