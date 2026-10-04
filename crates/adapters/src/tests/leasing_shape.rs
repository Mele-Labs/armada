//! The pool reshaped on this machine: a slot added, removed, closed and
//! reopened, against real repositories, and what a lease makes of each.

use std::path::{Path, PathBuf};

use crate::leasing::{Holder, Leased, Pool, SlotState, Unshaped};
use crate::tests::repo::TempRepo;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn pool(repo: &TempRepo, count: usize) -> Pool {
    Pool::at(repo.root(), count, "main", Vec::new())
}

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

/// The slot a lease took, or `None` for a full pool.
fn took(pool: &Pool, branch: &str) -> Option<usize> {
    match pool.try_lease(branch, &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => Some(lease.slot()),
        Ok(Leased::Full(_)) => None,
        Err(why) => panic!("{branch}: {why:?}"),
    }
}

fn path_of(pool: &Pool, branch: &str) -> PathBuf {
    match pool.try_lease(branch, &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{branch} took no slot: {other:?}"),
    }
}

#[test]
fn a_closed_slot_is_never_leased_until_it_is_opened() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    pool.close(1).expect("slot-1 closes");

    assert_eq!(took(&pool, "one"), Some(2), "the open slot is taken");
    assert_eq!(took(&pool, "two"), None, "the closed slot is not");
    assert!(!pool.open_for(&Holder::job("waiting")), "nor would a Job's");

    pool.open(1).expect("slot-1 opens");
    assert_eq!(took(&pool, "two"), Some(1));
}

#[test]
fn a_held_slot_closed_keeps_its_holder_and_stays_closed_once_released() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = path_of(&pool, "held");
    pool.close(1).expect("a held slot closes");

    let status = pool.status();
    assert!(status[0].closed);
    assert!(matches!(status[0].state, SlotState::Held { .. }));

    pool.release(&slot).expect("its holder gives it back");
    assert_eq!(took(&pool, "next"), None, "released, it stays closed");
}

#[test]
fn an_added_slot_is_not_made_until_a_lease_makes_it_and_outlives_the_pool() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    assert_eq!(pool.add().expect("a slot is added"), 2);
    assert!(matches!(pool.state(2), SlotState::Unmade));

    // A new `Pool` is a restart: the Manifest still says one.
    let again = Pool::at(repo.root(), 1, "main", Vec::new());
    assert_eq!(again.bays(), vec![1, 2]);
    assert_eq!(took(&again, "one"), Some(1));
    assert_eq!(took(&again, "two"), Some(2));
}

#[test]
fn removing_takes_the_slot_pressed_and_gives_its_checkout_back() {
    let repo = a_repository();
    let pool = pool(&repo, 3);
    let made = path_of(&pool, "made");
    pool.release(&made).expect("clean and landed");
    assert_eq!(made, pool.path_of(1));

    pool.remove(1).expect("a free, made slot goes");
    assert!(!made.exists(), "its checkout is gone");
    let listed = repo.git(&["worktree", "list"]);
    assert!(!listed.contains("slot-1"), "{listed}");
    assert_eq!(pool.bays(), vec![2, 3], "the others keep their numbers");

    assert_eq!(pool.add().expect("added"), 1, "the lowest number unused");
}

#[test]
fn a_slot_not_made_is_removed_with_nothing_on_disk_to_give_back() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    pool.close(2).expect("closed first");
    pool.remove(2).expect("an unmade slot goes");
    assert_eq!(pool.bays(), vec![1]);
    assert!(!pool.closed(2));
}

#[test]
fn a_held_slot_is_not_removed() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    path_of(&pool, "held");
    assert!(matches!(pool.remove(1), Err(Unshaped::Held(_))));
    assert_eq!(pool.bays(), vec![1, 2]);
}

#[test]
fn a_stranded_slot_is_not_removed() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    let gone = Holder::Process {
        pid: std::process::id(),
        started: String::from("a start no process has"),
    };
    let slot = match pool.try_lease("stranded", &gone, 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{other:?}"),
    };
    std::fs::write(slot.join("half-done.rs"), "never committed").unwrap();

    assert!(matches!(pool.remove(1), Err(Unshaped::Stranded(_))));
    assert!(slot.join("half-done.rs").exists());
}

#[test]
fn the_last_slot_is_not_removed() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    assert_eq!(pool.remove(1), Err(Unshaped::LastSlot));
}

#[test]
fn a_slot_the_pool_does_not_have_is_refused_by_name() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    assert_eq!(pool.close(5), Err(Unshaped::NoSuchSlot(5)));
    assert_eq!(pool.remove(5), Err(Unshaped::NoSuchSlot(5)));
}
