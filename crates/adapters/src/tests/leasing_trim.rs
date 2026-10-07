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
    built(
        &slot.join("target/debug/incremental/old-sess/dep-graph.bin"),
        10,
        30,
    );
    built(
        &slot.join("target/debug/incremental/new-sess/dep-graph.bin"),
        10,
        1,
    );

    pool.release(&slot).expect("a clean slot is released");

    assert!(!slot.join("target/debug/deps/old-1.rlib").exists());
    assert!(slot.join("target/debug/deps/fresh-1.rlib").exists());
    assert!(
        !slot.join("target/debug/incremental/old-sess").exists(),
        "a stale session directory goes, and not only its files"
    );
    assert!(slot
        .join("target/debug/incremental/new-sess/dep-graph.bin")
        .exists());
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
fn a_release_leaves_a_target_under_the_ceiling() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new()).trimming(trim());
    let slot = lease(&pool);
    built(&slot.join("target/debug/deps/fresh-1.rlib"), 500_000, 1);
    pool.release(&slot).expect("a clean slot is released");
    assert!(slot.join("target/debug/deps/fresh-1.rlib").exists());
}

#[test]
fn a_sweep_trims_the_main_checkout_and_a_job_worktree_and_leaves_a_building_one() {
    use crate::leasing::sweep_repository;

    let repo = a_repository();
    let root = repo.root();
    let job = root.join(".armada/worktrees/a-job");
    let agent = root.join(".claude/worktrees/agent-1");
    for checkout in [root, job.as_path(), agent.as_path()] {
        built(&checkout.join("target/debug/deps/old-1.rlib"), 10, 30);
        built(&checkout.join("target/debug/deps/fresh-1.rlib"), 10, 1);
    }
    let over = root.join(".armada/bases/abc");
    built(&over.join("target/debug/deps/fresh-1.rlib"), 2_000_000, 1);

    // A build in `agent` holds cargo's lock on its profile directory.
    let lock = agent.join("target/debug/.cargo-lock");
    std::fs::write(&lock, "").unwrap();
    let building = std::fs::File::options().write(true).open(&lock).unwrap();
    building.lock().unwrap();

    sweep_repository(root, trim(), SystemTime::now(), &[]);

    for checkout in [root, job.as_path()] {
        assert!(!checkout.join("target/debug/deps/old-1.rlib").exists());
        assert!(checkout.join("target/debug/deps/fresh-1.rlib").exists());
    }
    assert!(!over.join("target").exists(), "over the ceiling goes whole");
    assert!(
        agent.join("target/debug/deps/old-1.rlib").exists(),
        "nothing is deleted under a live build"
    );

    drop(building);
    sweep_repository(root, trim(), SystemTime::now(), &[]);
    assert!(!agent.join("target/debug/deps/old-1.rlib").exists());
}

#[test]
fn a_sweep_leaves_the_target_of_a_checkout_it_was_told_a_person_holds() {
    use crate::leasing::sweep_repository;

    let repo = a_repository();
    let root = repo.root();
    let piloted = root.join(".armada/worktrees/a-piloted-job");
    let other = root.join(".armada/worktrees/another-job");
    for checkout in [piloted.as_path(), other.as_path()] {
        built(&checkout.join("target/debug/deps/old-1.rlib"), 10, 30);
    }

    sweep_repository(root, trim(), SystemTime::now(), &[piloted.clone()]);

    assert!(
        piloted.join("target/debug/deps/old-1.rlib").exists(),
        "a person's build output is theirs"
    );
    assert!(!other.join("target/debug/deps/old-1.rlib").exists());
}
