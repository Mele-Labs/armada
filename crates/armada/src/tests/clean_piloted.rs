//! `armada clean` and a Job a person is working in. `docs/concepts/pilot.md`.
//! #367.
//!
//! **Real git and a real store**, for `crate::tests::clean`'s reason: the
//! question is whether a checkout is still there afterwards.

use adapter_traits::{Vcs, WorktreeSpec};
use adapters::{GitVcs, UnmergedWork};
use core_model::{Actor, PilotReason, Target};
use store::Store;

use crate::clean::{clean, Scope};
use crate::serve::STORE_FILE;
use crate::tests::clean::{a_job, a_repository, at, branches, JOB, MANIFEST_ID};
use crate::tests::TempDir;

/// A Job at `piloted`, with its worktree and branch on disk.
fn a_piloted_job(machine: &std::path::Path, repo: &std::path::Path) -> WorktreeSpec {
    let spec = WorktreeSpec::for_job(&repo.to_string_lossy(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    let mut store = Store::open(&machine.join(STORE_FILE)).expect("a store");
    let mut job = a_job(JOB, MANIFEST_ID);
    store.insert_job(&job, &at()).expect("the job is stored");
    for target in [
        Target::Queued,
        Target::Running,
        Target::Piloted(PilotReason::TakeOver),
    ] {
        let moved = job
            .transition(target, Actor::Human, at())
            .expect("a legal move");
        store.record_transition(&moved).expect("recorded");
        job = moved.job;
    }
    spec
}

#[test]
fn a_clean_leaves_a_piloted_jobs_checkout_and_branch_and_names_it() {
    let repo = a_repository();
    let machine = TempDir::new();
    let spec = a_piloted_job(machine.path(), repo.path());

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(cleaned.jobs.is_empty(), "nothing was given back");
    assert!(
        std::path::Path::new(&spec.worktree_path()).is_dir(),
        "the checkout is still there"
    );
    assert!(branches(repo.path()).contains(&spec.branch()));
    let kept = &cleaned.piloted;
    assert_eq!(kept.len(), 1, "and it is named: {kept:?}");
    assert_eq!(kept[0].job_id, JOB);
    assert_eq!(kept[0].path, spec.worktree_path());
}

#[test]
fn force_does_not_reach_a_piloted_jobs_checkout() {
    let repo = a_repository();
    let machine = TempDir::new();
    let spec = a_piloted_job(machine.path(), repo.path());

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Delete,
    )
    .expect("a forced clean");

    assert!(std::path::Path::new(&spec.worktree_path()).is_dir());
    assert!(branches(repo.path()).contains(&spec.branch()));
    assert_eq!(cleaned.piloted.len(), 1);
}
