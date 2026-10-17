//! A pool is full when every slot is held, stranded, closed or not a checkout, and not while a slot
//! is only locked or not made for a moment. **Whoever hears "full" acts on it**: a session is
//! queued and the person asked what to do, so a look that was wrong for a few milliseconds must not
//! be the answer. The cause seen: the lock a lease drops is held on by a process spawned in the
//! instant before, until it execs, so the second look under the slot's lock finds it taken.

use std::fs::OpenOptions;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn a_pool_of_one() -> (TempRepo, Pool) {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    (repo, pool)
}

/// Another process holding slot 1's lock, as a lease or a release does while it works.
fn locked_by_another(repo: &TempRepo) -> std::fs::File {
    let slots = repo.root().join(".armada/slots");
    std::fs::create_dir_all(&slots).unwrap();
    let file = OpenOptions::new().create(true).append(true).open(slots.join("slot-1.lease")).unwrap();
    file.lock().expect("locked");
    file
}

#[test]
fn a_slot_locked_for_a_moment_is_not_a_full_pool_and_nobody_is_told_it_is() {
    let (repo, pool) = a_pool_of_one();
    let lock = locked_by_another(&repo);
    let told = AtomicBool::new(false);

    let leased = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(700));
            drop(lock);
        });
        pool.lease("mine", &Holder::hosted("me"), 0, &no_seed, |_| told.store(true, Ordering::SeqCst))
    });

    assert!(leased.is_ok(), "it took the slot once the lock was let go");
    assert!(!told.load(Ordering::SeqCst), "a lock held for a moment was reported as every slot held");
}

#[test]
fn a_try_lease_rides_out_a_lock_that_is_let_go_at_once() {
    let (repo, pool) = a_pool_of_one();
    let lock = locked_by_another(&repo);

    let leased = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(60));
            drop(lock);
        });
        pool.try_lease("mine", &Holder::hosted("me"), 0, &no_seed)
    });

    assert!(matches!(leased, Ok(Leased::Took(_))), "{leased:?}");
}

#[test]
fn a_pool_whose_every_slot_is_held_is_full_and_says_so() {
    let (_repo, pool) = a_pool_of_one();
    assert!(matches!(pool.try_lease("theirs", &Holder::hosted("someone"), 0, &no_seed), Ok(Leased::Took(_))));

    let Ok(Leased::Full(full)) = pool.try_lease("mine", &Holder::hosted("me"), 0, &no_seed) else {
        panic!("every slot is held");
    };

    assert!(full.settled());
}

#[test]
fn a_pool_whose_only_slot_is_closed_is_full_and_says_so() {
    let (_repo, pool) = a_pool_of_one();
    pool.close(1).expect("closed");

    let Ok(Leased::Full(full)) = pool.try_lease("mine", &Holder::hosted("me"), 0, &no_seed) else {
        panic!("a closed slot is not leased");
    };

    assert!(full.settled());
}
