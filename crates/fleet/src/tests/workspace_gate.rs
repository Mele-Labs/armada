//! A step gate over a repository with workspaces: a Check runs in its
//! manifest's directory, only where the change gates that manifest, and each
//! gating manifest records what it came to.
//!
//! The repository has a root and three workspaces, `lib`, `a` and `b`, where
//! `a` and `b` depend on `lib`. Each Check is `touch <marker>`, so where it ran
//! is a file on disk rather than a row that could be written without it.

use std::path::Path;

use adapter_traits::Footprint;
use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::{CheckOutcome, GateOutcome, NotRunReason};
use testkit::FakeWorkProduct;
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::gated::Gated;
use crate::policy::Policies;
use crate::tests::gate::{diff_evidence, judging};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;

const STEP: &str =
    "version: 1\nworkflow_id: fixture\nname: fixture\nsteps:\n  - id: implement\n    \
    label: Implement\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    \
    advance_gate: auto\n    mechanical_checks: [{ type: every_manifest_check }]\n";

struct Repository {
    root: Manifest,
    below: Vec<Manifest>,
}

/// `<dir>`'s manifest: its own `test` Check leaving `<dir>-ran` behind, and
/// whatever else `more` says.
fn text(id: &str, marker: &str, more: &str, extra: &str) -> String {
    format!("version: 1\nid: {id}\n{more}checks:\n  test:\n    run: \"/usr/bin/touch {marker}\"\n{extra}")
}

fn repository(a_says: &str) -> Repository {
    let root = Manifest::parse(
        Path::new("armada.yml"),
        &text("01WSROOT", "root-ran", "", ""),
    )
    .expect("the root parses");
    let depends = "depends_on: [\"lib/**\"]\n";
    let workspace = |dir: &str, id: &str, more: &str, extra: &str| {
        Manifest::parse_workspace(
            &Path::new(dir).join("armada.yml"),
            dir,
            &text(id, &format!("{dir}-ran"), more, extra),
            &root,
        )
        .unwrap_or_else(|why| panic!("{dir} did not parse: {why}"))
    };
    let below = vec![
        workspace("lib", "01WSLIB", "", ""),
        workspace("a", "01WSA", depends, a_says),
        workspace("b", "01WSB", depends, ""),
    ];
    Repository { root, below }
}

impl Repository {
    fn workflow(&self) -> ResolvedWorkflow {
        let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
            .expect("the workflow parses");
        let all: Vec<&Manifest> = std::iter::once(&self.root).chain(&self.below).collect();
        ResolvedWorkflow::resolve_gated(&def, &self.root, &all).expect("resolves")
    }

    fn gated(&self, changed: &[&str]) -> Gated {
        let paths: Vec<String> = changed.iter().map(|path| path.to_string()).collect();
        Gated::of(&self.root, &self.below, &paths)
    }
}

/// A worktree with a directory per manifest.
fn checkout() -> TempDir {
    let home = TempDir::new();
    for dir in ["lib", "a", "b"] {
        std::fs::create_dir_all(home.path().join(dir)).expect("a workspace directory");
    }
    home
}

async fn ruled(
    workflow: &ResolvedWorkflow,
    home: &TempDir,
    changed: &[&str],
    gated: Option<&Gated>,
) -> Ruling {
    ruled_over(workflow.frozen(), home, changed, gated).await
}

async fn ruled_over(
    workflow: &core_model::FrozenWorkflow,
    home: &TempDir,
    changed: &[&str],
    gated: Option<&Gated>,
) -> Ruling {
    let worktree = adapter_traits::Worktree::at(
        home.path().to_string_lossy().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let at_step = AtStep::first(workflow, &worktree)
        .expect("a first step")
        .over(gated);
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        &[],
        crate::gate::Began::At(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(changed),
        CheckBudget::of(std::time::Duration::from_secs(20)),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &judging(),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await
}

/// Which markers the Checks left, by path under the worktree.
fn left(home: &TempDir) -> Vec<&'static str> {
    ["root-ran", "lib/lib-ran", "a/a-ran", "b/b-ran"]
        .into_iter()
        .filter(|marker| home.path().join(marker).exists())
        .collect()
}

fn outcomes(ruling: &Ruling) -> Vec<(String, CheckOutcome)> {
    ruling
        .checks()
        .iter()
        .map(|row| (row.name.clone(), row.outcome))
        .collect()
}

#[tokio::test]
async fn a_change_in_a_runs_a_in_its_directory_and_nothing_else() {
    let (repository, home) = (repository(""), checkout());
    let changed = ["a/src/x.ts"];
    let ruling = ruled(
        &repository.workflow(),
        &home,
        &changed,
        Some(&repository.gated(&changed)),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["a/a-ran"], "a ran in a's directory");
    assert_eq!(
        outcomes(&ruling),
        [
            ("test".to_string(), CheckOutcome::Skipped),
            ("lib:test".to_string(), CheckOutcome::Skipped),
            ("a:test".to_string(), CheckOutcome::Passed),
            ("b:test".to_string(), CheckOutcome::Skipped),
        ],
        "the same name in four manifests is four rows"
    );
}

#[tokio::test]
async fn a_change_in_lib_runs_lib_a_and_b() {
    let (repository, home) = (repository(""), checkout());
    let changed = ["lib/src/x.ts"];
    let ruling = ruled(
        &repository.workflow(),
        &home,
        &changed,
        Some(&repository.gated(&changed)),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["lib/lib-ran", "a/a-ran", "b/b-ran"]);
}

#[tokio::test]
async fn a_change_the_root_owns_runs_the_root_in_the_worktree() {
    let (repository, home) = (repository(""), checkout());
    let changed = ["crates/x.rs"];
    let ruling = ruled(
        &repository.workflow(),
        &home,
        &changed,
        Some(&repository.gated(&changed)),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["root-ran"]);
}

/// **An empty diff gates nothing** where the repository has workspaces.
#[tokio::test]
async fn an_empty_diff_runs_nothing() {
    let (repository, home) = (repository(""), checkout());
    let ruling = ruled(
        &repository.workflow(),
        &home,
        &[],
        Some(&repository.gated(&[])),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert!(left(&home).is_empty(), "no Check ran");
    assert!(ruling
        .checks()
        .iter()
        .all(|row| row.outcome == CheckOutcome::Skipped));
}

/// **A repository with no workspaces is unchanged**: no `Gated`, and an empty
/// diff still runs a Check that declares no `when`.
#[tokio::test]
async fn with_no_workspaces_an_empty_diff_still_runs_the_root() {
    let home = checkout();
    let root = Manifest::parse(
        Path::new("armada.yml"),
        &text("01WSROOT", "root-ran", "", ""),
    )
    .expect("the root parses");
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let workflow = ResolvedWorkflow::resolve(&def, &root).expect("resolves");

    let ruling = ruled(&workflow, &home, &[], None).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["root-ran"]);
    assert_eq!(
        outcomes(&ruling),
        [("test".to_string(), CheckOutcome::Passed)]
    );
}

/// A prerequisite runs in its Check's directory, once per manifest: the root
/// and `a` each name a `prepare` and each Check finds its own manifest's.
#[tokio::test]
async fn a_prerequisite_runs_in_the_directory_of_the_check_that_names_it() {
    let (root, below) = {
        let root = Manifest::parse(
            Path::new("armada.yml"),
            "version: 1\nid: 01WSROOT\ncommands:\n  prepare:\n    run: \"/usr/bin/touch prepared\"\n\
             checks:\n  test:\n    run: \"/bin/test -f prepared\"\n    requires: [prepare]\n",
        )
        .expect("the root parses");
        let a = Manifest::parse_workspace(
            Path::new("a/armada.yml"),
            "a",
            "version: 1\nid: 01WSA\ncommands:\n  prepare:\n    run: \"/usr/bin/touch prepared-in-a\"\n\
             checks:\n  test:\n    run: \"/bin/test -f prepared-in-a\"\n    requires: [prepare]\n",
            &root,
        )
        .expect("a parses");
        (root, vec![a])
    };
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let workflow =
        ResolvedWorkflow::resolve_gated(&def, &root, &[&root, &below[0]]).expect("resolves");
    let home = checkout();
    let changed = ["crates/x.rs", "a/x.ts"];
    let paths: Vec<String> = changed.iter().map(|path| path.to_string()).collect();
    let gated = Gated::of(&root, &below, &paths);

    let ruling = ruled(&workflow, &home, &changed, Some(&gated)).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert!(home.path().join("prepared").exists());
    assert!(home.path().join("a/prepared-in-a").exists());
    assert!(!home.path().join("a/prepared").exists());
}

fn failing() -> &'static str {
    "checks:\n  test:\n    run: \"/usr/bin/false\"\n"
}

#[tokio::test]
async fn each_gating_manifest_records_what_it_came_to() {
    // `a`'s Check fails; `b` depends on lib and has a Check `when` can never
    // meet from a dependency; the root and lib do not gate a change in `a`.
    let home = checkout();
    let root = Manifest::parse(
        Path::new("armada.yml"),
        &text("01WSROOT", "root-ran", "", ""),
    )
    .expect("the root parses");
    let workspace = |dir: &str, id: &str, body: &str| {
        Manifest::parse_workspace(
            &Path::new(dir).join("armada.yml"),
            dir,
            &format!("version: 1\nid: {id}\ndepends_on: [\"lib/**\"]\n{body}"),
            &root,
        )
        .expect("a workspace parses")
    };
    let below = vec![
        workspace("lib", "01WSLIB", ""),
        workspace("a", "01WSA", failing()),
        workspace(
            "b",
            "01WSB",
            "checks:\n  test:\n    run: \"/usr/bin/true\"\n    when: [\"src/**\"]\n",
        ),
    ];
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let all: Vec<&Manifest> = std::iter::once(&root).chain(&below).collect();
    let workflow = ResolvedWorkflow::resolve_gated(&def, &root, &all).expect("resolves");
    let changed = ["lib/x.ts", "a/x.ts"];
    let paths: Vec<String> = changed.iter().map(|path| path.to_string()).collect();
    let gated = Gated::of(&root, &below, &paths);

    let ruling = ruled(&workflow, &home, &changed, Some(&gated)).await;
    let came_to: Vec<(String, GateOutcome)> = gated
        .outcomes(workflow.steps()[0].checks(), ruling.checks())
        .into_iter()
        .map(|gate| (gate.manifest_id.as_str().to_string(), gate.outcome))
        .collect();

    assert_eq!(
        came_to,
        [
            // lib declares no Check on this step.
            (
                "01WSLIB".to_string(),
                GateOutcome::DidNotRun(NotRunReason::NotDeclared)
            ),
            ("01WSA".to_string(), GateOutcome::RanAndFailed),
            // lib's change is not under b's own `src/**`.
            (
                "01WSB".to_string(),
                GateOutcome::DidNotRun(NotRunReason::PathConditionUnmet)
            ),
        ]
    );
}

/// `when` in a workspace file reads paths relative to the workspace.
#[tokio::test]
async fn a_workspaces_when_reads_paths_relative_to_it() {
    let (repository, home) = (
        repository("  extra:\n    run: \"/usr/bin/touch a-extra\"\n    when: [\"src/**\"]\n"),
        checkout(),
    );
    let changed = ["a/docs/x.md"];
    ruled(
        &repository.workflow(),
        &home,
        &changed,
        Some(&repository.gated(&changed)),
    )
    .await;
    assert!(!home.path().join("a/a-extra").exists(), "docs is not src");

    let changed = ["a/src/x.ts"];
    ruled(
        &repository.workflow(),
        &home,
        &changed,
        Some(&repository.gated(&changed)),
    )
    .await;
    assert!(home.path().join("a/a-extra").exists());
}

/// A red confirmed by the whole run alone runs it in the workspace, and the
/// one-test runs there too.
#[tokio::test]
async fn a_red_is_confirmed_in_the_directory_of_its_check() {
    let home = checkout();
    let root = Manifest::parse(
        Path::new("armada.yml"),
        &text("01WSROOT", "root-ran", "", ""),
    )
    .expect("the root parses");
    let a = Manifest::parse_workspace(
        Path::new("a/armada.yml"),
        "a",
        "version: 1\nid: 01WSA\nchecks:\n  suite:\n    run: \"sh check.sh\"\n    one_test:\n      \
         run: \"sh one.sh {}\"\n",
        &root,
    )
    .expect("a parses");
    let dir = home.path().join("a");
    std::fs::write(
        dir.join("check.sh"),
        "echo whole >> runs\nif [ -f .ran ]; then exit 0; fi\ntouch .ran\n\
         echo 'Summary [   0.010s] 9 tests run: 0 passed, 1 failed, 0 skipped'\n\
         echo 'FAIL [   0.010s] (1/9) nt parses_it'\nexit 1\n",
    )
    .expect("the check");
    std::fs::write(dir.join("one.sh"), "echo \"$1\" >> alone\nexit 0\n").expect("one test");
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let workflow = ResolvedWorkflow::resolve_gated(&def, &root, &[&a]).expect("resolves");
    let changed = ["a/x.ts"];
    let paths: Vec<String> = changed.iter().map(|path| path.to_string()).collect();
    let gated = Gated::of(&root, std::slice::from_ref(&a), &paths);

    let ruling = ruled(&workflow, &home, &changed, Some(&gated)).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(
        std::fs::read_to_string(dir.join("alone")).expect("one test ran in a"),
        "parses_it\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("runs"))
            .expect("the whole run")
            .lines()
            .count(),
        2,
        "the red, then the whole run alone"
    );
}

/// What the gate came to replaces the placeholder dispatch wrote, and a load
/// reads it back.
#[tokio::test]
async fn the_gate_replaces_the_jobs_gating_manifests_with_what_each_came_to() {
    let (repository, checkout) = (repository(""), checkout());
    let home = TempDir::new();
    let fleet = crate::daemon::Fleet::assembled(crate::tests::daemon::fittings(
        &home,
        FakeWorkProduct::changed(&[]),
    ));
    let job = fleet
        .propose(crate::tests::daemon::a_proposal("change lib"))
        .await
        .expect("a proposed Job");
    assert!(job.gate_manifests().is_empty());

    let workflow = repository.workflow();
    let changed = ["lib/src/x.ts"];
    let gated = repository.gated(&changed);
    let ruling = ruled(&workflow, &checkout, &changed, Some(&gated)).await;
    fleet
        .kept_gates(job.id(), &workflow.steps()[0], Some(&gated), &ruling)
        .await;

    let read = fleet.load(job.id()).await.expect("the Job loads");
    let kept: Vec<(&str, GateOutcome)> = read
        .gate_manifests()
        .iter()
        .map(|gate| (gate.manifest_id.as_str(), gate.outcome))
        .collect();
    assert_eq!(
        kept,
        [
            ("01WSLIB", GateOutcome::RanAndPassed),
            ("01WSA", GateOutcome::RanAndPassed),
            ("01WSB", GateOutcome::RanAndPassed),
        ]
    );

    // No gate, no change: a repository with no workspaces writes nothing.
    fleet
        .kept_gates(job.id(), &workflow.steps()[0], None, &ruling)
        .await;
    assert_eq!(
        fleet.load(job.id()).await.expect("loads").gate_manifests(),
        read.gate_manifests()
    );
}

/// The Job froze its workflow before `a` had a manifest. `a` gates the change
/// at the gate, its Checks are taken from the manifests Fleet serves, and they
/// run. **Without the widening the same change runs nothing.**
#[tokio::test]
async fn a_manifest_the_job_froze_no_checks_for_gets_its_checks_run_at_the_gate() {
    let (served, home) = (repository(""), checkout());
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let without_a: Vec<&Manifest> = vec![&served.root, &served.below[0], &served.below[2]];
    let frozen = ResolvedWorkflow::resolve_gated(&def, &served.root, &without_a)
        .expect("resolves")
        .frozen()
        .clone();
    let changed = ["a/src/x.ts"];
    let gated = served.gated(&changed);

    let before = ruled_over(&frozen, &home, &changed, Some(&gated)).await;
    assert!(
        left(&home).is_empty(),
        "frozen, nothing of a's runs: {before:?}"
    );

    let unmet = gated.unmet_by(&frozen, &served.root, &served.below);
    assert_eq!(
        unmet
            .iter()
            .map(|manifest| manifest.dir())
            .collect::<Vec<_>>(),
        ["a"]
    );
    let widened = config::with_manifests_added(&frozen, &unmet);
    let ruling = ruled_over(&widened, &home, &changed, Some(&gated)).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["a/a-ran"], "a's Check ran in a's directory");
    assert!(outcomes(&ruling).contains(&("a:test".to_string(), CheckOutcome::Passed)));
    assert_eq!(
        frozen.steps()[0].checks().len(),
        3,
        "the Job's own is unchanged"
    );
}

/// A manifest the Job did freeze Checks for is not read again from main: a
/// Check added to `a` there since waits for a new Job.
#[tokio::test]
async fn a_manifest_the_job_froze_checks_for_is_not_read_again_from_main() {
    let frozen = repository("").workflow().frozen().clone();
    let on_main = repository("  extra:\n    run: \"/usr/bin/touch a-extra\"\n");
    let home = checkout();
    let changed = ["a/src/x.ts"];
    let gated = on_main.gated(&changed);

    assert!(gated
        .unmet_by(&frozen, &on_main.root, &on_main.below)
        .is_empty());
    let ruling = ruled_over(&frozen, &home, &changed, Some(&gated)).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(left(&home), ["a/a-ran"], "only what the Job froze ran");
    assert!(!home.path().join("a/a-extra").exists());
}
