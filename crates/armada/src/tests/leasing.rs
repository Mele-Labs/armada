//! What `armada worktree --status` says of a slot a Job holds.

use std::path::PathBuf;

use adapters::leasing::{Holder, Slot, SlotState};

use crate::leasing::line;

fn held_by_a_job(kept: Option<&str>, completed: bool) -> Slot {
    Slot {
        number: 2,
        path: PathBuf::from("/repo/.armada/slots/slot-2"),
        state: SlotState::Held {
            branch: String::from("armada/1-fix"),
            holder: Holder::job("01JOB"),
            since: 0,
            kept: kept.map(str::to_string),
            completed,
        },
        closed: false,
    }
}

#[test]
fn a_slot_a_completed_job_holds_reads_as_done_until_it_is_cleared() {
    let said = line(&held_by_a_job(None, true), 600);
    assert!(said.starts_with("slot-2  done "), "{said}");
    assert!(said.contains("by job 01JOB"), "{said}");
    assert!(said.contains("until the Job is cleared"), "{said}");
}

#[test]
fn a_refused_release_reads_as_kept_whether_or_not_the_job_completed() {
    let said = line(&held_by_a_job(Some("2 uncommitted"), true), 600);
    assert!(said.starts_with("slot-2  kept "), "{said}");
}

#[test]
fn a_running_job_s_slot_reads_as_held() {
    let said = line(&held_by_a_job(None, false), 600);
    assert!(said.starts_with("slot-2  held "), "{said}");
}

#[test]
fn a_closed_slot_says_so_beside_its_holder() {
    let slot = Slot {
        closed: true,
        ..held_by_a_job(None, false)
    };
    let said = line(&slot, 600);
    assert!(said.starts_with("slot-2 closed  held "), "{said}");
    assert!(said.contains("by job 01JOB"), "{said}");
}

/// `armada worktree lease` reads the pool fresh each time, the Manifest's
/// size and then this machine's own, so a slot closed or added from Bridge
/// is what the next lease finds.
#[test]
fn the_clis_lease_honours_a_closed_slot_and_this_machines_size() {
    use super::clean::a_repository;

    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();
    let pool = || crate::leasing::pool_of(repo.path()).expect("a pool");
    assert_eq!(pool().bays(), vec![1]);

    pool().close(1).expect("closed");
    let none = |_: &std::path::Path, _: &std::path::Path| Err(String::from("no seed"));
    let here = Holder::of(std::process::id()).expect("running");
    assert!(
        matches!(
            pool().try_lease("one", &here, 0, &none),
            Ok(adapters::leasing::Leased::Full(_))
        ),
        "a closed slot is not leased"
    );

    assert_eq!(pool().add().expect("added"), 2);
    assert_eq!(crate::leasing::lease(repo.path(), "one"), 0);
    assert!(pool().path_of(2).is_dir(), "the added slot was made");
    assert!(!pool().path_of(1).exists(), "the closed one was not");
}
