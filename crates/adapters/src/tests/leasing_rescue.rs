//! A stranded slot read, scrapped and stashed, against real repositories: what
//! it holds, what a scrap refuses to lose, and what a stash keeps on the remote.

use std::path::{Path, PathBuf};

use adapter_traits::{RescueRefused, SlotRescue};

use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

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

/// One slot, left by a holder that is gone, on branch `half-done` with one
/// commit nothing else has, a tracked file edited and a new file written.
fn a_stranded_slot(repo: &TempRepo) -> (Pool, PathBuf) {
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("half-done", &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot taken: {other:?}"),
    };
    std::fs::write(slot.join("kept.rs"), "fn kept() {}\n").unwrap();
    git(&slot, &["add", "kept.rs"]);
    git(&slot, &["commit", "-q", "-m", "Keep one"]);
    std::fs::write(slot.join("kept.rs"), "fn kept() { 1 }\n").unwrap();
    std::fs::write(slot.join("new.rs"), "fn new() {}\n").unwrap();
    (pool, slot)
}

/// Whether the slot can be leased again, which is what freed means.
fn is_free(pool: &Pool) -> bool {
    matches!(
        pool.try_lease("next", &here(), 0, &no_seed),
        Ok(Leased::Took(_))
    )
}

#[test]
fn a_stranded_slot_says_what_it_holds_and_its_change_against_the_base() {
    let repo = a_repository();
    let (pool, slot) = a_stranded_slot(&repo);

    let work = pool.stranded_work(1).expect("stranded");
    assert_eq!(work.branch.as_deref(), Some("half-done"));
    assert_eq!(work.commit, git(&slot, &["rev-parse", "HEAD"]));
    assert!(work
        .uncommitted
        .iter()
        .any(|path| path.ends_with("kept.rs")));
    assert!(work.uncommitted.iter().any(|path| path.ends_with("new.rs")));
    assert_eq!(work.commits.len(), 1);
    assert_eq!(work.commits[0].subject, "Keep one");
    assert_eq!(work.unpushed, 1);

    let diff = pool.stranded_diff(1).expect("stranded");
    assert!(diff.contains("+fn kept() { 1 }"), "{diff}");
}

/// The pool's reading carries what a stranded slot holds, the same as asking
/// the slot, and a slot that shows no work carries none.
#[test]
fn the_pools_reading_carries_what_a_stranded_slot_holds() {
    let repo = a_repository();
    let (pool, _) = a_stranded_slot(&repo);

    let read = pool.readings();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].work, Some(pool.stranded_work(1).expect("stranded")));

    let other = a_repository();
    let held_pool = Pool::at(other.root(), 1, "main", Vec::new());
    held_pool
        .try_lease("x", &here(), 0, &no_seed)
        .expect("a slot");
    let held = held_pool.readings();
    assert_eq!(held[0].work, None, "a live holder's slot shows no work");
}

/// Which readings show work is one rule, and a read that fails shows none.
#[test]
fn only_a_stranded_a_kept_or_a_dirty_session_slot_shows_work() {
    use adapter_traits::{SlotHeld, SlotReading, StrandedWork};

    let work = |files: &[&str]| StrandedWork {
        branch: None,
        commit: String::from("abc"),
        uncommitted: files.iter().map(|one| one.to_string()).collect(),
        commits: Vec::new(),
        unpushed: 0,
    };
    let reading = |held, kept: Option<&str>| SlotReading {
        slot: 1,
        path: String::new(),
        held,
        branch: None,
        since: None,
        warm: false,
        behind: None,
        closed: false,
        kept: kept.map(str::to_string),
        completed: false,
        work: None,
    };
    let job = || SlotHeld::Job(String::from("01J"));

    assert!(reading(SlotHeld::Stranded(String::new()), None)
        .with_work(|_| Some(work(&[])))
        .work
        .is_some());
    assert!(reading(job(), Some("dirty"))
        .with_work(|_| Some(work(&[])))
        .work
        .is_some());
    assert!(reading(job(), None)
        .with_work(|_| panic!("a Job's live slot is not read"))
        .work
        .is_none());
    let session = || reading(SlotHeld::Session(String::from("zsh")), None);
    assert!(session().with_work(|_| Some(work(&[]))).work.is_none());
    assert!(session().with_work(|_| Some(work(&["a.rs"]))).work.is_some());
    assert!(session().with_work(|_| None).work.is_none());
}

/// **Each commit says where else it exists**: on a remote branch, on the local
/// base, or only in the slot, which is what a Scrap would lose. The base here
/// is `origin/main`, so a commit local `main` has and the remote does not is
/// the slot's and the base's both.
#[test]
fn each_commit_says_whether_it_exists_anywhere_but_the_slot() {
    use adapter_traits::CommitHome::{OnMain, OnRemote, OnlyHere};

    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("half-done", &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot taken: {other:?}"),
    };
    let commit = |file: &str, message: &str| {
        std::fs::write(slot.join(file), message).unwrap();
        git(&slot, &["add", file]);
        git(&slot, &["commit", "-q", "-m", message]);
    };
    commit("pushed.rs", "Pushed elsewhere");
    git(
        &slot,
        &["push", "-q", "origin", "HEAD:refs/heads/someone-elses"],
    );
    repo.commit_one("landed.rs", "fn landed() {}\n", "Landed here only");
    git(&slot, &["merge", "-q", "--no-edit", "main"]);
    commit("mine.rs", "Only in the slot");
    // Commits on its branch no longer strand a slot; a dirty file does.
    std::fs::write(slot.join("wip.rs"), "not committed").unwrap();

    let work = pool.stranded_work(1).expect("stranded");
    let home = |subject: &str| {
        work.commits
            .iter()
            .find(|one| one.subject == subject)
            .unwrap_or_else(|| panic!("no commit {subject:?} in {:?}", work.commits))
            .home
    };
    assert_eq!(home("Only in the slot"), OnlyHere);
    assert_eq!(home("Pushed elsewhere"), OnRemote);
    assert_eq!(home("Landed here only"), OnMain);
    assert!(
        work.commits[0].home == OnlyHere,
        "newest first, and it is the slot's"
    );
}

#[test]
fn a_slot_that_is_not_stranded_is_neither_read_nor_acted_on() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    match pool.try_lease("held", &here(), 0, &no_seed) {
        Ok(Leased::Took(_)) => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        pool.stranded_work(1),
        Err(RescueRefused::NotStranded(_))
    ));
    assert!(matches!(
        pool.rescue(1, SlotRescue::Scrap),
        Err(RescueRefused::NotStranded(_))
    ));
    assert!(matches!(
        pool.stranded_work(9),
        Err(RescueRefused::NoSuchSlot(9))
    ));
}

/// **A scrap throws the uncommitted files away and frees the slot, and keeps
/// the branch while it holds a commit nothing else has.**
#[test]
fn a_scrap_discards_the_files_and_keeps_a_branch_with_unpushed_commits() {
    let repo = a_repository();
    let (pool, slot) = a_stranded_slot(&repo);

    let done = pool.rescue(1, SlotRescue::Scrap).expect("scrapped");
    assert!(done.branch_kept);
    assert_eq!(done.branch.as_deref(), Some("half-done"));
    assert!(!slot.join("new.rs").exists());
    assert!(git(&slot, &["status", "--porcelain"]).is_empty());
    assert!(!git(repo.root(), &["branch", "--list", "half-done"]).is_empty());
    assert!(is_free(&pool));
}

/// A branch the base already holds goes with the scrap.
#[test]
fn a_scrap_deletes_a_branch_the_base_holds_in_full() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("nothing-new", &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{other:?}"),
    };
    std::fs::write(slot.join("scratch.rs"), "x").unwrap();

    let done = pool.rescue(1, SlotRescue::Scrap).expect("scrapped");
    assert!(!done.branch_kept);
    assert!(git(repo.root(), &["branch", "--list", "nothing-new"]).is_empty());
    assert!(is_free(&pool));
}

/// **A stash commits the uncommitted files to the branch and pushes it under
/// its own name**, so the remote has everything the slot held.
#[test]
fn a_stash_commits_and_pushes_the_branch_and_frees_the_slot() {
    let repo = a_repository();
    let (pool, _slot) = a_stranded_slot(&repo);

    let done = pool
        .rescue(
            1,
            SlotRescue::Stash {
                message: String::from("Work left in slot-1, kept"),
            },
        )
        .expect("stashed");
    assert!(done.committed.is_some());
    assert!(done.branch_kept);
    let pushed = git(
        repo.root(),
        &["ls-remote", "--heads", "origin", "half-done"],
    );
    assert!(!pushed.is_empty(), "the branch is on the remote");
    let tip = git(repo.root(), &["rev-parse", "half-done"]);
    assert!(pushed.starts_with(&tip), "the remote has the stash commit");
    let files = git(
        repo.root(),
        &["show", "--name-only", "--format=", "half-done"],
    );
    assert!(
        files.contains("new.rs") && files.contains("kept.rs"),
        "{files}"
    );
    assert!(is_free(&pool));
}

#[test]
fn a_stash_with_no_remote_changes_nothing() {
    let repo = TempRepo::with_a_commit();
    let (pool, slot) = a_stranded_slot(&repo);
    assert!(matches!(
        pool.rescue(
            1,
            SlotRescue::Stash {
                message: String::from("kept")
            }
        ),
        Err(RescueRefused::NoRemote)
    ));
    assert!(slot.join("new.rs").exists());
}

/// A Job's slot the Job could not give back: leased by its id, dirtied, and
/// the release refused, which records why beside the slot.
fn a_kept_slot(repo: &TempRepo) -> (Pool, PathBuf) {
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("armada/killed", &Holder::job("JOB1"), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot taken: {other:?}"),
    };
    std::fs::write(slot.join("only-copy.rs"), "fn only() {}\n").unwrap();
    assert!(pool.release_held(1, &Holder::job("JOB1")).is_err());
    (pool, slot)
}

/// **A kept slot reads as one, with the reason**, so the bay can say why the
/// Job's slot is still held.
#[test]
fn a_job_slot_whose_release_was_refused_reads_as_kept() {
    let repo = a_repository();
    let (pool, _slot) = a_kept_slot(&repo);

    let reading = &pool.readings()[0];
    assert!(matches!(reading.held, adapter_traits::SlotHeld::Job(ref id) if id == "JOB1"));
    assert!(reading.kept.as_deref().is_some_and(|why| !why.is_empty()));
    assert!(!reading.completed);
    let work = pool.stranded_work(1).expect("read as a stranded one is");
    assert!(work
        .uncommitted
        .iter()
        .any(|path| path.ends_with("only-copy.rs")));
}

/// **Each act ends the Job's claim**, so the slot is free after and the
/// Job's lease record is gone.
#[test]
fn a_scrap_of_a_kept_slot_ends_the_jobs_claim() {
    let repo = a_repository();
    let (pool, slot) = a_kept_slot(&repo);

    pool.rescue(1, SlotRescue::Scrap).expect("scrapped");
    assert!(!slot.join("only-copy.rs").exists());
    assert!(matches!(
        pool.readings()[0].held,
        adapter_traits::SlotHeld::Free
    ));
    assert!(is_free(&pool));
}

#[test]
fn a_stash_of_a_kept_slot_pushes_the_jobs_branch_and_ends_the_claim() {
    let repo = a_repository();
    let (pool, _slot) = a_kept_slot(&repo);

    pool.rescue(
        1,
        SlotRescue::Stash {
            message: String::from("kept"),
        },
    )
    .expect("stashed");
    let pushed = git(
        repo.root(),
        &["ls-remote", "--heads", "origin", "armada/killed"],
    );
    assert!(!pushed.is_empty(), "the Job's branch is on the remote");
    assert!(matches!(
        pool.readings()[0].held,
        adapter_traits::SlotHeld::Free
    ));
    assert!(is_free(&pool));
}

/// A Job's slot that was not refused is a live one, and no rescue's.
#[test]
fn a_live_job_slot_is_not_rescued() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    match pool.try_lease("armada/live", &Holder::job("JOB2"), 0, &no_seed) {
        Ok(Leased::Took(_)) => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        pool.rescue(1, SlotRescue::Scrap),
        Err(RescueRefused::NotStranded(_))
    ));
}
