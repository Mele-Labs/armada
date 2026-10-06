//! A release trims stale build output and leaves a slot warm: what the
//! pool keeps is what a recent build wrote, and a `target/` past its ceiling
//! goes whole.

use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::leasing::{Holder, Leased, Pool, Trim};
use crate::tests::repo::TempRepo;

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn lease(pool: &Pool) -> std::path::PathBuf {
    match pool.try_lease("branch", &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("took no slot: {other:?}"),
    }
}

/// A file of `bytes` bytes last written `days_ago` days ago.
fn built(at: &Path, bytes: usize, days_ago: u32) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, vec![0u8; bytes]).unwrap();
    let then = SystemTime::now() - DAY * days_ago;
    std::fs::File::options()
        .write(true)
        .open(at)
        .unwrap()
        .set_modified(then)
        .unwrap();
}

fn trim() -> Trim {
    Trim {
        older_than: DAY * 14,
        ceiling_bytes: 1_000_000,
    }
}

#[test]
fn a_release_drops_what_no_build_has_touched_in_a_fortnight_and_keeps_the_rest() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new()).trimming(trim());
    let slot = lease(&pool);
    built(&slot.join("target/debug/deps/old-1.rlib"), 10, 30);
    built(&slot.join("target/debug/deps/fresh-1.rlib"), 10, 1);
    built(&slot.join("target/debug/incremental/old-sess/dep-graph.bin"), 10, 30);
    built(&slot.join("target/debug/incremental/new-sess/dep-graph.bin"), 10, 1);

    pool.release(&slot).expect("a clean slot is released");

    assert!(!slot.join("target/debug/deps/old-1.rlib").exists());
    assert!(slot.join("target/debug/deps/fresh-1.rlib").exists());
    assert!(
        !slot.join("target/debug/incremental/old-sess").exists(),
        "a stale session directory goes, and not only its files"
    );
    assert!(slot.join("target/debug/incremental/new-sess/dep-graph.bin").exists());
}

#[test]
fn a_release_removes_a_target_that_is_still_over_the_ceiling() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new()).trimming(trim());
    let slot = lease(&pool);
    built(&slot.join("target/debug/deps/fresh-1.rlib"), 2_000_000, 1);

    pool.release(&slot).expect("a clean slot is released");

    assert!(!slot.join("target").exists());
}

#[test]
fn a_release_leaves_a_target_under_the_ceiling_and_a_slot_with_none() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new()).trimming(trim());
    let slot = lease(&pool);
    built(&slot.join("target/debug/deps/fresh-1.rlib"), 500_000, 1);
    pool.release(&slot).expect("a clean slot is released");
    assert!(slot.join("target/debug/deps/fresh-1.rlib").exists());

    let bare = lease(&pool);
    assert!(!bare.join("target").exists());
    pool.release(&bare).expect("a slot with no build is released");
}
