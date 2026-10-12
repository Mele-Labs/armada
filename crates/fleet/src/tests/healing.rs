//! A Job held up by its worktree rather than its work heals without a person.
//! `crate::healing`. Both cases are Job 13's, on 7 Oct 2026.
//!
//! **The repair Drone is a shell.** One harness serves every Drone in a case:
//! told to repair, it does what the repair would do to the fake (removes a
//! flag file, touches a file) and exits; told anything else, it is a step's
//! Drone and waits.

use std::path::{Path, PathBuf};

use adapter_traits::{BroughtUpToDate, Grant, Repair, Standing};
use config::{Manifest, ResolvedWorkflow};
use core_model::{EscalationTrigger, Job, JobStatus, TransitionReason};
use testkit::{Delivered, FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_over, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// What a Drone told to repair does, as a shell line. Anything else waits.
fn a_harness_repairing_by(repair: &str) -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            &format!(
                "IFS= read -r line; case \"$line\" in *\"REPAIR THE\"*) {repair};; *) sleep 30;; esac"
            ),
        ],
    )
}

fn gated_on(check: Gate<'static>) -> ResolvedWorkflow {
    testkit::retried(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[check],
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        0,
    )
}

fn a_fleet(home: &TempDir, harness: FakeHarness, workflow: ResolvedWorkflow) -> Fixture {
    a_fleet_committing_through(home, harness, workflow, FakeVcs::new())
}

fn a_fleet_committing_through(
    home: &TempDir,
    harness: FakeHarness,
    workflow: ResolvedWorkflow,
    vcs: FakeVcs,
) -> Fixture {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/lead.ts"]),
        harness,
        vcs,
    );
    fittings.starting().workflows = one(workflow);
    fittings.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  bootstrap:\n    run: /usr/bin/true\n\
         setup:\n  requires: [bootstrap]\n",
    )
    .expect("a manifest that bootstraps");
    Fleet::assembled(fittings)
}

async fn proposed(home: &TempDir, fleet: &Fixture) -> (Job, PathBuf) {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the lead"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, &job);
    let at = crate::tests::daemon::spec_held(home, &job)
        .expect("a legal spec")
        .worktree_path();
    (job, PathBuf::from(at))
}

/// The grants every repair Drone this harness rendered was started with.
fn repairs_granted(fleet: &Fixture) -> Vec<Repair> {
    fleet
        .harness()
        .configured()
        .iter()
        .flat_map(|config| config.toolbelt().granted().to_vec())
        .filter_map(|grant| match grant {
            Grant::RepairTheWorktree(repair) => Some(repair),
            _ => None,
        })
        .collect()
}

/// A Drone resolved `lead.ts` and `git add` is not a command it may run, so the
/// index still holds it unmerged and the diff shows nothing.
fn left_unmerged(work: &FakeWorkProduct, worktree: &Path) -> PathBuf {
    let flag = worktree.join(".unmerged");
    std::fs::write(worktree.join("lead.ts"), "export const lead = 1;\n").expect("resolved file");
    work.unmerged_until_gone(&flag, &["lead.ts"]);
    flag
}

/// [`left_unmerged`] with the flag at a place the caller chose, because a Fleet
/// built before the worktree existed already holds that path.
fn left_unmerged_by(work: &FakeWorkProduct, worktree: &Path, flag: &Path) {
    std::fs::write(worktree.join("lead.ts"), "export const lead = 1;\n").expect("resolved file");
    work.unmerged_until_gone(flag, &["lead.ts"]);
}

/// **Case 1, at the gate.** The Judge refused Job 13's step on "a mode change
/// only" until a person staged the file. A repair Drone stages it before the
/// gate reads the diff and the step advances.
#[tokio::test]
async fn a_resolution_nothing_staged_is_staged_before_the_gate_reads_the_diff() {
    let home = TempDir::new();
    // The flag lives in the worktree, which is known only after the Job
    // exists, so the repair removes it by its fixed name there.
    let fleet = a_fleet(
        &home,
        a_harness_repairing_by("rm -f .unmerged"),
        gated_on(Gate::DiffNonempty),
    );
    let (job, worktree) = proposed(&home, &fleet).await;
    dispatched(&fleet, job.id()).await.expect("dispatched");
    let flag = left_unmerged(fleet.work(), &worktree);

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("submitted");
    let turned = fleet.turn().await.expect("the gate runs");

    assert!(
        turned.ruled().is_some_and(|ruling| ruling.advanced()),
        "the step advances without a person: {:?}",
        turned.ruled()
    );
    assert!(
        !flag.exists(),
        "the index no longer holds the path unmerged"
    );
    assert_eq!(repairs_granted(&fleet), vec![Repair::TheIndex]);
    assert_ne!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// **Case 1, before a Drone.** A reused slot with a leftover unmerged path is
/// repaired before the step's Drone is put on it.
#[tokio::test]
async fn an_index_left_unmerged_is_staged_before_a_drone_is_put_on_the_worktree() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        a_harness_repairing_by("rm -f .unmerged"),
        gated_on(Gate::DiffNonempty),
    );
    let (job, worktree) = proposed(&home, &fleet).await;
    let flag = left_unmerged(fleet.work(), &worktree);

    dispatched(&fleet, job.id()).await.expect("dispatched");

    let job = fleet.load(job.id()).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert!(!flag.exists());
    assert_eq!(repairs_granted(&fleet), vec![Repair::TheIndex]);
}

/// **Only a failed repair reaches a person**, and it says what was tried.
#[tokio::test]
async fn a_repair_that_leaves_the_index_unmerged_escalates_saying_what_was_tried() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        a_harness_repairing_by("true"),
        gated_on(Gate::DiffNonempty),
    );
    let (job, worktree) = proposed(&home, &fleet).await;
    left_unmerged(fleet.work(), &worktree);

    let refused = dispatched(&fleet, job.id()).await;

    let said = format!("{:?}", refused.expect_err("nothing could be put on it"));
    assert!(said.contains("lead.ts"), "it names the path: {said}");
    assert!(
        said.contains("repair Drone"),
        "it names what was tried: {said}"
    );
    let job = fleet.load(job.id()).await.unwrap();
    assert_eq!(job.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::NoWorktree))
    );
}

/// **Case 2.** `desktop_test` failed with "Electron failed to install
/// correctly" on a reused slot. A repair Drone reruns the bootstrap and Fleet
/// runs the Check again: the step advances and no Drone is asked to fix it.
#[tokio::test]
async fn a_check_naming_a_broken_install_is_run_again_after_a_repair() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        a_harness_repairing_by("touch .installed"),
        gated_on(Gate::Check {
            name: "desktop_test",
            run: "sh check.sh",
            expect_exit_code: 0,
            when: &[],
        }),
    );
    let (job, worktree) = proposed(&home, &fleet).await;
    std::fs::write(
        worktree.join("check.sh"),
        "[ -f .installed ] || { echo 'Error: Electron failed to install correctly'; exit 1; }\n",
    )
    .expect("the check written");
    dispatched(&fleet, job.id()).await.expect("dispatched");

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("submitted");
    let turned = fleet.turn().await.expect("the gate runs");

    assert!(
        turned.ruled().is_some_and(|ruling| ruling.advanced()),
        "the Check passes on the second run: {:?}",
        turned.ruled()
    );
    assert!(worktree.join(".installed").exists());
    assert_eq!(
        repairs_granted(&fleet),
        vec![Repair::TheInstall(String::from("/usr/bin/true"))],
        "granted the repository's own bootstrap and nothing else"
    );
}

/// Whether the branch was pushed and a pull request opened for it.
fn gone_out(fleet: &Fixture) -> bool {
    let did = fleet.vcs().delivered();
    did.iter().any(|it| matches!(it, Delivered::Pushed { .. }))
        && did
            .iter()
            .any(|it| matches!(it, Delivered::OpenedForReview { .. }))
}

/// A Fleet whose catch-up conflicts in `lead.ts` on the step's entry and whose
/// commit is refused while the index holds it unmerged: Job 14, 12 Oct 2026.
///
/// The repair is whatever `repair` does to the flag file that stands for the
/// index. The path conflicted by the catch-up is the Drone's to resolve, so
/// the repair on entry leaves it alone and the delivery on entry cannot commit.
fn a_delivery_that_cannot_commit(home: &TempDir, repair: &str) -> (Fixture, PathBuf) {
    let flag = home.path().join(".unmerged");
    let fleet = a_fleet_committing_through(
        home,
        a_harness_repairing_by(&repair.replace("FLAG", &flag.to_string_lossy())),
        gated_on(Gate::DiffNonempty),
        FakeVcs::new().refusing_to_commit_while(&flag),
    );
    fleet.vcs().now_behind(
        Standing::Behind { commits: 1 },
        Some(BroughtUpToDate::Conflicted {
            base: String::from("main"),
            files: vec![String::from("lead.ts")],
        }),
    );
    (fleet, flag)
}

/// **Job 14.** The step delivers, its catch-up left `lead.ts` conflicted, and
/// the commit on entry was refused. The Drone resolved it, the gate found the
/// index unmerged, and a repair Drone staged it. Fleet goes back to where it
/// stopped: the commit is made and the branch goes out.
#[tokio::test]
async fn a_delivery_the_index_stopped_goes_out_once_a_repair_has_staged_the_resolution() {
    let home = TempDir::new();
    let (fleet, flag) = a_delivery_that_cannot_commit(&home, "rm -f 'FLAG'");
    let (job, worktree) = proposed(&home, &fleet).await;
    left_unmerged_by(fleet.work(), &worktree, &flag);

    dispatched(&fleet, job.id()).await.expect("dispatched");
    assert!(
        fleet.vcs().committed().is_empty(),
        "the commit on entry was refused over the unmerged path"
    );
    assert!(!gone_out(&fleet), "and nothing went out");
    // The Drone resolved the conflict, so the base is held now and the retry
    // is not caught up over it again.
    fleet.vcs().now_behind(Standing::UpToDate, None);

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("submitted");
    let turned = fleet.turn().await.expect("the gate runs");

    assert!(
        turned.ruled().is_some_and(|ruling| ruling.advanced()),
        "the gate passes: {:?}",
        turned.ruled()
    );
    assert!(!flag.exists(), "the index no longer holds the path unmerged");
    assert_eq!(repairs_granted(&fleet), vec![Repair::TheIndex]);
    assert_eq!(fleet.vcs().committed().len(), 1, "the work is committed");
    assert!(gone_out(&fleet), "pushed and opened for review");
}

/// **A repair that did not clear the index delivers nothing**, and the gate
/// escalates as it did before.
#[tokio::test]
async fn a_repair_that_leaves_the_index_unmerged_does_not_retry_the_delivery() {
    let home = TempDir::new();
    let (fleet, flag) = a_delivery_that_cannot_commit(&home, "true");
    let (job, worktree) = proposed(&home, &fleet).await;
    left_unmerged_by(fleet.work(), &worktree, &flag);

    dispatched(&fleet, job.id()).await.expect("dispatched");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("submitted");
    fleet.turn().await.expect("the gate runs");

    assert!(flag.exists(), "the repair did not stage it");
    assert!(fleet.vcs().committed().is_empty());
    assert!(!gone_out(&fleet));
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Escalated
    );
}
