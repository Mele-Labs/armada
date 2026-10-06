//! A slot leased for a Job: held by the Job's id, never taken back for a dead
//! process, and given back only by the pool's rules.

use std::path::{Path, PathBuf};

use adapter_traits::{SlotHeld, SlotLeased, SlotPool, SlotStanding, Vcs, WorktreeSpec};

use crate::leasing::{Holder, Leased, Pool, SlotState};
use crate::tests::repo::TempRepo;
use crate::GitVcs;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn branch_of(at: &Path) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["branch", "--show-current"])
        .output()
        .expect("git on PATH");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn slots(repo: &TempRepo, count: u32) -> SlotPool {
    SlotPool::of(&repo.root().to_string_lossy(), count, "main", Vec::new())
}

fn spec(repo: &TempRepo, handle: &str) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root().to_string_lossy(), handle).expect("a legal spec")
}

fn leased(repo: &TempRepo, pool: &SlotPool, handle: &str, job: &str) -> (u32, PathBuf) {
    match GitVcs.lease_slot(pool, &spec(repo, handle), job) {
        Ok(SlotLeased::Took { slot, worktree, .. }) => (slot, PathBuf::from(worktree.path())),
        other => panic!("{job} took no slot: {other:?}"),
    }
}

#[test]
fn a_job_leases_a_slot_on_its_own_branch_and_holds_it_by_its_id() {
    let repo = a_repository();
    let pool = slots(&repo, 2);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");

    assert_eq!(path, repo.root().join(".armada/slots/slot-1"));
    assert_eq!(slot, 1);
    assert_eq!(branch_of(&path), "armada/1-a-job");
    assert_eq!(GitVcs.slot_standing(&pool, 1, "01JOB"), SlotStanding::Held);
    assert!(matches!(
        GitVcs.slot_standing(&pool, 1, "01OTHER"),
        SlotStanding::HeldBy(_)
    ));
}

/// Fleet restarting is a new pid; the lease names no pid at all.
#[test]
fn a_job_s_lease_reads_as_held_whatever_process_asks() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    leased(&repo, &pool, "1-a-job", "01JOB");

    let status = Pool::at(repo.root(), 1, "main", Vec::new()).status();
    match &status[0].state {
        SlotState::Held { holder, .. } => assert_eq!(holder, &Holder::job("01JOB")),
        other => panic!("a Job's slot read as {other:?}"),
    }
    let next = Pool::at(repo.root(), 1, "main", Vec::new()).try_lease(
        "someone-else",
        &Holder::of(std::process::id()).unwrap(),
        0,
        &no_seed,
    );
    assert!(
        matches!(next, Ok(Leased::Full(_))),
        "a clean Job's slot was taken back: {next:?}"
    );
}

#[test]
fn a_job_that_already_holds_a_slot_is_handed_the_same_one_untouched() {
    let repo = a_repository();
    let pool = slots(&repo, 2);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    std::fs::write(path.join("half-done.rs"), "the Drone's work").unwrap();

    let (again, at) = leased(&repo, &pool, "1-a-job", "01JOB");
    assert_eq!((again, at.clone()), (slot, path));
    assert!(
        at.join("half-done.rs").exists(),
        "the second lease reset it"
    );
}

#[test]
fn the_pool_is_open_to_a_job_only_while_a_slot_is_free_or_its_own() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    assert!(GitVcs.slot_open(&pool, "01ONE"));
    leased(&repo, &pool, "1-one", "01ONE");

    assert!(GitVcs.slot_open(&pool, "01ONE"));
    assert!(!GitVcs.slot_open(&pool, "01TWO"));
    assert_eq!(
        GitVcs
            .lease_slot(&pool, &spec(&repo, "2-two"), "01TWO")
            .unwrap(),
        SlotLeased::Full
    );
}

#[test]
fn a_release_refused_keeps_the_slot_held_and_says_why_in_the_status() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    std::fs::write(path.join("notes.md"), "never committed").unwrap();

    let kept = GitVcs
        .release_slot(&pool, slot, "01JOB")
        .expect_err("a dirty tree is not given back");
    assert!(kept.0.contains("notes.md"), "{}", kept.0);
    match &Pool::at(repo.root(), 1, "main", Vec::new()).status()[0].state {
        SlotState::Held {
            kept: Some(why), ..
        } => assert!(why.contains("notes.md")),
        other => panic!("the refusal is not on the slot: {other:?}"),
    }

    std::fs::remove_file(path.join("notes.md")).unwrap();
    GitVcs
        .release_slot(&pool, slot, "01JOB")
        .expect("clean, with nothing on it");
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Free
    );
}

#[test]
fn a_job_cannot_give_back_a_slot_another_holds() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "1-a-job", "01JOB");

    assert!(GitVcs.release_slot(&pool, slot, "01OTHER").is_err());
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Held
    );
}

#[test]
fn a_slot_nobody_made_is_gone() {
    let repo = a_repository();
    assert_eq!(
        GitVcs.slot_standing(&slots(&repo, 2), 2, "01JOB"),
        SlotStanding::Gone
    );
}

/// A completed Job's slot says so, for `--status`, and stays held until the
/// Job's release; the release clears it with the rest of the record.
#[test]
fn a_completed_job_s_slot_says_so_until_it_is_released() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "1-a-job", "01JOB");

    GitVcs.mark_slot_completed(&pool, slot, "01OTHER");
    GitVcs.mark_slot_completed(&pool, slot, "01JOB");

    let status = || Pool::at(repo.root(), 1, "main", Vec::new()).status();
    match &status()[0].state {
        SlotState::Held {
            holder, completed, ..
        } => {
            assert_eq!(holder, &Holder::job("01JOB"));
            assert!(completed, "the holder's mark, and only the holder's");
        }
        other => panic!("a completed Job's slot read as {other:?}"),
    }
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Held
    );

    GitVcs
        .release_slot(&pool, slot, "01JOB")
        .expect("clean, with nothing on it");
    assert_eq!(status()[0].state, SlotState::Free);
}

/// What Bridge's Cleanup draws: the Job's slot on its branch, warm once its
/// seed path is there, and behind once the base moves past it.
#[test]
fn the_pool_reads_each_slot_with_its_holder_warmth_and_lag() {
    let repo = a_repository();
    let root = repo.root().to_string_lossy().to_string();
    let pool = SlotPool::of(&root, 2, "main", vec![String::from("target")]);
    let (_, path) = leased(&repo, &pool, "1-a-job", "01JOB");

    let cold = GitVcs.slot_pool(&pool);
    assert_eq!(cold.len(), 2);
    assert_eq!(cold[0].held, SlotHeld::Job(String::from("01JOB")));
    assert_eq!(cold[0].branch.as_deref(), Some("armada/1-a-job"));
    assert!(cold[0].since.is_some(), "the lease records when");
    assert!(!cold[0].warm, "no build yet");
    assert_eq!(cold[0].behind, Some(0));
    assert_eq!(cold[1].held, SlotHeld::Unmade);
    assert_eq!((cold[1].warm, cold[1].behind), (false, None));

    std::fs::create_dir_all(path.join("target/debug")).expect("a build directory");
    repo.commit_one("one.txt", "one\n", "the base moves");
    repo.commit_one("two.txt", "two\n", "and again");
    repo.git(&["push", "-q", "origin", "main"]);

    let warm = GitVcs.slot_pool(&pool);
    assert!(warm[0].warm);
    assert_eq!(warm[0].behind, Some(2));
}

/// A pause's whole road through the real pool: the work is committed to the
/// Job's own branch, the slot goes to somebody else, and the Job comes back to
/// whichever slot is free with its file at the branch's tip.
#[test]
fn a_parked_jobs_work_is_at_its_branch_s_tip_in_whichever_slot_it_comes_back_to() {
    let repo = a_repository();
    let pool = slots(&repo, 2);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    std::fs::write(path.join("wip.txt"), "half a thought\n").expect("a file to keep");

    let parked = GitVcs.park_slot(&pool, slot, "01JOB").expect("it parks");
    assert_eq!(parked.branch, "armada/1-a-job");
    assert!(parked.commit.is_some(), "the untracked file was committed");
    assert_eq!(parked.files, vec!["wip.txt"]);
    let subject = repo.git(&["log", "-1", "--format=%s", &parked.branch]);
    assert!(
        subject.contains("WIP") && subject.contains("job 01JOB"),
        "the commit names the Job and says it is work in progress: {subject}"
    );
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Free
    );

    // Somebody else takes slot 1 while the Job is parked.
    let (taken, _) = leased(&repo, &pool, "2-another", "01OTHER");
    assert_eq!(taken, slot, "the freed slot is the first one free");

    let back = match GitVcs.lease_existing_slot(&pool, &parked.branch, "01JOB") {
        Ok(SlotLeased::Took { slot, worktree, .. }) => (slot, PathBuf::from(worktree.path())),
        other => panic!("the Job came back to no slot: {other:?}"),
    };
    assert_ne!(back.0, slot, "it came back to the other slot");
    assert_eq!(branch_of(&back.1), "armada/1-a-job");
    assert_eq!(
        std::fs::read_to_string(back.1.join("wip.txt")).expect("the file came with the branch"),
        "half a thought\n"
    );
    assert_eq!(
        GitVcs.slot_standing(&pool, back.0, "01JOB"),
        SlotStanding::Held
    );
}

#[test]
fn a_park_refused_by_the_pool_says_so_and_keeps_the_slot() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "1-a-job", "01JOB");

    let refused = GitVcs
        .park_slot(&pool, slot, "01SOMEBODY")
        .expect_err("not theirs");
    assert!(matches!(
        refused,
        adapter_traits::SlotParkRefused::HeldByAnother(_)
    ));
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Held
    );
}
