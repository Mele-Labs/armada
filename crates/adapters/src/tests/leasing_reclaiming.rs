//! What a slot holds once the pull request from its branch merged, against real repositories:
//! the read Fleet gives the slot back on.

use std::path::{Path, PathBuf};

use adapter_traits::SlotAfterMerge;

use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn git(at: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&run.stderr));
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// A pool of one slot leased on `feat`, whose one commit was pushed, merged into main with a
/// merge commit, and whose remote branch was deleted: how a pull request lands here.
fn a_merged_slot() -> (TempRepo, Pool, PathBuf) {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    // A test thread that spawns git while another holds a slot's lock leaves the child a copy of
    // the lock's descriptor until it execs, so a lease can find its slot busy for a moment. The
    // pool asks again, as `armada worktree lease` does, and so does this.
    let mut leased = pool.try_lease("feat", &here(), 0, &no_seed);
    for _ in 0..50 {
        if matches!(leased, Ok(Leased::Took(_))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        leased = pool.try_lease("feat", &here(), 0, &no_seed);
    }
    let Ok(Leased::Took(lease)) = leased else {
        panic!("feat took no slot: {leased:?}");
    };
    let slot = lease.path().to_path_buf();
    std::fs::write(slot.join("work.rs"), "fn work() {}").unwrap();
    git(&slot, &["add", "."]);
    git(&slot, &["commit", "-q", "-m", "the work"]);
    git(&slot, &["push", "-q", "origin", "feat"]);
    git(repo.root(), &["merge", "-q", "--no-ff", "-m", "merge feat", "feat"]);
    git(repo.root(), &["push", "-q", "origin", "main"]);
    git(repo.root(), &["push", "-q", "origin", "--delete", "feat"]);
    git(repo.root(), &["fetch", "-q", "--prune"]);
    (repo, pool, slot)
}

#[test]
fn a_clean_slot_still_on_the_merged_branch_is_idle() {
    let (_repo, pool, _slot) = a_merged_slot();

    assert_eq!(pool.after_merge(1, "feat"), SlotAfterMerge::Idle);
}

#[test]
fn a_slot_detached_on_a_commit_the_base_has_is_idle() {
    let (_repo, pool, slot) = a_merged_slot();
    git(&slot, &["switch", "-q", "--detach", "origin/main"]);

    assert_eq!(pool.after_merge(1, "feat"), SlotAfterMerge::Idle);
}

#[test]
fn a_commit_made_after_the_merge_keeps_the_slot_and_is_named() {
    let (_repo, pool, slot) = a_merged_slot();
    std::fs::write(slot.join("more.rs"), "fn more() {}").unwrap();
    git(&slot, &["add", "."]);
    git(&slot, &["commit", "-q", "-m", "follow-up"]);

    assert_eq!(
        pool.after_merge(1, "feat"),
        SlotAfterMerge::Unpushed(1)
    );
}

#[test]
fn an_uncommitted_file_keeps_the_slot_and_is_named() {
    let (_repo, pool, slot) = a_merged_slot();
    std::fs::write(slot.join("scratch.rs"), "fn scratch() {}").unwrap();

    assert_eq!(
        pool.after_merge(1, "feat"),
        SlotAfterMerge::Dirty(vec![String::from("scratch.rs")])
    );
}

#[test]
fn a_new_branch_cut_in_the_slot_is_new_work() {
    let (_repo, pool, slot) = a_merged_slot();
    git(&slot, &["switch", "-q", "-c", "follow-up"]);

    assert_eq!(pool.after_merge(1, "feat"), SlotAfterMerge::Moved);
}

#[test]
fn a_commit_made_on_a_detached_head_keeps_the_slot() {
    let (_repo, pool, slot) = a_merged_slot();
    git(&slot, &["switch", "-q", "--detach", "origin/main"]);
    std::fs::write(slot.join("more.rs"), "fn more() {}").unwrap();
    git(&slot, &["add", "."]);
    git(&slot, &["commit", "-q", "-m", "loose"]);

    assert_eq!(pool.after_merge(1, "feat"), SlotAfterMerge::Unpushed(1));
}

#[test]
fn a_slot_detached_on_a_pushed_commit_the_base_lacks_is_new_work() {
    let (repo, pool, slot) = a_merged_slot();
    git(repo.root(), &["switch", "-q", "-c", "other"]);
    std::fs::write(repo.root().join("other.rs"), "fn other() {}").unwrap();
    git(repo.root(), &["add", "."]);
    git(repo.root(), &["commit", "-q", "-m", "someone else's work"]);
    git(repo.root(), &["push", "-q", "origin", "other"]);
    git(repo.root(), &["switch", "-q", "main"]);
    git(&slot, &["fetch", "-q"]);
    git(&slot, &["switch", "-q", "--detach", "origin/other"]);

    assert_eq!(pool.after_merge(1, "feat"), SlotAfterMerge::Moved);
}
