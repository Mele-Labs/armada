//! A kept reading of the pool reads the slots that were touched, and the rest
//! from memory: against real git and the real file system, since what is
//! under test is which events arrive.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use adapter_traits::{SlotHeld, SlotLeased, SlotPool, Vcs, WorktreeSpec};

use crate::leasing::watching::{events_heard, heard_by, heard_of, last_reread, last_status_asked, unwatched};
use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;
use crate::GitVcs;

/// A pool of three: slot 1 held by a Job, slots 2 and 3 free and made.
fn a_pool_with_two_free_slots() -> (TempRepo, SlotPool) {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\nnode_modules/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    let pool = SlotPool::of(&repo.root().to_string_lossy(), 3, "main", Vec::new());
    for (handle, job) in [("1-a", "01A"), ("2-b", "01B"), ("3-c", "01C")] {
        let spec = WorktreeSpec::for_job(&repo.root().to_string_lossy(), handle).expect("a spec");
        assert!(matches!(
            GitVcs::new().lease_slot(&pool, &spec, job),
            Ok(SlotLeased::Took { .. })
        ));
    }
    GitVcs::new().release_slot(&pool, 2, "01B").expect("released");
    GitVcs::new().release_slot(&pool, 3, "01C").expect("released");
    (repo, pool)
}

fn slot(repo: &TempRepo, number: usize) -> PathBuf {
    repo.root().join(format!(".armada/slots/slot-{number}"))
}

fn write(at: &Path, relative: &str) {
    let path = at.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
    std::fs::write(path, "x\n").expect("a file");
}

/// How long no event must arrive before the file system counts as quiet.
const QUIET: Duration = Duration::from_millis(300);

/// Waits until `root`'s pool has been delivered no event for `QUIET`, and, with
/// `heard`, until something has been heard. One write is several events (a
/// create, then its modify and close), and a read taken between them leaves the
/// last for the next read to ask git about, which a loaded runner showed
/// (main red at bb9dd85). Ten seconds at most.
fn quiet(root: &Path, heard: bool) {
    let until = Instant::now() + Duration::from_secs(10);
    let mut seen = events_heard(root);
    let mut since = Instant::now();
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
        let now = events_heard(root);
        if now != seen {
            (seen, since) = (now, Instant::now());
            continue;
        }
        let some = heard_of(root);
        if since.elapsed() >= QUIET && (!heard || some.0 || !some.1.is_empty()) {
            return;
        }
    }
}

/// What has been heard, once something has and the file system is quiet.
fn heard_soon(root: &Path) -> (bool, Vec<usize>) {
    quiet(root, true);
    heard_of(root)
}

/// The file system quiet after a read, so what the read stirred is in before the next.
fn settled(root: &Path) {
    quiet(root, false);
}

/// A read, a pause for whatever the read itself stirred, and one more.
fn kept_and_quiet(repo: &TempRepo, pool: &SlotPool) {
    GitVcs::new().slot_pool_kept(pool, true);
    settled(repo.root());
    GitVcs::new().slot_pool_kept(pool, false);
    assert_eq!(last_reread(repo.root()), Vec::<usize>::new(), "reading moved something");
}

#[test]
fn nothing_touched_reads_nothing_and_reading_touches_nothing() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);
    assert_eq!(heard_of(repo.root()), (false, Vec::new()));
}

#[test]
fn an_edit_in_a_free_slot_reads_that_slot_alone() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 2), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![2]));

    let read = GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![2]);
    assert!(matches!(read[1].held, SlotHeld::Stranded(_)), "{:?}", read[1].held);
    assert_eq!(read[2].held, SlotHeld::Free);
}

#[test]
fn a_build_and_an_install_in_a_free_slot_are_not_an_edit() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 2), "target/debug/deps/a.rlib");
    write(&slot(&repo, 2), "node_modules/y/index.js");
    // The control: a real edit elsewhere, heard after both.
    write(&slot(&repo, 3), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![3]));
    settled(repo.root());
    assert_eq!(heard_of(repo.root()), (false, vec![3]));
}

#[test]
fn an_edit_in_a_held_slot_is_not_heard() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 1), "notes.txt");
    write(&slot(&repo, 3), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![3]));
    settled(repo.root());
    assert_eq!(heard_of(repo.root()), (false, vec![3]));
}

#[test]
fn a_commit_in_a_held_slot_moves_that_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    repo.git(&[
        "-C",
        &slot(&repo, 1).to_string_lossy(),
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.com",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "work",
    ]);
    assert_eq!(heard_soon(repo.root()), (false, vec![1]));
    settled(repo.root());
    assert_eq!(heard_of(repo.root()), (false, vec![1]), "the branch moving is not the base moving");
    GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1]);
}

#[test]
fn the_base_moving_reads_every_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    repo.commit_one("one.txt", "one\n", "the base moves");
    repo.git(&["push", "-q", "origin", "main"]);
    assert!(heard_soon(repo.root()).0);
    let read = GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
    assert_eq!(read[0].behind, Some(1));
}

#[test]
fn a_full_read_goes_over_every_slot_but_asks_git_for_status_of_none_that_was_quiet() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
}

#[test]
fn a_free_slot_read_for_the_first_time_asks_git() {
    let (repo, pool) = a_pool_with_two_free_slots();

    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
    settled(repo.root());
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
}

#[test]
fn an_edit_is_found_by_the_next_full_read_which_asks_git_of_that_slot_alone() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    write(&slot(&repo, 2), "notes.txt");
    assert_eq!(heard_soon(repo.root()), (false, vec![2]));
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2]);
    assert!(matches!(read[1].held, SlotHeld::Stranded(_)), "{:?}", read[1].held);

    // Trusted again, and still what it was.
    settled(repo.root());
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
    assert!(matches!(read[1].held, SlotHeld::Stranded(_)), "{:?}", read[1].held);

    // And the edit undone is heard as the edit was.
    std::fs::remove_file(slot(&repo, 2).join("notes.txt")).expect("gone");
    assert_eq!(heard_soon(repo.root()), (false, vec![2]));
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(read[1].held, SlotHeld::Free);
}

#[test]
fn a_watch_that_reported_an_error_asks_git_of_every_free_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    heard_by(repo.root(), Err(notify::Error::generic("the stream broke")));
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
}

#[test]
fn a_rescan_flag_asks_git_of_every_free_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    // What notify says when the kernel dropped events and a directory must be
    // looked at again.
    let overflow = notify::Event::new(notify::EventKind::Other).set_flag(notify::event::Flag::Rescan);
    heard_by(repo.root(), Ok(overflow));
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
}

#[test]
fn a_pool_the_file_system_would_not_be_watched_for_asks_git_every_time() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    unwatched(repo.root());
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
}

#[test]
fn what_this_adapter_did_asks_git_of_every_free_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    GitVcs::new().release_slot(&pool, 1, "01A").expect("released");
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![1, 2, 3]);
}

/// Slot 1 held by a process that is running, slots 2 and 3 free.
fn a_pool_with_a_process_in_slot_one() -> (TempRepo, SlotPool, std::process::Child) {
    let (repo, wire) = a_pool_with_two_free_slots();
    GitVcs::new().release_slot(&wire, 1, "01A").expect("released");
    let child = std::process::Command::new("sleep").arg("120").spawn().expect("sleep");
    let holder = Holder::of(child.id()).expect("the child is running");
    let pool = Pool::at(repo.root(), 3, "main", Vec::new());
    let seed = |_: &Path, _: &Path| Err(String::from("nothing to seed from"));
    assert!(matches!(
        pool.try_lease("feature/held", &holder, 0, &seed),
        Ok(Leased::Took(_))
    ));
    (repo, wire, child)
}

#[test]
fn a_stream_restarted_by_a_slot_coming_free_asks_git_of_every_free_slot_once() {
    let (repo, pool, mut child) = a_pool_with_a_process_in_slot_one();
    kept_and_quiet(&repo, &pool);
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());

    // Nothing here is an act of this adapter: the holder is gone, and the
    // read that finds slot 1 free adds it to the stream, which restarts, and
    // an edit in that moment is one the stream never reports.
    child.kill().expect("killed");
    child.wait().expect("reaped");
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(read[0].held, SlotHeld::Free);
    assert_eq!(last_status_asked(repo.root()), vec![1, 2, 3]);

    settled(repo.root());
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
}

#[test]
fn a_stream_restarted_by_a_slot_being_taken_asks_git_of_every_free_slot_at_the_next_read() {
    let (repo, pool, mut child) = a_pool_with_a_process_in_slot_one();
    child.kill().expect("killed");
    child.wait().expect("reaped");
    kept_and_quiet(&repo, &pool);

    // A second holder takes slot 1 behind this adapter's back, as an agent's
    // `armada worktree lease` does: the next read lets its work tree go.
    let other = std::process::Command::new("sleep").arg("120").spawn();
    let mut other = other.expect("sleep");
    let holder = Holder::of(other.id()).expect("the process is running");
    let direct = Pool::at(repo.root(), 3, "main", Vec::new());
    let seed = |_: &Path, _: &Path| Err(String::from("nothing to seed from"));
    assert!(matches!(
        direct.try_lease("feature/again", &holder, 0, &seed),
        Ok(Leased::Took(_))
    ));
    settled(repo.root());
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert!(matches!(read[0].held, SlotHeld::Session(_)), "{:?}", read[0].held);
    // Slots 2 and 3 were read before the stream let slot 1 go, so this read
    // trusts them and the next does not.
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
    settled(repo.root());
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
    settled(repo.root());
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), Vec::<usize>::new());
    other.kill().expect("killed");
    other.wait().expect("reaped");
}

#[test]
fn an_edit_under_a_directory_the_watch_skips_is_asked_about_where_git_counts_it() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    // `target` is ignored by the repository, and `.gitnexus` is not.
    write(&slot(&repo, 2), "target/debug/deps/a.rlib");
    write(&slot(&repo, 3), ".gitnexus/index.db");
    settled(repo.root());
    let read = GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![3]);
    assert_eq!(read[1].held, SlotHeld::Free);
    assert!(matches!(read[2].held, SlotHeld::Stranded(_)), "{:?}", read[2].held);
}

#[test]
fn an_edit_to_the_repositorys_exclude_file_asks_git_of_every_free_slot() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    std::fs::create_dir_all(repo.root().join(".git/info")).expect("a directory");
    std::fs::write(repo.root().join(".git/info/exclude"), "notes.txt\n").expect("written");
    assert!(heard_soon(repo.root()).0);
    GitVcs::new().slot_pool_kept(&pool, true);
    assert_eq!(last_status_asked(repo.root()), vec![2, 3]);
}

#[test]
fn what_this_adapter_did_is_read_again_without_waiting_for_the_file_system() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    GitVcs::new().release_slot(&pool, 1, "01A").expect("released");
    GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(last_reread(repo.root()), vec![1, 2, 3]);
}

fn lease_record(repo: &TempRepo, number: usize) -> PathBuf {
    repo.root().join(format!(".armada/slots/slot-{number}.lease"))
}

#[test]
fn a_lease_record_written_after_the_slot_moved_is_heard_and_read() {
    let (repo, pool) = a_pool_with_two_free_slots();
    kept_and_quiet(&repo, &pool);

    // A lease moves the slot's branch first and writes the record last, and the
    // read the branch move calls for finds a slot nobody holds.
    repo.git(&["-C", &slot(&repo, 2).to_string_lossy(), "switch", "-q", "-c", "armada/session-x"]);
    assert_eq!(heard_soon(repo.root()), (false, vec![2]));
    let between = GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(between[1].held, SlotHeld::Free);

    settled(repo.root());
    std::fs::write(lease_record(&repo, 2), "branch armada/session-x\nholder job 01X\nsince 1\n")
        .expect("a record");
    assert_eq!(heard_soon(repo.root()), (false, vec![2]), "the record is a change to the slot");
    let read = GitVcs::new().slot_pool_kept(&pool, false);
    assert_eq!(read[1].held, SlotHeld::Hosted(String::from("01X")));
}
