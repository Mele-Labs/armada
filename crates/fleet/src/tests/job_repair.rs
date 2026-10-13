//! A Commander's repair Drone on the Job's own branch. `#2170`.
//!
//! **The Drone is a shell**, as `tests::healing`'s is: told `REPAIR THE JOB` it
//! does what the repair would do to the fake and prints the line the fake reads
//! as a turn ending; told anything else it is a step's Drone and waits. The
//! Commander's host is `tests::commander_conversation`'s scripted one.
//!
//! What a pure value can say is `acceptance`'s (`tests/commander.rs`): where a
//! repair is aimed and what it is granted. What needs a Fleet is here.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, Grant, Repair};
use api::Conversations;
use core_model::{Actor, JobId, JobStatus, StepId};
use ipc::Breakage;
use store::Moved;
use testkit::{FakeHarness, FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::helm::Hosting;
use crate::session::LiveSession;
use crate::tests::admitted::started;
use crate::tests::commander_conversation::{resolved, said, settled, woke_rows, Hosted, Scripted};
use crate::tests::daemon::{diff_evidence, fitted_with, one};
use crate::tests::planted::the_drone_it_holds_is_gone;
use crate::tests::restarting::stopped;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

const IMPLEMENT: &str = "implement";

/// A repair Drone runs `repair` and ends its turn when `ends`; a step's Drone
/// waits. A repair that does not print the line is a Drone that died.
fn harness(repair: &str, ends: bool) -> FakeHarness {
    let end = if ends { "; echo turn-ended" } else { "" };
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            &format!(
                "IFS= read -r line; case \"$line\" in *\"REPAIR THE JOB\"*) {repair}{end};; *) echo BUSY; sleep 30;; esac"
            ),
        ],
    )
    .reading(
        "turn-ended",
        vec![DroneEvent::Ended {
            turns: 1,
            cost_micros: 2_500,
            refusals: 0,
        }],
    )
}

/// One step gated on `diff_nonempty` and nothing else, so the only thing that
/// can fail it is the baseline.
fn gated() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn a_fleet(home: &TempDir, host: &Arc<Scripted>, harness: FakeHarness) -> Arc<Hosted> {
    let mut fitted = fitted_with(home, FakeWorkProduct::inherited(&[]), harness);
    fitted.starting().workflows = one(gated());
    fitted.judge = Arc::new(FakeJudge::that_fails("no model is asked about a repair"));
    Arc::new(Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>))
}

/// Until the ended Drone's slot is empty, ending it again where the first kill
/// missed: a signal sent before the process group exists is lost.
async fn until_reaped(fleet: &Hosted) {
    for turn in 0..4000 {
        fleet.turn().await.expect("a turn");
        let slot = fleet.the_only_slot().await;
        let held = slot.lock().await;
        let Some(working) = held.as_ref() else {
            return;
        };
        if turn % 40 == 39 {
            let _ = working.session().terminate().await;
        }
        drop(held);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

/// A Job whose step stopped and whose Drone is gone: where a repair is offered.
async fn escalated(fleet: &Arc<Hosted>, home: &TempDir) -> JobId {
    let job = stopped(fleet, home).await;
    the_drone_it_holds_is_gone(fleet).await;
    until_reaped(fleet).await;
    job
}

async fn worktree_of(fleet: &Hosted, job: &JobId) -> PathBuf {
    let record = fleet.load(job).await.expect("the Job reads");
    let worktree = fleet
        .worktree_of(&record)
        .expect("reads the disk")
        .expect("the Job has a worktree");
    PathBuf::from(worktree.path())
}

async fn send(
    fleet: &Arc<Hosted>,
    job: &JobId,
    broke: Breakage,
) -> Result<ipc::RepairSent, api::Refusal> {
    Arc::clone(fleet)
        .send_repair_drone(resolved(fleet, job).await, ipc::SendRepairDrone { broke })
        .await
}

async fn until<F, Fut>(what: &str, mut ready: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _ in 0..600 {
        if ready().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never: {what}");
}

/// The repair Drone has handed in and the stopped step has been put back.
async fn until_restarted(fleet: &Arc<Hosted>, job: &JobId) {
    until("the step was restarted", || async {
        fleet.load(job).await.expect("reads").status() != JobStatus::Escalated
    })
    .await;
}

/// The Drones that were told `REPAIR THE JOB`, as they were configured.
fn repairs(fleet: &Hosted) -> Vec<adapter_traits::DroneSpawnConfig> {
    fleet
        .harness()
        .configured()
        .into_iter()
        .filter(|config| config.prompt().as_str().starts_with("REPAIR THE JOB"))
        .collect()
}

async fn the_card_out(fleet: &Arc<Hosted>, job: &JobId) -> Option<ipc::CommanderCard> {
    use api::Queries;
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .commander_card
}

async fn log_of(home: &TempDir, fleet: &Hosted, job: &JobId) -> String {
    let record = fleet.load(job).await.expect("the Job");
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &record.handle(),
    ))
    .expect("the Job's log")
}

/// **The Job's own worktree on the Job's own branch, and nothing cut beside
/// them.** A Trigger's repair leases a slot and a branch of its own; this does
/// neither, and what the Drone wrote is committed where the Job's work is.
#[tokio::test]
async fn a_repair_works_in_the_jobs_own_worktree_on_its_own_branch_and_cuts_nothing_else() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", true));
    let job = escalated(&fleet, &home).await;
    let own = worktree_of(&fleet, &job).await;
    let branch = fleet.load(&job).await.unwrap().branch().expect("a branch").as_str().to_string();
    let (made, cut) = (fleet.vcs().created().len(), fleet.vcs().cut_from().len());
    let delivered = fleet.vcs().delivered().len();

    let sent = send(&fleet, &job, Breakage::Dependencies).await.expect("sent");
    until_restarted(&fleet, &job).await;

    assert_eq!(sent.step_id.as_str(), IMPLEMENT);
    let ran = repairs(&fleet);
    assert_eq!(ran.len(), 1, "one repair Drone");
    assert_eq!(ran[0].worktree().path(), own.to_string_lossy());
    assert_eq!(ran[0].worktree().branch(), branch);
    assert_eq!(
        (fleet.vcs().created().len(), fleet.vcs().cut_from().len()),
        (made, cut),
        "no slot was leased and no branch cut"
    );
    let commits = fleet.vcs().committed();
    let last = commits.last().expect("what it wrote was committed");
    assert_eq!(
        (last.branch.as_str(), last.message.as_str()),
        (branch.as_str(), "Repair the worktree: dependencies")
    );
    assert_eq!(fleet.vcs().delivered().len(), delivered, "and nothing was pushed");
    assert!(
        fleet.store().lock().await.trigger_firings(&job).unwrap().is_empty(),
        "and it is no Trigger's repair, held for the owner to place"
    );
}

/// **A conflict in the worktree is repaired and the owner is not asked.** The
/// move that puts the step back is the Commander's.
#[tokio::test]
async fn a_conflict_in_the_worktree_gets_a_repair_and_the_owner_is_never_asked() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("rm -f .unmerged", true));
    let job = escalated(&fleet, &home).await;
    let worktree = worktree_of(&fleet, &job).await;
    std::fs::write(worktree.join("lead.ts"), "export const lead = 1;\n").unwrap();
    fleet
        .work()
        .unmerged_until_gone(&worktree.join(".unmerged"), &["lead.ts"]);

    send(&fleet, &job, Breakage::MergeConflict).await.expect("sent");
    until_restarted(&fleet, &job).await;

    assert!(!worktree.join(".unmerged").exists(), "the index is clean");
    let granted: Vec<Repair> = repairs(&fleet)[0]
        .toolbelt()
        .granted()
        .iter()
        .filter_map(|grant| match grant {
            Grant::RepairTheWorktree(repair) => Some(repair.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(granted, vec![Repair::TheIndex]);
    assert!(repairs(&fleet)[0].prompt().as_str().contains("conflicts"));
    assert!(the_card_out(&fleet, &job).await.is_none(), "no card went to him");
    use api::Queries;
    assert!(!fleet.get_job(ipc::JobId::from(&job)).await.unwrap().job.asking);
    let mover = fleet
        .store()
        .lock()
        .await
        .events_for(&job)
        .unwrap()
        .iter()
        .rev()
        .find_map(|event| match event.moved() {
            Moved::Job { .. } => Some(event.actor()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        })
        .expect("the Job moved");
    assert_eq!(mover, Actor::Commander, "the step was put back by the Commander");
    let log = log_of(&home, &fleet, &job).await;
    assert!(log.contains("sent a repair Drone onto the Job's branch"), "{log}");
    assert!(log.contains("merge conflict"), "{log}");
}

/// A repair that leaves a marker in a file the index held unmerged has not
/// resolved it, whatever it staged.
#[tokio::test]
async fn a_repair_that_leaves_a_conflict_marker_hands_nothing_in() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("rm -f .unmerged", true));
    let job = escalated(&fleet, &home).await;
    let worktree = worktree_of(&fleet, &job).await;
    std::fs::write(worktree.join("lead.ts"), "<<<<<<< ours\nlead\n").unwrap();
    fleet
        .work()
        .unmerged_until_gone(&worktree.join(".unmerged"), &["lead.ts"]);

    send(&fleet, &job, Breakage::MergeConflict).await.expect("sent");
    until("the Commander was woken", || async { !host.carried().is_empty() }).await;
    settled(&fleet, &job).await;

    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Escalated);
    assert_eq!(
        woke_rows(&said(&fleet, &job).await),
        ["The repair Drone on step implement ended with nothing handed in"]
    );
}

/// **Its work is the step's, and the step's gate rules on it.** The step was
/// entered before the breakage, so its baseline excludes the repair's file and
/// `diff_nonempty` passes on it; with no file written the same step fails.
#[tokio::test]
async fn the_repairs_work_counts_as_the_steps_and_the_jobs_own_check_rules_on_it() {
    for repaired in [true, false] {
        let home = TempDir::new();
        let host = Scripted::answering();
        let fleet = a_fleet(&home, &host, harness("true", true));
        let job = escalated(&fleet, &home).await;
        if repaired {
            fleet
                .work()
                .wrote(&[("src/parse.rs", adapter_traits::Change::Modified)]);
        }

        send(&fleet, &job, Breakage::MissingBinary).await.expect("sent");
        until_restarted(&fleet, &job).await;
        started(&fleet, &job).await.expect("the step's Drone is back on");
        submitted_by_the_one(&fleet, diff_evidence()).await.expect("handed in");
        let turned = fleet.turn().await.expect("the gate ruled");

        let advanced = turned.ruled().is_some_and(|ruling| ruling.advanced());
        assert_eq!(
            advanced, repaired,
            "the gate ruled on the step with its repair written: {repaired}"
        );
    }
}

/// **Neither a Check nor the Manifest is the Drone's to edit**, and it holds no
/// grant that dispatches, plans or pushes.
#[tokio::test]
async fn a_repair_drone_is_held_off_the_manifest_and_granted_no_dispatch_or_plan() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", true));
    let job = escalated(&fleet, &home).await;

    send(&fleet, &job, Breakage::Worktree).await.expect("sent");
    until_restarted(&fleet, &job).await;

    let belt = repairs(&fleet)[0].toolbelt().clone();
    assert!(belt.held_off().contains(&"armada.yml".to_string()));
    assert!(belt.held_off().contains(&".armada".to_string()));
    assert!(belt.granted().contains(&Grant::ChangeTheWorktree));
    for grant in belt.granted() {
        assert!(
            !matches!(
                grant,
                Grant::DispatchAJob | Grant::RecordThePlan | Grant::WorkThePlan
            ),
            "{grant:?}"
        );
    }
}

/// **Offered where `restart_step` is, and refused at the moment it is asked
/// anywhere else.**
#[tokio::test]
async fn a_repair_is_refused_where_the_step_could_not_be_put_back_after_it() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", true));
    let job = stopped(&fleet, &home).await;

    // The Drone is still standing in the slot.
    let alive = send(&fleet, &job, Breakage::Dependencies).await.expect_err("a Drone is on it");
    assert!(alive.error().message.contains("alive"), "{}", alive.error().message);

    the_drone_it_holds_is_gone(&fleet).await;
    until_reaped(&fleet).await;
    let worktree = worktree_of(&fleet, &job).await;
    std::fs::remove_dir_all(&worktree).unwrap();
    let gone = send(&fleet, &job, Breakage::Worktree).await.expect_err("no worktree");
    assert!(gone.error().message.contains("redispatch"), "{}", gone.error().message);
    assert!(repairs(&fleet).is_empty(), "and no Drone was spent on either");
}

/// A Job still working is not a Job held for a person.
#[tokio::test]
async fn a_repair_is_refused_on_a_job_that_is_working() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", true));
    let job = crate::tests::commander_conversation::working(&fleet, &home, "bound the reader").await;

    let refused = send(&fleet, &job, Breakage::Dependencies).await.expect_err("working");

    assert!(refused.error().message.contains("is running"), "{}", refused.error().message);
    assert!(repairs(&fleet).is_empty());
}

/// **One repair at a time per Job**, and a person's restart does not put a
/// step Drone in the worktree beside it.
#[tokio::test]
async fn while_a_repair_is_out_a_second_and_a_restart_are_refused() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let go = home.path().join("go");
    let waiting = format!("while [ ! -f {} ]; do sleep 0.02; done", go.display());
    let fleet = a_fleet(&home, &host, harness(&waiting, true));
    let job = escalated(&fleet, &home).await;
    send(&fleet, &job, Breakage::Dependencies).await.expect("sent");

    let second = send(&fleet, &job, Breakage::Dependencies).await.expect_err("one is out");
    let restart = fleet.restart_step(&job, None).await.expect_err("a repair is out");
    std::fs::write(&go, "").unwrap();
    until_restarted(&fleet, &job).await;

    assert!(second.error().message.contains("repair Drone is on"), "{}", second.error().message);
    assert!(
        restart.to_string().contains("repair Drone is on"),
        "{restart}"
    );
    assert_eq!(repairs(&fleet).len(), 1);
}

/// **The loop bound is the repair's only count.** Three repairs that hand
/// nothing in each wake the Commander; the fourth is not woken for and a card
/// goes to the owner instead. None of it spends `healing`'s repairs per kind.
#[tokio::test]
async fn a_repair_that_hands_nothing_in_wakes_the_commander_and_the_bound_stops_it_at_the_fourth() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", false));
    let job = escalated(&fleet, &home).await;
    let row = "The repair Drone on step implement ended with nothing handed in";

    for woken in 1..=3 {
        send(&fleet, &job, Breakage::MissingBinary).await.expect("sent");
        until("the Commander was woken", || async { host.carried().len() == woken }).await;
        settled(&fleet, &job).await;
        assert!(the_card_out(&fleet, &job).await.is_none(), "wake {woken}");
    }
    assert_eq!(woke_rows(&said(&fleet, &job).await), [row, row, row]);

    send(&fleet, &job, Breakage::MissingBinary).await.expect("sent");
    until("the card went out", || async { the_card_out(&fleet, &job).await.is_some() }).await;
    let card = the_card_out(&fleet, &job).await.unwrap();
    assert_eq!(card.by, ipc::CardBy::Fleet, "Fleet put it out, at the bound");
    assert_eq!(card.text, row);
    assert_eq!(host.carried().len(), 3, "and no fourth wake");
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Escalated);

    assert_eq!(repairs(&fleet).len(), 4, "every repair ran");
    let log = log_of(&home, &fleet, &job).await;
    assert!(
        !log.contains("repairs of this kind already ran"),
        "healing's count was never touched: {log}"
    );
    assert!(log.contains("woken three times for one problem"), "{log}");
}

/// The step is the one the breakage stopped, named on the wire.
#[tokio::test]
async fn the_answer_names_the_step_the_breakage_stopped_and_the_drone_sent() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = a_fleet(&home, &host, harness("true", true));
    let job = escalated(&fleet, &home).await;

    let sent = send(&fleet, &job, Breakage::MergeConflict).await.expect("sent");

    assert_eq!(sent.job_id.as_str(), job.as_str());
    assert_eq!(sent.step_id.to_domain(), StepId::new(IMPLEMENT));
    assert!(!sent.drone_id.as_str().is_empty());
    assert_eq!(sent.broke, Breakage::MergeConflict);
    until_restarted(&fleet, &job).await;
}
