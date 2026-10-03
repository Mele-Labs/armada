//! `armada clean` against a Job that holds a pool slot.
//!
//! A slot is shared by every lease after this one, so the clean never removes
//! it: it is given back by the pool's own rules or left held and named.

use std::path::{Path, PathBuf};

use adapter_traits::{CommitTime, SlotLeased, SlotPool, Vcs, Worktree, WorktreeSpec};
use adapters::leasing::{Pool, SlotState};
use adapters::{GitVcs, UnmergedWork};
use store::Store;

use super::clean::{a_job, a_repository, at, branches, git, JOB, MANIFEST_ID};
use crate::clean::{clean, Cleaned, Holding, Scope};
use crate::serve::STORE_FILE;
use crate::tests::TempDir;

const NINE: CommitTime = CommitTime::seconds_since_epoch(1_787_734_800);

fn slots(repo: &Path) -> SlotPool {
    SlotPool::of(&repo.to_string_lossy(), 8, "main", Vec::new())
}

/// [`JOB`], leased into a slot and recorded there, as Fleet's dispatch does.
/// Answers the slot's path and the Job's branch.
fn a_job_in_a_slot(machine: &Path, repo: &Path) -> (PathBuf, String) {
    let job = a_job(JOB, MANIFEST_ID);
    let spec = WorktreeSpec::for_job(&repo.to_string_lossy(), &job.handle()).expect("a spec");
    let Ok(SlotLeased::Took { slot, worktree, .. }) = GitVcs.lease_slot(&slots(repo), &spec, JOB)
    else {
        panic!("the Job took no slot");
    };
    let job = job.in_slot(slot);
    let mut store = Store::open(&machine.join(STORE_FILE)).expect("a store");
    store.insert_job(&job, &at()).expect("the job is stored");
    store.record_slot(&job).expect("its slot is recorded");
    (PathBuf::from(worktree.path()), spec.branch())
}

fn a_commit(slot: &Path, branch: &str) {
    std::fs::write(slot.join("answer.txt"), "42").expect("the work");
    GitVcs::new()
        .commit_all(
            &Worktree::at(slot.to_string_lossy(), branch.to_string()),
            "the work",
            NINE,
        )
        .expect("Fleet commits when the last step advances");
}

fn completed(repo: &Path) {
    GitVcs.mark_slot_completed(&slots(repo), 1, JOB);
}

fn state(repo: &Path) -> SlotState {
    Pool::at(repo, 8, "main", Vec::new()).state(1)
}

fn cleaned(repo: &Path, machine: &Path, unmerged: UnmergedWork) -> Cleaned {
    clean(repo, machine, Scope::Repository, unmerged).expect("a clean")
}

fn held_by_the_job(state: &SlotState) -> bool {
    matches!(state, SlotState::Held { holder, .. } if *holder == adapters::leasing::Holder::job(JOB))
}

// ------------------------------------------------------------ plain clean

/// Its branch is checked out in the slot, so git refused the delete and the
/// clean reported a fault for a slot that was only doing its job.
#[test]
fn a_plain_clean_names_a_completed_jobs_slot_and_leaves_its_branch() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, branch) = a_job_in_a_slot(machine.path(), repo.path());
    a_commit(&slot, &branch);
    completed(repo.path());

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Keep);

    assert!(cleaned.faults.is_empty(), "{:?}", cleaned.faults);
    assert!(cleaned.jobs.is_empty(), "{:?}", cleaned.jobs);
    assert_eq!(cleaned.held.len(), 1);
    let held = &cleaned.held[0];
    assert_eq!((held.job_id.as_str(), held.slot), (JOB, 1));
    assert_eq!(held.why, Holding::Completed);
    assert!(branches(repo.path()).contains(&branch));
    assert!(held_by_the_job(&state(repo.path())));
}

#[test]
fn a_plain_clean_names_a_kept_slot_with_the_reason_it_was_kept() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, _) = a_job_in_a_slot(machine.path(), repo.path());
    std::fs::write(slot.join("notes.md"), "never committed").unwrap();
    GitVcs
        .release_slot(&slots(repo.path()), 1, JOB)
        .expect_err("a dirty tree is kept");

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Keep);

    let Holding::Kept(why) = &cleaned.held[0].why else {
        panic!("{:?}", cleaned.held);
    };
    assert!(why.contains("notes.md"), "{why}");
    assert!(slot.join("notes.md").exists());
}

// ------------------------------------------------------------------ --force

#[test]
fn force_releases_a_completed_jobs_landed_slot_then_deletes_its_branch() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, branch) = a_job_in_a_slot(machine.path(), repo.path());
    a_commit(&slot, &branch);
    git(repo.path(), &["merge", "--ff-only", "--quiet", &branch]);
    completed(repo.path());

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Delete);

    assert!(cleaned.faults.is_empty(), "{:?}", cleaned.faults);
    assert!(cleaned.held.is_empty(), "{:?}", cleaned.held);
    assert_eq!(cleaned.jobs.len(), 1);
    assert!(!branches(repo.path()).contains(&branch));
    assert_eq!(state(repo.path()), SlotState::Free);
    assert!(slot.exists(), "a slot is released, never removed");
}

#[test]
fn force_releases_a_kept_slot_once_what_kept_it_is_gone() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, branch) = a_job_in_a_slot(machine.path(), repo.path());
    std::fs::write(slot.join("notes.md"), "never committed").unwrap();
    GitVcs
        .release_slot(&slots(repo.path()), 1, JOB)
        .expect_err("a dirty tree is kept");
    std::fs::remove_file(slot.join("notes.md")).unwrap();

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Delete);

    assert!(cleaned.held.is_empty(), "{:?}", cleaned.held);
    assert!(!branches(repo.path()).contains(&branch));
    assert_eq!(state(repo.path()), SlotState::Free);
}

/// `--force` deletes an unmerged branch elsewhere, but a slot is released by
/// the pool's rules, and those keep work that is on neither remote nor base.
#[test]
fn force_refuses_a_slot_holding_unlanded_commits_and_says_so() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, branch) = a_job_in_a_slot(machine.path(), repo.path());
    a_commit(&slot, &branch);
    completed(repo.path());

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Delete);

    let Holding::Refused(why) = &cleaned.held[0].why else {
        panic!("{:?}", cleaned.held);
    };
    assert!(why.contains("1 commits"), "{why}");
    assert!(branches(repo.path()).contains(&branch));
    assert!(held_by_the_job(&state(repo.path())));
}

#[test]
fn force_refuses_a_slot_with_uncommitted_files_and_names_one() {
    let repo = a_repository();
    let machine = TempDir::new();
    let (slot, branch) = a_job_in_a_slot(machine.path(), repo.path());
    completed(repo.path());
    std::fs::write(slot.join("scratch.rs"), "half done").unwrap();

    let cleaned = cleaned(repo.path(), machine.path(), UnmergedWork::Delete);

    let Holding::Refused(why) = &cleaned.held[0].why else {
        panic!("{:?}", cleaned.held);
    };
    assert!(why.contains("scratch.rs"), "{why}");
    assert!(slot.join("scratch.rs").exists());
    assert!(branches(repo.path()).contains(&branch));
}

/// The Job has not ended, so its slot is its work in progress. Even a clean
/// tree level with `main` is left exactly as it is.
#[test]
fn neither_form_touches_a_slot_whose_job_has_not_ended() {
    for unmerged in [UnmergedWork::Keep, UnmergedWork::Delete] {
        let repo = a_repository();
        let machine = TempDir::new();
        let (_, branch) = a_job_in_a_slot(machine.path(), repo.path());

        let cleaned = cleaned(repo.path(), machine.path(), unmerged);

        assert_eq!(cleaned.held[0].why, Holding::Live, "{unmerged:?}");
        assert!(cleaned.jobs.is_empty(), "{unmerged:?}");
        assert!(branches(repo.path()).contains(&branch), "{unmerged:?}");
        assert!(held_by_the_job(&state(repo.path())), "{unmerged:?}");
    }
}
