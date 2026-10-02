//! The worktree pool, against real repositories: what a lease takes, what it
//! keeps, and what a release refuses to throw away.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::leasing::{Full, Holder, Leased, Pool, ReleaseRefused, SlotState};
use crate::tests::repo::TempRepo;

/// A repository with a remote that has `main`, and a `.gitignore` that
/// ignores the build directory and logs, as a real one would.
fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n*.log\n", "ignore builds");
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

/// A holder whose process has ended: a pid recorded beside a start time no
/// process carries.
fn gone() -> Holder {
    Holder::recorded(std::process::id(), "a start no process has")
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn take(pool: &Pool, branch: &str, holder: &Holder) -> PathBuf {
    match pool.try_lease(branch, holder, 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{branch} took no slot: {other:?}"),
    }
}

fn git(at: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args([
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
        ])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(
        run.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn commit_in(at: &Path, file: &str) {
    std::fs::write(at.join(file), file).unwrap();
    git(at, &["add", "--", file]);
    git(at, &["commit", "-q", "-m", file]);
}

#[test]
fn two_leases_take_two_slots_each_on_its_own_branch_at_the_base() {
    let repo = a_repository();
    let pool = pool(&repo, 8);
    let one = take(&pool, "one", &here());
    let two = take(&pool, "two", &here());

    assert_ne!(one, two);
    assert_eq!(git(&one, &["branch", "--show-current"]), "one");
    assert_eq!(git(&two, &["branch", "--show-current"]), "two");
    assert_eq!(git(&one, &["rev-parse", "HEAD"]), repo.head_str());
}

#[test]
fn a_ninth_lease_waits_while_eight_are_held_and_takes_the_one_released() {
    let repo = a_repository();
    let pool = pool(&repo, 8);
    let held: Vec<PathBuf> = (1..=8)
        .map(|n| take(&pool, &format!("held-{n}"), &here()))
        .collect();

    match pool.try_lease("ninth", &here(), 0, &no_seed) {
        Ok(Leased::Full(Full { slots })) => assert_eq!(slots.len(), 8),
        other => panic!("a ninth lease took something: {other:?}"),
    }

    let (waiting, heard) = mpsc::channel();
    let waiter = {
        let pool = pool.clone();
        std::thread::spawn(move || {
            let mut told = false;
            pool.lease("ninth", &here(), 0, &no_seed, |_| {
                if !told {
                    told = true;
                    let _ = waiting.send(());
                }
            })
        })
    };
    heard.recv().expect("the ninth lease said it was waiting");
    pool.release(&held[2])
        .expect("a clean, landed slot is released");

    let ninth = waiter.join().unwrap().expect("the ninth lease took a slot");
    assert_eq!(ninth.path(), held[2]);
    assert_eq!(git(&held[2], &["branch", "--show-current"]), "ninth");
}

#[test]
fn a_lease_keeps_the_build_and_resets_everything_else_to_the_base() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "first", &here());
    std::fs::create_dir_all(slot.join("target/debug")).unwrap();
    std::fs::write(slot.join("target/debug/kept"), "warm").unwrap();
    std::fs::write(slot.join("build.log"), "ignored, and not warm").unwrap();
    commit_in(&slot, "first.rs");
    git(&slot, &["push", "-q", "origin", "first"]);
    pool.release(&slot).expect("pushed and clean");

    let again = take(&pool, "second", &here());
    assert_eq!(again, slot, "the one slot is reused");
    assert_eq!(
        std::fs::read_to_string(slot.join("target/debug/kept")).unwrap(),
        "warm"
    );
    assert!(!slot.join("build.log").exists());
    assert!(!slot.join("first.rs").exists());
    assert_eq!(git(&slot, &["rev-parse", "HEAD"]), repo.head_str());
}

#[test]
fn a_release_refuses_a_dirty_tree_and_holds_the_slot() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "dirty", &here());
    std::fs::write(slot.join("notes.md"), "not committed").unwrap();

    assert!(matches!(
        pool.release(&slot),
        Err(ReleaseRefused::Dirty { .. })
    ));
    assert!(matches!(
        pool.try_lease("next", &here(), 0, &no_seed),
        Ok(Leased::Full(_))
    ));
}

#[test]
fn a_release_refuses_commits_on_neither_the_remote_nor_the_base() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "unlanded", &here());
    commit_in(&slot, "only-here.rs");

    match pool.release(&slot) {
        Err(ReleaseRefused::Unlanded { commits, .. }) => assert_eq!(commits, 1),
        other => panic!("an unpushed commit was let go: {other:?}"),
    }
}

#[test]
fn a_dead_holders_slot_is_reclaimed_only_once_its_tree_is_clean() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "abandoned", &gone());
    std::fs::write(slot.join("half-done.rs"), "written, never committed").unwrap();

    match pool.try_lease("next", &here(), 0, &no_seed) {
        Ok(Leased::Full(Full { slots })) => assert!(
            matches!(slots[0].state, SlotState::Stranded { .. }),
            "{:?}",
            slots[0].state
        ),
        other => panic!("a dirty slot was taken from a dead holder: {other:?}"),
    }

    std::fs::remove_file(slot.join("half-done.rs")).unwrap();
    match pool.try_lease("next", &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => {
            assert_eq!(lease.path(), slot);
            assert_eq!(lease.reclaimed_from(), Some("abandoned"));
        }
        other => panic!("a clean slot was not reclaimed: {other:?}"),
    }
}

#[test]
fn a_branch_holding_unlanded_work_is_not_reset_by_a_lease() {
    let repo = a_repository();
    repo.git(&["branch", "kept"]);
    repo.git(&["checkout", "-q", "kept"]);
    repo.commit_one("kept.rs", "work", "work nobody pushed");
    repo.git(&["checkout", "-q", "main"]);

    assert!(pool(&repo, 1)
        .try_lease("kept", &here(), 0, &no_seed)
        .is_err());
}

/// The shell between a command and its caller ends with the command; the
/// caller does not, so it is what a lease is held for.
#[test]
fn the_caller_is_a_running_process_that_is_not_this_one() {
    let caller = Holder::the_caller().expect("a test runs under something");
    assert!(caller.alive());
    assert_ne!(caller.pid(), std::process::id());
    assert!(!gone().alive());
}

#[test]
fn a_new_slot_is_seeded_from_a_warm_base() {
    let repo = a_repository();
    let commit = repo.head_str();
    let base = repo.root().join(".armada/bases").join(&commit);
    std::fs::create_dir_all(base.join("target/debug")).unwrap();
    std::fs::write(base.join("target/debug/built"), "warm").unwrap();
    std::fs::write(base.join(".armada-seed-warm"), &commit).unwrap();

    let copy = |from: &Path, to: &Path| -> Result<(), String> {
        std::fs::create_dir_all(to.join("debug")).map_err(|e| e.to_string())?;
        std::fs::copy(from.join("debug/built"), to.join("debug/built"))
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    let pool = Pool::at(repo.root(), 1, "main", vec![String::from("target")]);
    let lease = match pool.try_lease("seeded", &here(), 0, &copy) {
        Ok(Leased::Took(lease)) => lease,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        std::fs::read_to_string(lease.path().join("target/debug/built")).unwrap(),
        "warm"
    );
    assert_eq!(lease.seeded_from(), Some(commit.as_str()));
}
