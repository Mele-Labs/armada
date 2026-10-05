//! Parking a slot's work on its branch, and leasing an existing branch at its
//! tip, against real repositories.

use std::path::{Path, PathBuf};

use crate::leasing::{Holder, LeaseRefused, Leased, ParkRefused, Pool, SlotState};
use crate::tests::repo::TempRepo;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n*.log\n", "ignore builds");
    repo.commit_one("tracked.txt", "one\n", "a tracked file");
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

fn gone() -> Holder {
    Holder::Process {
        pid: std::process::id(),
        started: String::from("a start no process has"),
    }
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

#[test]
fn a_park_commits_untracked_and_modified_files_and_frees_the_slot() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "paused", &here());
    std::fs::write(slot.join("tracked.txt"), "two\n").unwrap();
    std::fs::write(slot.join("new.rs"), "fn new() {}").unwrap();
    std::fs::write(slot.join("build.log"), "ignored").unwrap();

    let parked = pool.park(1, &here()).expect("a dirty slot is parked");

    let committed = parked.committed.expect("the work was committed");
    assert_eq!(parked.branch, "paused");
    assert!(committed.files.iter().any(|f| f == "new.rs"));
    assert!(committed.files.iter().any(|f| f == "tracked.txt"));
    let tip = git(repo.root(), &["rev-parse", "refs/heads/paused"]);
    assert_eq!(tip, committed.commit);
    assert!(git(repo.root(), &["log", "-1", "--format=%s", &tip]).starts_with("WIP"));
    let kept = git(repo.root(), &["ls-tree", "-r", "--name-only", &tip]);
    assert!(
        kept.contains("new.rs") && !kept.contains("build.log"),
        "{kept}"
    );
    assert_eq!(
        git(repo.root(), &["show", &format!("{tip}:tracked.txt")]),
        "two"
    );
    assert_eq!(pool.state(1), SlotState::Free);
    assert!(
        git(repo.root(), &["branch", "-r", "--contains", &tip]).is_empty(),
        "a park never pushes"
    );
}

#[test]
fn a_park_of_a_clean_slot_only_releases_it() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    take(&pool, "idle", &here());
    let before = git(repo.root(), &["rev-parse", "refs/heads/idle"]);

    let parked = pool.park(1, &here()).expect("a clean slot is parked");

    assert!(parked.committed.is_none());
    assert_eq!(git(repo.root(), &["rev-parse", "refs/heads/idle"]), before);
    assert_eq!(pool.state(1), SlotState::Free);
}

#[test]
fn a_park_refuses_on_the_base_and_on_a_detached_head_and_changes_nothing() {
    let repo = a_repository();
    // The base is free to be checked out in a slot only where the repository's
    // own checkout is elsewhere.
    repo.git(&["checkout", "-q", "-b", "elsewhere"]);
    let pool = pool(&repo, 1);
    let slot = take(&pool, "wrong", &here());
    std::fs::write(slot.join("work.rs"), "x").unwrap();

    git(&slot, &["switch", "--quiet", "main"]);
    assert!(matches!(
        pool.park(1, &here()),
        Err(ParkRefused::OnTheBase(_))
    ));
    git(&slot, &["switch", "--quiet", "--detach"]);
    assert!(matches!(
        pool.park(1, &here()),
        Err(ParkRefused::OnNoBranch)
    ));
    assert!(
        slot.join("work.rs").exists(),
        "nothing was committed or lost"
    );
    assert_eq!(git(&slot, &["status", "--porcelain"]), "?? work.rs");
    assert!(matches!(pool.state(1), SlotState::Held { .. }));
}

#[test]
fn a_park_refuses_a_slot_another_holds() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    take(&pool, "theirs", &Holder::job("job-1"));
    assert!(matches!(
        pool.park(1, &here()),
        Err(ParkRefused::HeldByAnother(_))
    ));
}

#[test]
fn a_park_refuses_while_a_take_or_release_is_under_way() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    take(&pool, "busy", &here());
    let lock = std::fs::OpenOptions::new()
        .append(true)
        .open(repo.root().join(".armada/slots/slot-1.lease"))
        .unwrap();
    lock.lock().unwrap();

    assert!(matches!(pool.park(1, &here()), Err(ParkRefused::Busy)));
}

#[test]
fn a_lease_of_an_existing_branch_puts_the_slot_at_its_tip() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    let slot = take(&pool, "paused", &here());
    std::fs::write(slot.join("saved.rs"), "saved").unwrap();
    let parked = pool.park(1, &here()).unwrap();
    let tip = parked.committed.unwrap().commit;

    let leased = match pool.try_lease_existing("paused", &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease,
        other => panic!("the parked branch took no slot: {other:?}"),
    };

    assert_eq!(git(leased.path(), &["branch", "--show-current"]), "paused");
    assert_eq!(git(leased.path(), &["rev-parse", "HEAD"]), tip);
    assert_eq!(
        std::fs::read_to_string(leased.path().join("saved.rs")).unwrap(),
        "saved"
    );
}

#[test]
fn a_lease_of_an_existing_branch_refuses_one_checked_out_elsewhere_by_name() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    let held = take(&pool, "busy-elsewhere", &here());

    match pool.try_lease_existing("busy-elsewhere", &here(), 0, &no_seed) {
        Err(LeaseRefused::CheckedOutElsewhere { branch, at }) => {
            assert_eq!(branch, "busy-elsewhere");
            assert_eq!(at.canonicalize().unwrap(), held.canonicalize().unwrap());
        }
        other => panic!("a branch another slot has was leased: {other:?}"),
    }
}

#[test]
fn a_lease_of_an_existing_branch_refuses_a_branch_that_is_not_there() {
    let repo = a_repository();
    assert!(matches!(
        pool(&repo, 1).try_lease_existing("nope", &here(), 0, &no_seed),
        Err(LeaseRefused::NoSuchBranch(_))
    ));
}

#[test]
fn a_lease_of_an_existing_branch_is_full_while_every_slot_is_held() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    take(&pool, "one", &here());
    repo.git(&["branch", "other"]);
    assert!(matches!(
        pool.try_lease_existing("other", &here(), 0, &no_seed),
        Ok(Leased::Full(_))
    ));
}

#[test]
fn a_dead_holders_slot_with_a_commit_on_its_branch_is_taken_back() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let slot = take(&pool, "left-behind", &gone());
    std::fs::write(slot.join("done.rs"), "x").unwrap();
    git(&slot, &["add", "done.rs"]);
    git(&slot, &["commit", "-q", "-m", "done"]);
    let tip = git(&slot, &["rev-parse", "HEAD"]);

    match pool.try_lease("next", &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => assert_eq!(lease.reclaimed_from(), Some("left-behind")),
        other => panic!("a committed branch was held against a new lease: {other:?}"),
    }
    assert_eq!(
        git(repo.root(), &["rev-parse", "refs/heads/left-behind"]),
        tip
    );
}
