//! Bringing a base into a worktree while another process holds `index.lock`,
//! asked of a real repository. **A lock is transient, and the Job it caught
//! must not be escalated for it**: on 2026-10-11 one was, with ninety files of
//! passing work in the tree, on `index.lock': File exists.

use std::path::PathBuf;
use std::time::Duration;

use adapter_traits::{Vcs, Worktree, WorktreeSpec};

use crate::merging_in::{merged_in, MergedIn, OnConflict};
use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K9LOCKED00000000000001";

fn a_worktree(repo: &TempRepo) -> Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree")
}

/// Where git keeps this worktree's own `index.lock`, which is not in its
/// working directory.
fn index_lock(worktree: &Worktree) -> PathBuf {
    let run = std::process::Command::new("git")
        .args(["-C", worktree.path(), "rev-parse", "--absolute-git-dir"])
        .output()
        .expect("git on PATH");
    PathBuf::from(String::from_utf8_lossy(&run.stdout).trim()).join("index.lock")
}

/// Hold the lock now, and let it go after `for_how_long`, as the git that took
/// it finishing would.
fn held_for(worktree: &Worktree, for_how_long: Duration) -> std::thread::JoinHandle<()> {
    let lock = index_lock(worktree);
    std::fs::write(&lock, "").expect("the lock");
    std::thread::spawn(move || {
        std::thread::sleep(for_how_long);
        std::fs::remove_file(lock).expect("the lock goes");
    })
}

#[test]
fn a_clean_merge_waits_out_a_lock_another_process_holds() {
    let repo = TempRepo::with_a_commit();
    let worktree = a_worktree(&repo);
    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    let released = held_for(&worktree, Duration::from_millis(700));

    let merged = merged_in(&worktree, "main", OnConflict::PutItBack);
    released.join().expect("the lock was released");

    assert!(
        matches!(merged, Ok(MergedIn::Clean { commits: 1 })),
        "a lock held for under a second is not a failure: {merged:?}"
    );
}

#[test]
fn uncommitted_work_is_set_aside_and_put_back_across_a_lock() {
    let repo = TempRepo::with_a_commit();
    repo.commit_one("shared.txt", "as it was\n", "a file both will touch");
    let worktree = a_worktree(&repo);
    // Edited in place where the base is about to write, so the merge refuses
    // before it starts and the work has to be stashed.
    std::fs::write(
        format!("{}/shared.txt", worktree.path()),
        "still in progress\n",
    )
    .expect("the file");
    repo.commit_one("shared.txt", "what somebody else merged\n", "got there first");
    let released = held_for(&worktree, Duration::from_millis(700));

    let merged = merged_in(&worktree, "main", OnConflict::PutItBack);
    released.join().expect("the lock was released");

    assert!(
        matches!(merged, Ok(MergedIn::PutBack { .. })),
        "the work conflicts, and that is the answer — not the lock: {merged:?}"
    );
    assert_eq!(
        std::fs::read_to_string(format!("{}/shared.txt", worktree.path())).expect("the file"),
        "still in progress\n",
        "the work is back where it was"
    );
}

/// **Only the lock is waited out.** A refusal for any other reason comes back
/// at once, as it did.
#[test]
fn a_lock_that_stays_is_the_answer_after_a_bounded_wait() {
    let repo = TempRepo::with_a_commit();
    let worktree = a_worktree(&repo);
    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    let lock = index_lock(&worktree);
    std::fs::write(&lock, "").expect("the lock");

    let started = std::time::Instant::now();
    let merged = merged_in(&worktree, "main", OnConflict::PutItBack);
    std::fs::remove_file(&lock).expect("the lock goes");

    let said = format!("{:?}", merged.expect_err("a lock nobody releases"));
    assert!(said.contains("index.lock"), "{said}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the wait is bounded: {:?}",
        started.elapsed()
    );
}
