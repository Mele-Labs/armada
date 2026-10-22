//! Parking the slot of a session that has ended: the branch it is on is the one
//! its work goes to, and what it left running in the slot goes with it.
//! Real repositories and real processes.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use adapter_traits::{SlotLeased, SlotParkRefused, SlotPool, Vcs, WorktreeSpec};

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

fn slots(repo: &TempRepo, count: u32) -> SlotPool {
    SlotPool::of(&repo.root().to_string_lossy(), count, "main", Vec::new())
}

fn leased(repo: &TempRepo, pool: &SlotPool, handle: &str, session: &str) -> (u32, PathBuf) {
    let spec = WorktreeSpec::for_job(&repo.root().to_string_lossy(), handle).expect("a spec");
    match GitVcs::new().lease_slot(pool, &spec, session) {
        Ok(SlotLeased::Took { slot, worktree, .. }) => (slot, PathBuf::from(worktree.path())),
        other => panic!("{session} took no slot: {other:?}"),
    }
}

fn git(at: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&run.stderr));
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// A process that does nothing for a while, working in `at`.
fn idling_in(at: &Path) -> Child {
    Command::new("sleep")
        .arg("60")
        .current_dir(at)
        .stdout(Stdio::null())
        .spawn()
        .expect("sleep on PATH")
}

/// Whether `child` is still running; a killed child nobody has waited for is a
/// zombie, which is not.
fn running(child: &mut Child) -> bool {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait().expect("asked") {
            Some(_) => return false,
            None if Instant::now() > until => return true,
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[test]
fn a_session_that_switched_branch_in_its_slot_parks_on_the_branch_it_is_on() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-47e2d39c", "47e2d39c");
    // What the session may do inside its slot: put its work on a branch of its own.
    git(&path, &["switch", "--quiet", "-c", "sessions/mine"]);
    std::fs::write(path.join("kept.txt"), "committed there\n").unwrap();
    git(&path, &["add", "kept.txt"]);
    git(&path, &["commit", "--quiet", "-m", "work on the new branch"]);
    std::fs::write(path.join("wip.txt"), "half a thought\n").unwrap();

    let parked = GitVcs::new()
        .park_hosted_slot(&pool, slot, "47e2d39c")
        .expect("the branch it is on is where it parks");

    assert_eq!(parked.branch, "sessions/mine");
    assert!(matches!(
        Pool::at(repo.root(), 1, "main", Vec::new()).state(1),
        SlotState::Free
    ));
    assert_eq!(repo.git(&["show", "sessions/mine:wip.txt"]).trim(), "half a thought");
    assert_eq!(repo.git(&["show", "sessions/mine:kept.txt"]).trim(), "committed there");
}

#[test]
fn a_job_that_switched_branch_parks_on_the_branch_it_is_on() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "1-a-job", "01JOB");
    git(&path, &["switch", "--quiet", "-c", "elsewhere"]);
    std::fs::write(path.join("wip.txt"), "x").unwrap();

    let parked = GitVcs::new().park_slot(&pool, slot, "01JOB").expect("it parks where it is");

    assert_eq!(parked.branch, "elsewhere");
    assert_eq!(repo.git(&["show", "elsewhere:wip.txt"]).trim(), "x");
}

#[test]
fn a_session_on_the_base_keeps_its_files_on_a_new_branch() {
    let repo = a_repository();
    repo.git(&["checkout", "-q", "-b", "elsewhere"]);
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-7", "S7");
    std::fs::write(path.join("wip.txt"), "x").unwrap();
    git(&path, &["switch", "--quiet", "main"]);

    let parked = GitVcs::new().park_hosted_slot(&pool, slot, "S7").expect("it gets a branch");

    assert_eq!(parked.branch, "kept/slot-1-S7");
    assert_eq!(repo.git(&["show", "kept/slot-1-S7:wip.txt"]).trim(), "x");
}

#[test]
fn a_detached_session_keeps_its_files_on_a_new_branch() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-7", "S7");
    std::fs::write(path.join("wip.txt"), "x").unwrap();
    git(&path, &["switch", "--quiet", "--detach"]);

    let parked = GitVcs::new().park_hosted_slot(&pool, slot, "S7").expect("it gets a branch");

    assert_eq!(parked.branch, "kept/slot-1-S7");
    assert_eq!(repo.git(&["show", "kept/slot-1-S7:wip.txt"]).trim(), "x");
}

#[test]
fn parking_a_sessions_slot_ends_what_still_runs_in_it_and_nothing_outside_it() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-9", "S9");
    // A backgrounded dev server, and a Check's process, in the slot and in a directory of it.
    std::fs::create_dir_all(path.join("apps/desktop")).unwrap();
    let mut mock = idling_in(&path);
    let mut check = idling_in(&path.join("apps/desktop"));
    // Beside it: the repository's own checkout, and a directory whose name merely begins with
    // the slot's.
    let sibling = repo.root().join(".armada/slots/slot-10");
    std::fs::create_dir_all(&sibling).unwrap();
    let mut elsewhere = idling_in(repo.root());
    let mut beside = idling_in(&sibling);

    GitVcs::new().park_hosted_slot(&pool, slot, "S9").expect("parked");

    let (mock_ran, check_ran) = (running(&mut mock), running(&mut check));
    let (elsewhere_ran, beside_ran) = (running(&mut elsewhere), running(&mut beside));
    let _ = elsewhere.kill();
    let _ = beside.kill();
    let _ = mock.kill();
    let _ = check.kill();
    assert!(!mock_ran, "a server left running in the slot is ended");
    assert!(!check_ran, "a Check's process in a directory of the slot is ended");
    assert!(elsewhere_ran, "a process in the repository's checkout is left");
    assert!(beside_ran, "slot-10 is not slot-1");
}

#[test]
fn a_refused_park_ends_nothing_that_was_running() {
    let repo = a_repository();
    let pool = slots(&repo, 1);
    let (slot, path) = leased(&repo, &pool, "session-8", "S8");
    let mut mock = idling_in(&path);

    assert!(GitVcs::new().park_hosted_slot(&pool, slot, "NOT-S8").is_err());

    let ran = running(&mut mock);
    let _ = mock.kill();
    assert!(ran, "a slot that stays held keeps what runs in it");
}

#[test]
fn an_ended_terminal_sessions_slot_is_swept_once_its_process_is_gone_and_not_before() {
    let repo = a_repository();
    let local = Pool::at(repo.root(), 1, "main", Vec::new());
    let mut agent = idling_in(repo.root());
    let holder = Holder::of(agent.id()).expect("the child is running");
    let path = match local.try_lease("fleet/by-terminal", &holder, 0, &|_: &Path, _: &Path| {
        Err(String::from("nothing to seed from"))
    }) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot: {other:?}"),
    };
    let mut mock = idling_in(&path);
    let wire = slots(&repo, 1);

    assert!(matches!(
        GitVcs::new().park_ended_session_slot(&wire, 1, "S1"),
        Err(SlotParkRefused::HeldByAnother(_))
    ));
    assert!(running(&mut mock), "the terminal still runs, so the slot is not swept");

    agent.kill().expect("killed");
    agent.wait().expect("reaped");
    GitVcs::new().park_ended_session_slot(&wire, 1, "S1").expect("parked");
    let ran = running(&mut mock);
    let _ = mock.kill();
    assert!(!ran);
}

#[test]
fn lsof_output_is_read_as_a_directory_per_process_and_inside_means_by_components() {
    use crate::leasing::strays::{inside, working_directories};

    let listing = "p12\ncnode\nfcwd\nn/repo/.armada/slots/slot-1/apps\np13\ncsh\nfcwd\nn/repo/.armada/slots/slot-10\n";
    let read = working_directories(listing);
    assert_eq!(read.len(), 2);
    assert_eq!((read[0].0, read[0].1.as_str()), (12, "node"));
    let slot = Path::new("/repo/.armada/slots/slot-1");
    assert!(inside(&read[0].2, slot));
    assert!(inside(slot, slot));
    assert!(!inside(&read[1].2, slot));
}
