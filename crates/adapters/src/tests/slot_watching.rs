//! A kept reading of the pool reads the slots that were touched, and the rest
//! from memory: against real git and the real file system, since what is
//! under test is which events arrive.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use adapter_traits::{SlotHeld, SlotLeased, SlotPool, Vcs, WorktreeSpec};

use crate::leasing::watching::{heard_of, last_reread};
use crate::tests::repo::TempRepo;
use crate::GitVcs;

/// A pool of three: slot 1 held by a Job, slots 2 and 3 free and made.
fn a_pool_with_two_free_slots() -> (TempRepo, SlotPool) {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\nnode_modules/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    let pool = SlotPool::of(&repo.root().to_string_lossy(), 3, "main", Vec::new());
    for (handle, job) in [("1-a", "01A"), ("2-b", "01B"), ("3-c", "01C")] {
        let spec = WorktreeSpec::for_job(&repo.root().to_string_lossy(), handle).expect("a spec");
        assert!(matches!(
            GitVcs.lease_slot(&pool, &spec, job),
            Ok(SlotLeased::Took { .. })
        ));
    }
    GitVcs.release_slot(&pool, 2, "01B").expect("released");
    GitVcs.release_slot(&pool, 3, "01C").expect("released");
    (repo, pool)
}

fn slot(repo: &TempRepo, number: usize) -> PathBuf {
    repo.root().join(format!(".armada/slots/slot-{number}"))
}

fn write(at: &Path, relative: &str) {
    let path = at.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
    std::fs::write(path, "x\n").expect("a file");
}

/// What has been heard, once something has.
fn heard_soon(root: &Path) -> (bool, Vec<usize>) {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let heard = heard_of(root);
        if heard.0 || !heard.1.is_empty() || Instant::now() > until {
            return heard;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn settled() {
    std::thread::sleep(Duration::from_millis(400));
}

/// A read, a pause for whatever the read itself stirred, and one more.
fn kept_and_quiet(repo: &TempRepo, pool: &SlotPool) {
    GitVcs.slot_pool_kept(pool, true);
    settled();
    GitVcs.slot_pool_kept(pool, false);
    assert_eq!(last_reread(repo.root()), Vec::<usize>::new(), "reading moved something");
}

#[test]
fn nothing_touched_reads_nothing_and_reading_touches_nothing() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);
    assert_eq!(heard_of(repo.root()), (false, Vec::new()));
}

#[test]
fn an_edit_in_a_free_slot_reads_that_slot_alone() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 2), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![2]));

    let read = GitVcs.slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![2]);
    assert!(matches!(read[1].held, SlotHeld::Stranded(_)), "{:?}", read[1].held);
    assert_eq!(read[2].held, SlotHeld::Free);
}

#[test]
fn a_build_and_an_install_in_a_free_slot_are_not_an_edit() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 2), "target/debug/deps/a.rlib");
    write(&slot(&repo, 2), "node_modules/y/index.js");
    // The control: a real edit elsewhere, heard after both.
    write(&slot(&repo, 3), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![3]));
    settled();
    assert_eq!(heard_of(repo.root()), (false, vec![3]));
}

#[test]
fn an_edit_in_a_held_slot_is_not_heard() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 1), "notes.txt");
    write(&slot(&repo, 3), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![3]));
    settled();
    assert_eq!(heard_of(repo.root()), (false, vec![3]));
}

#[test]
fn a_commit_in_a_held_slot_moves_that_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    repo.git(&[
        "-C",
        &slot(&repo, 1).to_string_lossy(),
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.com",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "work",
    ]);
    assert_eq!(heard_soon(repo.root()), (false, vec![1]));
    settled();
    assert_eq!(heard_of(repo.root()), (false, vec![1]), "the branch moving is not the base moving");
    GitVcs.slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1]);
}

#[test]
fn the_base_moving_reads_every_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    repo.commit_one("one.txt", "one\n", "the base moves");
    repo.git(&["push", "-q", "origin", "main"]);
    assert!(heard_soon(repo.root()).0);
    let read = GitVcs.slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
    assert_eq!(read[0].behind, Some(1));
}

#[test]
fn a_full_read_goes_to_git_for_every_slot_whatever_was_heard() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    GitVcs.slot_pool_kept(&pool, true);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
}

#[test]
fn what_this_adapter_did_is_read_again_without_waiting_for_the_file_system() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    GitVcs.release_slot(&pool, 1, "01A").expect("released");
    GitVcs.slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
}
