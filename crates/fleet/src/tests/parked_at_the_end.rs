//! A killed or failed Job whose slot the pool refuses for uncommitted files
//! has them committed to its branch and the slot freed the moment it ends,
//! instead of sitting `kept`. `docs/concepts/fleet.md`, *Worktree slots*.

use adapter_traits::SlotParkRefused;
use core_model::{Actor, EscalationTrigger, JobId, Target};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::leasing::{approved, finished, root, Fixture};
use crate::tests::tmp::TempDir;

const DIRTY: &str = "slot-1 has 2 uncommitted files";

async fn ended_as(fleet: &Fixture, job: &JobId, target: Target, by: Actor) {
    let loaded = fleet.load(job).await.expect("the Job");
    fleet.move_job(&loaded, target, by).await.expect("moved");
}

async fn log_of(fleet: &Fixture, home: &TempDir, job: &JobId) -> String {
    let handle = fleet.load(job).await.expect("the Job").handle();
    std::fs::read_to_string(crate::transcript::log_of(&root(home), &handle)).unwrap_or_default()
}

async fn a_dirty_job(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, home, "one with files nobody committed").await;
    fleet.vcs().keep_next_release(DIRTY);
    (fleet, job)
}

fn assert_saved_and_free(fleet: &Fixture, home: &TempDir, job: &JobId, log: &str) {
    assert_eq!(
        fleet.vcs().parked_slots(),
        vec![(1, job.as_str().to_string())]
    );
    assert_eq!(fleet.vcs().slot_holders(&root(home))[0], None);
    assert!(
        log.contains("the Job ended and its uncommitted files were committed to its branch")
            && log.contains(&format!("wip-{}", job.as_str()))
            && log.contains("wip.txt"),
        "the log names the commit and its files: {log}"
    );
}

#[tokio::test]
async fn a_killed_job_with_uncommitted_files_commits_them_and_frees_its_slot() {
    let home = TempDir::new();
    let (fleet, job) = a_dirty_job(&home).await;

    ended_as(&fleet, &job, Target::Killed, Actor::Human).await;

    let log = log_of(&fleet, &home, &job).await;
    assert_saved_and_free(&fleet, &home, &job, &log);
}

#[tokio::test]
async fn a_failed_job_with_uncommitted_files_commits_them_and_frees_its_slot() {
    let home = TempDir::new();
    let (fleet, job) = a_dirty_job(&home).await;
    ended_as(
        &fleet,
        &job,
        Target::Escalated(EscalationTrigger::Interrupted),
        Actor::Fleet,
    )
    .await;

    ended_as(&fleet, &job, Target::CompletedFailed, Actor::Human).await;

    let log = log_of(&fleet, &home, &job).await;
    assert_saved_and_free(&fleet, &home, &job, &log);
}

/// A clean slot goes by the pool's own release, with nothing committed.
#[tokio::test]
async fn a_killed_job_with_a_clean_slot_commits_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, &home, "nothing to save").await;

    ended_as(&fleet, &job, Target::Killed, Actor::Human).await;

    assert!(fleet.vcs().parked_slots().is_empty());
    assert_eq!(
        fleet.vcs().released_slots(),
        vec![(1, job.as_str().to_string())]
    );
}

#[tokio::test]
async fn a_job_that_completed_keeps_its_slot_and_its_uncommitted_files() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, &home, "fix the off-by-one").await;
    fleet.vcs().keep_next_release(DIRTY);

    let job = finished(&fleet, job).await;

    assert!(fleet.vcs().parked_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
}

#[tokio::test]
async fn a_park_git_refuses_leaves_the_killed_job_s_slot_kept_with_the_reason() {
    let home = TempDir::new();
    let (fleet, job) = a_dirty_job(&home).await;
    fleet.vcs().refuse_next_park(SlotParkRefused::OnNoBranch);

    ended_as(&fleet, &job, Target::Killed, Actor::Human).await;

    assert!(fleet.vcs().parked_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
    let log = log_of(&fleet, &home, &job).await;
    assert!(
        log.contains("its uncommitted files could not be committed")
            && log.contains("HEAD is detached"),
        "{log}"
    );
}

/// A Job that has not ended is never parked, whatever its slot holds.
#[tokio::test]
async fn a_job_that_has_not_ended_is_not_parked() {
    let home = TempDir::new();
    let (fleet, job) = a_dirty_job(&home).await;

    assert!(fleet.vcs().parked_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
}
