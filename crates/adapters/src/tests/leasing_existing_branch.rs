//! A lease of a branch that already exists, against real repositories: it
//! lands on the branch's own tip and never moves it, whether the branch sits
//! in a slot whose holder is gone, in no slot, or only on `origin`.

use std::path::{Path, PathBuf};

use crate::leasing::{Holder, Lease, LeaseRefused, Leased, Pool};
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

fn gone() -> Holder {
    Holder::Process {
        pid: std::process::id(),
        started: String::from("a start no process has"),
    }
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
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

fn lease(pool: &Pool, branch: &str) -> Lease {
    match pool.try_lease(branch, &here(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease,
        other => panic!("{branch} took no slot: {other:?}"),
    }
}

fn commit_in(at: &Path, file: &str) -> String {
    std::fs::write(at.join(file), file).unwrap();
    git(at, &["add", "--", file]);
    git(at, &["commit", "-q", "-m", file]);
    git(at, &["rev-parse", "HEAD"])
}

/// A slot left on `branch` by a holder that is gone, with one commit on it
/// that is not on the base, and the base moved on past it.
fn a_dead_holders_slot(
    repo: &TempRepo,
    pool: &Pool,
    branch: &str,
    push: bool,
) -> (PathBuf, String) {
    let slot = match pool.try_lease(branch, &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{branch} took no slot: {other:?}"),
    };
    let tip = commit_in(&slot, "work.rs");
    if push {
        git(&slot, &["push", "-q", "origin", branch]);
    }
    repo.commit_one("later.rs", "main moved", "the base moves on");
    repo.git(&["push", "-q", "origin", "main"]);
    (slot, tip)
}

#[test]
fn a_pushed_branch_in_a_slot_whose_holder_is_gone_is_leased_at_its_own_tip() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let (slot, tip) = a_dead_holders_slot(&repo, &pool, "feature", true);

    let lease = lease(&pool, "feature");

    assert_eq!(lease.path(), slot);
    assert_eq!(lease.reclaimed_from(), Some("feature"));
    assert!(!lease.cut_from_base());
    assert_eq!(git(&slot, &["branch", "--show-current"]), "feature");
    assert_eq!(git(&slot, &["rev-parse", "HEAD"]), tip);
    assert_eq!(repo.git(&["rev-parse", "feature"]), tip);
}

#[test]
fn an_unpushed_branch_in_a_slot_whose_holder_is_gone_is_leased_at_its_own_tip() {
    let repo = a_repository();
    let pool = pool(&repo, 1);
    let (slot, tip) = a_dead_holders_slot(&repo, &pool, "feature", false);

    let lease = lease(&pool, "feature");

    assert_eq!(lease.path(), slot);
    assert_eq!(git(&slot, &["rev-parse", "HEAD"]), tip);
    assert_eq!(repo.git(&["rev-parse", "feature"]), tip);
}

#[test]
fn a_dead_holders_slot_with_uncommitted_files_is_not_taken_back_for_its_own_branch() {
    let repo = a_repository();
    // A second, free slot: the lease must not slip past the stranded one onto
    // it, because git would then refuse the branch it is already checked out on.
    let pool = pool(&repo, 2);
    let (slot, tip) = a_dead_holders_slot(&repo, &pool, "feature", true);
    std::fs::write(slot.join("work.rs"), "edited, never committed").unwrap();
    std::fs::write(slot.join("notes.md"), "never added").unwrap();

    match pool.try_lease("feature", &here(), 0, &no_seed) {
        Err(LeaseRefused::CheckedOutElsewhere { at, .. }) => {
            assert_eq!(at.canonicalize().unwrap(), slot.canonicalize().unwrap())
        }
        other => panic!("a slot holding uncommitted files was taken back: {other:?}"),
    }
    assert_eq!(git(&slot, &["branch", "--show-current"]), "feature");
    assert_eq!(git(&slot, &["rev-parse", "HEAD"]), tip);
    assert_eq!(
        std::fs::read_to_string(slot.join("work.rs")).unwrap(),
        "edited, never committed"
    );
    assert_eq!(
        std::fs::read_to_string(slot.join("notes.md")).unwrap(),
        "never added"
    );
}

#[test]
fn a_branch_a_live_holder_has_is_refused_rather_than_moved() {
    let repo = a_repository();
    let pool = pool(&repo, 2);
    let mine = lease(&pool, "feature");
    let tip = commit_in(mine.path(), "work.rs");

    match pool.try_lease("feature", &here(), 0, &no_seed) {
        Err(LeaseRefused::CheckedOutElsewhere { .. }) => {}
        other => panic!("a held branch was leased twice: {other:?}"),
    }
    assert_eq!(git(mine.path(), &["rev-parse", "HEAD"]), tip);
}

#[test]
fn a_branch_in_no_slot_is_leased_at_its_own_tip_not_the_base() {
    let repo = a_repository();
    repo.git(&["switch", "-q", "-c", "parked"]);
    repo.commit_one("parked.rs", "work", "work on a branch");
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["push", "-q", "origin", "parked"]);
    repo.git(&["switch", "-q", "main"]);
    repo.commit_one("later.rs", "main moved", "the base moves on");
    repo.git(&["push", "-q", "origin", "main"]);

    let lease = lease(&pool(&repo, 1), "parked");

    assert!(!lease.cut_from_base());
    assert_eq!(git(lease.path(), &["branch", "--show-current"]), "parked");
    assert_eq!(git(lease.path(), &["rev-parse", "HEAD"]), tip);
    assert_eq!(repo.git(&["rev-parse", "parked"]), tip);
}

#[test]
fn a_branch_only_origin_has_is_leased_at_its_tip_and_tracks_it() {
    let repo = a_repository();
    repo.git(&["switch", "-q", "-c", "elsewhere"]);
    repo.commit_one("elsewhere.rs", "work", "work pushed from another machine");
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["push", "-q", "origin", "elsewhere"]);
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["branch", "-q", "-D", "elsewhere"]);
    repo.git(&["update-ref", "-d", "refs/remotes/origin/elsewhere"]);

    let lease = lease(&pool(&repo, 1), "elsewhere");

    assert!(!lease.cut_from_base());
    assert_eq!(git(lease.path(), &["branch", "--show-current"]), "elsewhere");
    assert_eq!(git(lease.path(), &["rev-parse", "HEAD"]), tip);
    assert_eq!(
        git(lease.path(), &["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/elsewhere"
    );
}

#[test]
fn a_name_nothing_has_is_still_cut_from_the_base() {
    let repo = a_repository();

    let lease = lease(&pool(&repo, 1), "brand-new");

    assert!(lease.cut_from_base());
    assert_eq!(git(lease.path(), &["rev-parse", "HEAD"]), repo.head_str());
}

#[test]
fn leasing_an_existing_branch_nothing_has_is_refused() {
    let repo = a_repository();

    match pool(&repo, 1).try_lease_existing("nowhere", &here(), 0, &no_seed) {
        Err(LeaseRefused::NoSuchBranch(branch)) => assert_eq!(branch, "nowhere"),
        other => panic!("a branch nothing has was leased: {other:?}"),
    }
}
