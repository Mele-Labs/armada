//! A slot leased for a Job: held by the Job's id, never taken back for a dead
//! process, and given back only by the pool's rules.

use std::path::{Path, PathBuf};

use adapter_traits::{SlotHeld, SlotLeased, SlotPool, SlotStanding, Vcs, WorktreeSpec};

use crate::leasing::{Holder, Leased, Pool, SlotState};
use crate::tests::repo::TempRepo;
use crate::GitVcs;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn branch_of(at: &Path) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["branch", "--show-current"])
        .output()
        .expect("git on PATH");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn slots(repo: &TempRepo, count: u32) -> SlotPool {
    SlotPool::of(&repo.root().to_string_lossy(), count, "main", Vec::new())
}

fn spec(repo: &TempRepo, handle: &str) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root().to_string_lossy(), handle).expect("a legal spec")
}

fn leased(repo: &TempRepo, pool: &SlotPool, handle: &str, job: &str) -> (u32, PathBuf) {
    match GitVcs.lease_slot(pool, &spec(repo, handle), job) {
        Ok(SlotLeased::Took { slot, worktree, .. }) => (slot, PathBuf::from(worktree.path())),
        other => panic!("{job} took no slot: {other:?}"),
    }
}

#[test]
fn a_job_leases_a_slot_on_its_own_branch_and_holds_it_by_its_id() {
    let repo = a_repository();
    let pool = slots(&repo, 2);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");

    assert_eq!(path, repo.root().join(".armada/slots/slot-1"));
    assert_eq!(slot, 1);
    assert_eq!(branch_of(&path), "armada/1-a-job");
    assert_eq!(GitVcs.slot_standing(&pool, 1, "01JOB"), SlotStanding::Held);
    assert!(matches!(
        GitVcs.slot_standing(&pool, 1, "01OTHER"),
        SlotStanding::HeldBy(_)
    ));
}

/// Fleet restarting is a new pid; the lease names no pid at all.
#[test]
fn a_job_s_lease_reads_as_held_whatever_process_asks() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    leased(&repo, &pool, "1-a-job", "01JOB");

    let status = Pool::at(repo.root(), 1, "main", Vec::new()).status();
    match &status[0].state {
        SlotState::Held { holder, .. } => assert_eq!(holder, &Holder::hosted("01JOB")),
        other => panic!("a Job's slot read as {other:?}"),
    }
    let next = Pool::at(repo.root(), 1, "main", Vec::new()).try_lease(
        "someone-else",
        &Holder::of(std::process::id()).unwrap(),
        0,
        &no_seed,
    );
    assert!(
        matches!(next, Ok(Leased::Full(_))),
        "a clean Job's slot was taken back: {next:?}"
    );
}

#[test]
fn a_job_that_already_holds_a_slot_is_handed_the_same_one_untouched() {
    let repo = a_repository();
    let pool = slots(&repo, 2);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    std::fs::write(path.join("half-done.rs"), "the Drone's work").unwrap();

    let (again, at) = leased(&repo, &pool, "1-a-job", "01JOB");
    assert_eq!((again, at.clone()), (slot, path));
    assert!(
        at.join("half-done.rs").exists(),
        "the second lease reset it"
    );
}

#[test]
fn a_release_refused_keeps_the_slot_held() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    std::fs::write(path.join("notes.md"), "never committed").unwrap();

    let kept = GitVcs
        .release_slot(&pool, slot, "01JOB")
        .expect_err("a dirty tree is not given back");
    assert!(kept.0.contains("notes.md"), "{}", kept.0);
    assert!(matches!(
        Pool::at(repo.root(), 1, "main", Vec::new()).status()[0].state,
        SlotState::Held { .. }
    ));

    std::fs::remove_file(path.join("notes.md")).unwrap();
    GitVcs
        .release_slot(&pool, slot, "01JOB")
        .expect("clean, with nothing on it");
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Free
    );
}

#[test]
fn a_job_cannot_give_back_a_slot_another_holds() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "1-a-job", "01JOB");

    assert!(GitVcs.release_slot(&pool, slot, "01OTHER").is_err());
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Held
    );
}

#[test]
fn a_slot_nobody_made_is_gone() {
    let repo = a_repository();
    assert_eq!(
        GitVcs.slot_standing(&slots(&repo, 2), 2, "01JOB"),
        SlotStanding::Gone
    );
}

/// What Bridge's Cleanup draws: the Job's slot on its branch, warm once its
/// seed path is there, and behind once the base moves past it.
#[test]
fn the_pool_reads_each_slot_with_its_holder_warmth_and_lag() {
    let repo = a_repository();
    let root = repo.root().to_string_lossy().to_string();
    let pool = SlotPool::of(&root, 2, "main", vec![String::from("target")]);
    let (_, path) = leased(&repo, &pool, "1-a-job", "01JOB");

    let cold = GitVcs.slot_pool(&pool);
    assert_eq!(cold.len(), 2);
    assert_eq!(cold[0].held, SlotHeld::Hosted(String::from("01JOB")));
    assert_eq!(cold[0].branch.as_deref(), Some("armada/1-a-job"));
    assert!(cold[0].since.is_some(), "the lease records when");
    assert!(!cold[0].warm, "no build yet");
    assert_eq!(cold[0].behind, Some(0));
    assert_eq!(cold[1].held, SlotHeld::Unmade);
    assert_eq!((cold[1].warm, cold[1].behind), (false, None));

    std::fs::create_dir_all(path.join("target/debug")).expect("a build directory");
    repo.commit_one("one.txt", "one\n", "the base moves");
    repo.commit_one("two.txt", "two\n", "and again");
    repo.git(&["push", "-q", "origin", "main"]);

    let warm = GitVcs.slot_pool(&pool);
    assert!(warm[0].warm);
    assert_eq!(warm[0].behind, Some(2));
}

/// The slots are read side by side and still answer in the pool's order, each
/// with its own holder, however many there are past the fan-out's width.
#[test]
fn a_wide_pool_reads_in_slot_order_with_each_slots_own_holder() {
    let repo = a_repository();
    let pool = slots(&repo, 10);
    leased(&repo, &pool, "1-a-job", "01A");
    leased(&repo, &pool, "2-b-job", "01B");
    leased(&repo, &pool, "3-c-job", "01C");

    let read = GitVcs.slot_pool(&pool);
    let numbers: Vec<u32> = read.iter().map(|one| one.slot).collect();
    assert_eq!(numbers, (1..=10).collect::<Vec<u32>>());
    let holders: Vec<&SlotHeld> = read.iter().take(4).map(|one| &one.held).collect();
    assert_eq!(
        holders,
        [
            &SlotHeld::Hosted(String::from("01A")),
            &SlotHeld::Hosted(String::from("01B")),
            &SlotHeld::Hosted(String::from("01C")),
            &SlotHeld::Unmade,
        ]
    );
    assert!(read[3..].iter().all(|one| one.held == SlotHeld::Unmade));
}

/// A hosted session holds its slot under its own id as a Job would, so the pool
/// would call its work a Job's.
#[test]
fn a_hosted_sessions_wip_commit_names_a_session_and_not_a_job() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-7b1f3c52", "7b1f3c52-9a40");
    std::fs::write(path.join("wip.txt"), "half a thought\n").expect("a file to keep");

    let parked = GitVcs
        .park_hosted_slot(&pool, slot, "7b1f3c52-9a40")
        .expect("it parks");
    let subject = repo.git(&["log", "-1", "--format=%s", &parked.branch]);
    assert!(
        subject.contains("job 7b1f3c52-9a40"),
        "{subject}"
    );
}

#[test]
fn a_park_refused_by_the_pool_says_so_and_keeps_the_slot() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "1-a-job", "01JOB");

    let refused = GitVcs
        .park_slot(&pool, slot, "01SOMEBODY")
        .expect_err("not theirs");
    assert!(matches!(
        refused,
        adapter_traits::SlotParkRefused::HeldByAnother(_)
    ));
    assert_eq!(
        GitVcs.slot_standing(&pool, slot, "01JOB"),
        SlotStanding::Held
    );
}

/// A slot an agent session holds is released with its uncommitted files
/// committed to its branch, and only for the holder the person was shown.
#[test]
fn a_session_held_slot_is_released_with_its_files_committed_for_the_holder_shown() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let here = Holder::of(std::process::id()).expect("this process is running");
    let path = match pool.try_lease("fleet/by-hand", &here, 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot: {other:?}"),
    };
    std::fs::write(path.join("half.txt"), "half a thought\n").expect("a file");
    let wire = slots(&repo, 1);

    // Read, so the confirm can list the files; no rescue act reaches it.
    assert_eq!(
        GitVcs.session_work(&wire, 1).expect("read").uncommitted,
        vec!["half.txt"]
    );
    assert!(GitVcs.stranded_work(&wire, 1).is_err());

    let changed = GitVcs
        .release_session_slot(&wire, 1, "somebody (pid 1)")
        .expect_err("another holder was shown");
    assert!(matches!(
        changed,
        adapter_traits::SlotParkRefused::HolderChanged(_)
    ));
    assert!(path.join("half.txt").exists());

    let parked = GitVcs
        .release_session_slot(&wire, 1, &here.said())
        .expect("the holder shown still holds it");
    assert_eq!(parked.branch, "fleet/by-hand");
    assert_eq!(parked.files, vec!["half.txt"]);
    assert!(matches!(pool.state(1), SlotState::Free));
    assert_eq!(
        repo.git(&["show", "fleet/by-hand:half.txt"]).trim(),
        "half a thought"
    );

    let free = GitVcs
        .release_session_slot(&wire, 1, &here.said())
        .expect_err("nobody holds it now");
    assert!(matches!(
        free,
        adapter_traits::SlotParkRefused::HolderChanged(_)
    ));
}

/// A slot a terminal session stood in is parked for it **only once the process
/// that leased it has ended**: while it runs the park is refused and nothing is
/// committed, and after it the files go to the slot's branch.
#[test]
fn a_slot_left_by_an_ended_process_is_parked_and_not_while_it_runs() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let mut claude = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("sleep on PATH");
    let holder = Holder::of(claude.id()).expect("the child is running");
    let path = match pool.try_lease("fleet/by-terminal", &holder, 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot: {other:?}"),
    };
    std::fs::write(path.join("half.txt"), "half a thought\n").expect("a file");
    let wire = slots(&repo, 1);

    let running = GitVcs
        .park_ended_session_slot(&wire, 1, "S1")
        .expect_err("its process still runs");
    assert!(matches!(
        running,
        adapter_traits::SlotParkRefused::HeldByAnother(_)
    ));
    assert!(!repo
        .git(&["ls-tree", "--name-only", "fleet/by-terminal"])
        .contains("half.txt"));

    claude.kill().expect("killed");
    claude.wait().expect("reaped");
    let parked = GitVcs
        .park_ended_session_slot(&wire, 1, "S1")
        .expect("its process ended");
    assert_eq!(parked.branch, "fleet/by-terminal");
    assert_eq!(parked.files, vec!["half.txt"]);
    assert!(matches!(pool.state(1), SlotState::Free));
    assert_eq!(
        repo.git(&["show", "fleet/by-terminal:half.txt"]).trim(),
        "half a thought"
    );
    assert!(repo.git(&["log", "-1", "--format=%s", "fleet/by-terminal"]).contains("job S1"));
}

/// A repair's branch goes once its fix is placed or the repair failed, and
/// only after the slot that wrote it has been given back.
#[test]
fn a_repair_branch_is_deleted_once_its_slot_is_parked_and_not_before() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, _) = leased(&repo, &pool, "repair-1-a-job-3", "01JOB-repair-3");
    let repair = spec(&repo, "repair-1-a-job-3");

    let refused = GitVcs
        .delete_repair_branch(&repair)
        .expect_err("checked out");
    assert!(!refused.0.is_empty(), "{refused:?}");
    assert!(branch_exists(&repo, &repair.branch()));

    GitVcs
        .park_slot(&pool, slot, "01JOB-repair-3")
        .expect("parked");
    assert_eq!(GitVcs.delete_repair_branch(&repair), Ok(true));
    assert!(
        !branch_exists(&repo, &repair.branch()),
        "the branch is deleted"
    );
    assert_eq!(
        GitVcs.delete_repair_branch(&repair),
        Ok(false),
        "already gone is not a failure"
    );
}

fn branch_exists(repo: &TempRepo, branch: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo.root())
        .args(["rev-parse", "--verify", "--quiet", branch])
        .status()
        .expect("git on PATH")
        .success()
}
