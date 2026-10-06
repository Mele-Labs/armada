//! A Job's worktree is the pool slot it leased: leased at dispatch, held by
//! the Job's id, looked up rather than derived, and given back when the Job
//! ends. `docs/concepts/fleet.md`, *Worktree slots*.
//!
//! The pool's own git rules are `adapters`' tests. These are Fleet's half:
//! what it records, when it gives a slot back, and what it refuses to do when
//! a Job's slot is not its own any more.

use std::path::PathBuf;

use adapter_traits::{SlotStanding, Vcs, WorktreeSpec};
use adapters::leasing::{Holder, Pool, SlotState};
use adapters::GitVcs;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, QueuedReason, Target, TransitionReason,
};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::slots::Concurrency;
use crate::tests::admitted::admit;
use crate::tests::daemon::{
    a_fleet, a_proposal, diff_evidence, fitted_over, fittings, note_evidence, worktree_directory,
};
use crate::tests::reclaim::a_repository;
use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

fn slot(home: &TempDir, n: u32) -> PathBuf {
    home.path().join(format!(".armada/slots/slot-{n}"))
}

/// Propose and approve a Job, and admit what there is room for.
async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("proposed");
    worktree_directory(home, &job);
    fleet.approve(job.id()).await.expect("approved");
    admit(fleet).await.expect("admission runs");
    job.id().clone()
}

/// Four Drones at once, so four Jobs each hold a slot together.
fn four_at_once(home: &TempDir) -> Fixture {
    let mut fitted = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.concurrency = Concurrency::of(4);
    Fleet::assembled(fitted)
}

async fn status_of(fleet: &Fixture, job: &JobId) -> JobStatus {
    fleet.load(job).await.expect("the Job").status()
}

#[tokio::test]
async fn a_job_leases_a_slot_and_its_worktree_is_the_slot() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, &home, "fix the reader").await;

    let loaded = fleet.load(&job).await.expect("the Job");
    assert_eq!(loaded.status(), JobStatus::Running);
    assert_eq!(loaded.worktree_slot(), Some(1), "the record names its slot");
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home)),
        vec![
            Some(job.as_str().to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None
        ],
        "held by the Job's id"
    );
    let worktree = fleet
        .worktree_of(&loaded)
        .expect("readable")
        .expect("the Job has a worktree");
    assert_eq!(PathBuf::from(worktree.path()), slot(&home, 1));
    assert_eq!(
        worktree.branch(),
        format!("armada/{}", loaded.handle()),
        "the branch is still the Job's own"
    );
}

/// Real git, and a second Fleet over the same home: a Fleet restart. The
/// lease names the Job and no process, so the new Fleet's pid reads it as held.
#[tokio::test]
async fn a_fleet_restart_leaves_a_jobs_slot_held_by_that_job() {
    let home = TempDir::new();
    a_repository(&home);
    let first = Fleet::assembled(fitted_over(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::that_listens(),
        GitVcs::new(),
    ));
    let job = first
        .propose(a_proposal("fix the reader"))
        .await
        .expect("proposed");
    first.approve(job.id()).await.expect("approved");
    let _ = first.admit_next().await;
    assert_eq!(
        first.load(job.id()).await.expect("the Job").worktree_slot(),
        Some(1)
    );
    drop(first);

    let second = Fleet::assembled(fitted_over(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::that_listens(),
        GitVcs::new(),
    ));
    let _ = second.reconcile().await;

    let pool = Pool::at(home.path(), 8, "main", Vec::new());
    match pool.state(1) {
        SlotState::Held { holder, .. } => assert_eq!(holder, Holder::job(job.id().as_str())),
        other => panic!("the Job's slot after a restart: {other:?}"),
    }
    let loaded = second.load(job.id()).await.expect("the Job");
    assert_eq!(loaded.worktree_slot(), Some(1));
    assert!(
        !matches!(
            pool.try_lease("an-agent", &Holder::of(std::process::id()).unwrap(), 0, &|_, _| Err(
                String::new()
            )),
            Ok(adapters::leasing::Leased::Took(lease)) if lease.slot() == 1
        ),
        "the restart did not make the Job's slot look abandoned"
    );
}

#[tokio::test]
async fn terminal_states_release_the_slot_and_waiting_ones_do_not() {
    let home = TempDir::new();
    let fleet = four_at_once(&home);
    let reviewed = approved(&fleet, &home, "one held at a person's gate").await;
    let escalated = approved(&fleet, &home, "one escalated").await;
    let interrupted = approved(&fleet, &home, "one interrupted").await;
    let killed = approved(&fleet, &home, "one killed").await;
    let held = |job: &JobId| Some(job.as_str().to_string());

    let moved = |job: JobId, to: Target, by: Actor| {
        let fleet = &fleet;
        async move {
            let loaded = fleet.load(&job).await.expect("the Job");
            fleet.move_job(&loaded, to, by).await.expect("a legal move");
        }
    };
    moved(reviewed.clone(), Target::AwaitingReview, Actor::Fleet).await;
    moved(
        escalated.clone(),
        Target::Escalated(EscalationTrigger::NoWorktree),
        Actor::Fleet,
    )
    .await;
    moved(
        interrupted.clone(),
        Target::Escalated(EscalationTrigger::Interrupted),
        Actor::Fleet,
    )
    .await;
    let holders = fleet.vcs().slot_holders(&root(&home));
    assert_eq!(
        holders[..4],
        [
            held(&reviewed),
            held(&escalated),
            held(&interrupted),
            held(&killed)
        ],
        "awaiting_review, escalated and interrupted all hold their slots"
    );
    assert!(fleet.vcs().released_slots().is_empty());

    moved(killed.clone(), Target::Killed, Actor::Human).await;
    moved(reviewed.clone(), Target::Rejected, Actor::Human).await;
    moved(escalated.clone(), Target::CompletedFailed, Actor::Human).await;

    assert_eq!(
        fleet.vcs().released_slots(),
        vec![
            (4, killed.as_str().to_string()),
            (1, reviewed.as_str().to_string()),
            (2, escalated.as_str().to_string()),
        ],
        "killed, rejected and failed each gave its slot back"
    );
    let holders = fleet.vcs().slot_holders(&root(&home));
    assert_eq!(holders[..4], [None, None, held(&interrupted), None]);
}

/// Worked through the gate to `completed_success`, which is guarded on every
/// step having advanced, so it cannot be moved there by hand.
async fn finished(fleet: &Fixture, job: JobId) -> JobId {
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("a diff");
    fleet.turn().await.expect("the first step advances");
    submitted_by_the_one(fleet, note_evidence())
        .await
        .expect("a note");
    fleet.turn().await.expect("the Job finishes");
    assert_eq!(status_of(fleet, &job).await, JobStatus::CompletedSuccess);
    job
}

async fn completed(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = approved(fleet, home, title).await;
    finished(fleet, job).await
}

/// **A completed Job holds its slot until a person clears it**, the owner's
/// decision, so whatever reads its tree afterwards still finds it. The pool's
/// record says it completed, for `armada worktree --status`.
#[tokio::test]
async fn a_job_that_completes_holds_its_slot() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;

    assert!(fleet.vcs().released_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
    assert_eq!(
        fleet.vcs().completed_slots(),
        vec![(1, job.as_str().to_string())]
    );
    assert!(
        fleet
            .worktree_of(&fleet.load(&job).await.expect("the Job"))
            .expect("readable")
            .is_some(),
        "its tree is still its own"
    );
}

/// Clear, the Board's act on a finished Job, gives the slot back by the
/// pool's rules. The fixture's root is no repository, so the branch half after
/// it is refused; the slot went back first.
#[tokio::test]
async fn clearing_a_completed_job_gives_its_slot_back() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;

    let cleared = Fleet::reclaim_worktree(&fleet, &job).await;

    assert!(
        !matches!(cleared, Err(Adrift::SlotKept { .. })),
        "{cleared:?}"
    );
    assert_eq!(
        fleet.vcs().released_slots(),
        vec![(1, job.as_str().to_string())]
    );
}

/// Deleting a completed Job's record gives its slot back too: a lease held
/// for a Job nobody can name again would never be given back.
#[tokio::test]
async fn forgetting_a_completed_job_gives_its_slot_back() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;

    fleet
        .forget_job(&job)
        .await
        .expect("a terminal Job's record goes");

    assert_eq!(
        fleet.vcs().released_slots(),
        vec![(1, job.as_str().to_string())]
    );
}

/// The sweep is not a person clearing it: a completed Job's slot is left
/// held, however safe its tree.
#[tokio::test]
async fn the_sweep_leaves_a_completed_jobs_slot_held() {
    let home = TempDir::new();
    a_repository(&home);
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.reclaiming = crate::Reclaiming::every(std::time::Duration::ZERO);
    let fleet = Fleet::assembled(fitted);
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("proposed");
    crate::tests::reclaim::a_slot_for(&home, &job.handle());
    fleet.approve(job.id()).await.expect("approved");
    admit(&fleet).await.expect("admission runs");
    let job = finished(&fleet, job.id().clone()).await;

    let turned = fleet.turn().await.expect("a sweep");

    assert!(turned.reclaimed.is_empty(), "{:?}", turned.reclaimed);
    assert!(fleet.vcs().released_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
}

#[tokio::test]
async fn a_release_the_pool_refuses_leaves_the_slot_held_and_the_job_s_log_says_why() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, &home, "one with work nobody pushed").await;
    fleet
        .vcs()
        .keep_next_release("armada/1-x has 2 commits on neither the remote nor the base");

    let loaded = fleet.load(&job).await.expect("the Job");
    fleet
        .move_job(
            &loaded,
            Target::Escalated(EscalationTrigger::Interrupted),
            Actor::Fleet,
        )
        .await
        .expect("escalated");
    let loaded = fleet.load(&job).await.expect("the Job");
    fleet
        .move_job(&loaded, Target::CompletedFailed, Actor::Human)
        .await
        .expect("failed");

    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string()),
        "the slot stays held while it holds work"
    );
    let handle = fleet.load(&job).await.expect("the Job").handle();
    let said = std::fs::read_to_string(crate::transcript::log_of(&root(&home), &handle))
        .unwrap_or_default();
    assert!(
        said.contains("2 commits on neither the remote nor the base"),
        "the Job's own log says why: {said}"
    );
}

/// The worktree a Job's earlier steps worked in is now somebody else's. Fleet
/// stops the Job for a person; it never quietly leases a second slot.
#[tokio::test]
async fn a_job_whose_slot_another_holds_escalates_and_leases_no_other() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = at_the_gate(&fleet, &home).await;
    fleet
        .approve_review(&job)
        .await
        .expect("the slot is the Job's while the person reads");
    fleet.vcs().hold_slot(&root(&home), 1, "an agent's session");

    let refused = admit(&fleet)
        .await
        .expect_err("there is nothing of its own to put a Drone back onto");
    match &refused {
        Adrift::SlotLost { slot, why, .. } => {
            assert_eq!(*slot, 1);
            assert!(why.contains("an agent's session"), "{why}");
        }
        other => panic!("expected the lost slot, got {other:?}"),
    }
    assert_eq!(status_of(&fleet, &job).await, JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&job).await.expect("a reason"),
        Some(TransitionReason::Escalation(EscalationTrigger::NoWorktree))
    );
    assert_eq!(fleet.vcs().created().len(), 1, "no second slot was leased");
    assert!(
        !fleet
            .vcs()
            .slot_holders(&root(&home))
            .contains(&Some(job.as_str().to_string())),
        "and the Job holds none"
    );
}

#[tokio::test]
async fn with_every_slot_held_a_new_job_waits_on_resources_and_starts_when_one_frees() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    for n in 1..=8 {
        fleet.vcs().hold_slot(&root(&home), n, "an agent's session");
    }

    let job = approved(&fleet, &home, "fix the reader").await;
    assert_eq!(status_of(&fleet, &job).await, JobStatus::Queued);
    let loaded = fleet.load(&job).await.expect("the Job");
    assert_eq!(
        fleet.queued_reason(&loaded).await.expect("a reason").reason,
        Some(QueuedReason::WaitingOnResources),
        "the Board says what admission is waiting for"
    );

    fleet.vcs().free_slot(&root(&home), 6);
    let loaded = fleet.load(&job).await.expect("the Job");
    assert_eq!(
        fleet.queued_reason(&loaded).await.expect("a reason").reason,
        None
    );
    admit(&fleet).await.expect("admission runs");
    let started = fleet.load(&job).await.expect("the Job");
    assert_eq!(started.status(), JobStatus::Running);
    assert_eq!(started.worktree_slot(), Some(6));
}

/// A Job cut before the pool: no slot on its record, and a worktree at the
/// path its handle derives. It keeps working there until it ends.
#[tokio::test]
async fn a_job_with_no_slot_recorded_uses_its_derived_path() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("cut before the pool"))
        .await
        .expect("proposed");
    let derived = WorktreeSpec::for_job(&root(&home), &job.handle()).expect("a legal spec");
    std::fs::create_dir_all(derived.worktree_path()).expect("its old worktree");

    assert_eq!(job.worktree_slot(), None);
    let worktree = fleet
        .worktree_of(&job)
        .expect("readable")
        .expect("its old worktree is found");
    assert_eq!(worktree.path(), derived.worktree_path());
    assert_eq!(
        fleet.surviving_worktree(&job).expect("still there").path(),
        derived.worktree_path()
    );
    assert_eq!(
        fleet.vcs().slot_standing(
            &crate::leasing::pool_of(&fleet.first()),
            1,
            job.id().as_str()
        ),
        SlotStanding::Gone,
        "and it never touched the pool"
    );
}

/// Bridge's Cleanup reads the pool off `GET /worktrees`: every slot, and the
/// Job holding one named by its title as well as its id.
#[tokio::test]
async fn the_pool_crosses_the_wire_with_each_slot_and_its_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = approved(&fleet, &home, "fix the reader").await;

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, ipc::RunId::carried("01RUN"), events));
    let (status, body) = crate::tests::http::call(&app, "GET", "/worktrees", "").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let answer: ipc::WorktreesHeld =
        ipc::decode("what fleet is holding", &body).expect("the answer decodes");

    assert_eq!(answer.slots.len(), 8, "one row per slot the Manifest sizes");
    let first = &answer.slots[0];
    assert_eq!(first.slot, 1);
    assert_eq!(PathBuf::from(&first.path), slot(&home, 1));
    let ipc::SlotHolding::Job {
        job_id,
        job_title,
        job_status,
        kept,
        completed,
    } = &first.held
    else {
        panic!("a Job holds slot 1: {:?}", first.held);
    };
    assert_eq!(job_id, &ipc::JobId::carried(job.as_str()));
    assert_eq!(job_title.as_deref(), Some("fix the reader"));
    assert!(job_status.is_some(), "where the Job is travels with it");
    assert_eq!((kept, completed), (&None, &false));
    assert_eq!(first.base, "main");
    assert_eq!(answer.slots[1].held, ipc::SlotHolding::Unmade);
}

const DIRTY: &str =
    "/slots/slot-1 has 13 uncommitted, first src/log.rs. Commit or remove them, then release";

/// The pool refuses a slot holding uncommitted files. Clear commits them to the
/// Job's branch, which is what keeps them, and frees the slot.
#[tokio::test]
async fn clearing_a_slot_holding_uncommitted_files_commits_them_and_frees_it() {
    let home = TempDir::new();
    a_repository(&home);
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;
    fleet.vcs().keep_next_release(DIRTY);

    let (_, saved) = Fleet::cleared_worktree(&fleet, &job)
        .await
        .expect("a dirty slot is saved, not refused");

    let saved = saved.expect("the receipt names what was committed");
    assert_eq!(saved.files, vec!["wip.txt"]);
    assert_eq!(
        fleet.vcs().parked_slots(),
        vec![(1, job.as_str().to_string())]
    );
    assert_eq!(fleet.vcs().slot_holders(&root(&home))[0], None);
}

/// A park git refuses keeps today's refusal, said in git words and without the
/// instruction to run git.
#[tokio::test]
async fn a_slot_that_cannot_be_parked_stays_held_and_names_the_obstacle() {
    let home = TempDir::new();
    a_repository(&home);
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;
    fleet.vcs().keep_next_release(DIRTY);
    fleet
        .vcs()
        .refuse_next_park(adapter_traits::SlotParkRefused::OnNoBranch);

    let refused = Fleet::cleared_worktree(&fleet, &job)
        .await
        .expect_err("nothing to commit them to");

    let Adrift::SlotKept { why, .. } = refused else {
        panic!("{refused:?}");
    };
    assert!(why.contains("HEAD is detached"), "{why}");
    assert!(!why.contains("Commit or remove"), "{why}");
    assert!(fleet.vcs().parked_slots().is_empty());
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[0],
        Some(job.as_str().to_string())
    );
}

/// A clean slot is released as before, with nothing committed.
#[tokio::test]
async fn clearing_a_clean_slot_commits_nothing() {
    let home = TempDir::new();
    a_repository(&home);
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = completed(&fleet, &home, "fix the off-by-one").await;

    let (_, saved) = Fleet::cleared_worktree(&fleet, &job)
        .await
        .expect("cleared");

    assert!(saved.is_none());
    assert!(fleet.vcs().parked_slots().is_empty());
}
