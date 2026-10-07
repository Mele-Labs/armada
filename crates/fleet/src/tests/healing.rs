//! A Job held up by its worktree rather than its work heals without a person.
//! `crate::healing`. Both cases are Job 13's, on 7 Oct 2026.
//!
//! **The repair Drone is a shell.** One harness serves every Drone in a case:
//! told to repair, it does what the repair would do to the fake (removes a
//! flag file, touches a file) and exits; told anything else, it is a step's
//! Drone and waits.

use std::path::{Path, PathBuf};

use adapter_traits::{Grant, Repair};
use config::{Manifest, ResolvedWorkflow};
use core_model::{EscalationTrigger, Job, JobStatus, TransitionReason};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};

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
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/lead.ts"]),
        harness,
        FakeVcs::new(),
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
