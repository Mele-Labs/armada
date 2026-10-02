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
