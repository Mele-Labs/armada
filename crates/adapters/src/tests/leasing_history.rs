//! A slot's history: each lease and release the pool writes whoever made it, and each checkout
//! its reflog holds.

use std::path::Path;

use crate::leasing::{checkouts_in, checkouts_of, turns_of, Holder, Leased, Pool, KEPT_DAYS};
use crate::tests::repo::TempRepo;

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

fn took(pool: &Pool, branch: &str, holder: &Holder) -> std::path::PathBuf {
    match pool.try_lease(branch, holder, 7, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot taken: {other:?}"),
    }
}

/// A lease taken by a process, as `armada worktree lease` takes one, and given back, is two turns,
/// and the first says which session it was taken for and onto which branch.
#[test]
fn a_lease_and_its_release_are_each_written() {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new()).leased_for(Some("sess-9".to_string()));
    let slot = took(&pool, "bug-fix", &here());
    pool.release(&slot).expect("clean, so given back");

    let turns = turns_of(&slot, 0);
    assert_eq!(turns.len(), 2, "{turns:?}");
    assert!(turns[0].leased && !turns[1].leased);
    assert_eq!(turns[0].holder, here());
    assert_eq!(turns[0].session.as_deref(), Some("sess-9"));
    assert_eq!(turns[0].branch.as_deref(), Some("bug-fix"));
    assert_eq!(turns[1].holder, here());
    assert_eq!(turns[1].branch, None);
    assert!(turns[0].at_ms <= turns[1].at_ms);
}

/// A Job's lease is its id, and Fleet's own release of it is written the same way.
#[test]
fn a_jobs_lease_is_named_by_its_id() {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let job = Holder::hosted("job-1");
    let slot = took(&pool, "job-1-work", &job);
    pool.release_held(1, &job).expect("given back");

    let turns = turns_of(&slot, 0);
    assert_eq!(turns.iter().map(|turn| turn.leased).collect::<Vec<_>>(), [true, false]);
    assert!(turns.iter().all(|turn| turn.holder == job && turn.session.is_none()));
}

/// A holder that went without giving the slot back is released when the next lease takes it, so
/// the history never shows two leases in a row.
#[test]
fn a_lease_taken_from_a_gone_holder_releases_it_first() {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = took(&pool, "half-done", &here());
    let record = format!("{}.lease", slot.display());
    let Holder::Process { started, .. } = here() else { unreachable!() };
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(&record, text.replace(&started, "a start no process has")).unwrap();

    took(&pool, "next", &Holder::hosted("job-2"));

    let turns = turns_of(&slot, 0);
    let seen: Vec<(bool, bool)> = turns.iter().map(|turn| (turn.leased, matches!(turn.holder, Holder::Hosted(_)))).collect();
    assert_eq!(seen, [(true, false), (false, false), (true, true)], "{turns:?}");
}

/// Older than thirty days is cut on the next write, and is not read back either.
#[test]
fn what_is_older_than_thirty_days_is_cut() {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = pool.path_of(1);
    // An entry dated a second after the epoch is a month and more old by any machine's clock.
    std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
    std::fs::write(format!("{}.history", slot.display()), "1000\tleased\tjob long-ago\t-\told\n").unwrap();
    assert_eq!(turns_of(&slot, 0).len(), 1);
    assert!(turns_of(&slot, 1_728_000_000_000).is_empty());

    took(&pool, "fresh", &Holder::hosted("job-3"));

    let turns = turns_of(&slot, 0);
    assert_eq!(turns.len(), 1, "the old line is gone: {turns:?}");
    assert_eq!(turns[0].branch.as_deref(), Some("fresh"));
    assert_eq!(KEPT_DAYS, 30);
}

const REFLOG: &str = "\
0000000000000000000000000000000000000000 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa A <a@x> 1760000000 +0000\tcheckout: moving from main to bug-fix
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb A <a@x> 1760000100 -0400\tcommit: Fix it
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb A <a@x> 1760000200 -0400\tcheckout: moving from bug-fix to bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb cccccccccccccccccccccccccccccccccccccccc A <a@x> 1760000300 +0530\treset: moving to origin/main
cccccccccccccccccccccccccccccccccccccccc cccccccccccccccccccccccccccccccccccccccc A <a@x> 1760000400 +0000\tcheckout: moving from bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb to feature/x-to-y
this line is not a reflog line
";

/// Only `checkout: moving from X to Y` lines are checkouts, each with its instant in
/// epoch milliseconds whatever zone it was written in, and the commit `HEAD` stood at after it.
#[test]
fn only_checkout_lines_of_a_reflog_are_read() {
    let all = checkouts_in(REFLOG, 0);
    let seen: Vec<(i64, Option<&str>, &str, &str)> = all
        .iter()
        .map(|one| (one.at_ms, one.from.as_deref(), one.to.as_str(), &one.sha[..1]))
        .collect();
    assert_eq!(
        seen,
        [
            (1_760_000_000_000, Some("main"), "bug-fix", "a"),
            (1_760_000_200_000, Some("bug-fix"), "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "b"),
            (1_760_000_400_000, Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"), "feature/x-to-y", "c"),
        ]
    );
    assert_eq!(checkouts_in(REFLOG, 1_760_000_200_000).len(), 2, "since is inclusive");
    assert!(checkouts_in("", 0).is_empty());
}

/// The reflog is read off the worktree's own git directory, where a lease's `git switch` wrote it.
#[test]
fn a_slots_own_reflog_holds_its_leases_checkout() {
    let repo = TempRepo::with_a_commit();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = took(&pool, "bug-fix", &here());

    let all = checkouts_of(&slot, 0);
    let last = all.last().expect("the lease switched branch");
    assert_eq!(last.to, "bug-fix");
    assert_eq!(last.sha.len(), 40);
    assert!(checkouts_of(&repo.root().join("nowhere"), 0).is_empty());
}
