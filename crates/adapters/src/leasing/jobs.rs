//! A Job's lease, as `Vcs` asks for it: the pool, held by the Job's id.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use adapter_traits::{
    SlotKept, SlotLeased, SlotPool, SlotReading, SlotStanding, Worktree, WorktreeSpec,
};

use super::{Holder, LeaseRefused, Leased, Pool, SlotState};

fn pool(slots: &SlotPool) -> Pool {
    Pool::at(
        Path::new(slots.repo_root()),
        slots.slots() as usize,
        slots.base(),
        slots.keep().to_vec(),
    )
}

/// Fleet clones a new slot's build itself, because it knows when a warm-up is
/// under way and writes what the run sheet reads.
fn not_here(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("Fleet seeds a Job's slot"))
}

pub(crate) fn lease(
    slots: &SlotPool,
    spec: &WorktreeSpec,
    job: &str,
) -> Result<SlotLeased, LeaseRefused> {
    // Read here, at the edge: only `--status` reads it, to say for how long.
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    let branch = spec.branch();
    match pool(slots).try_lease(&branch, &Holder::job(job), since, &not_here)? {
        Leased::Full(_) => Ok(SlotLeased::Full),
        Leased::Took(lease) => Ok(SlotLeased::Took {
            slot: lease.slot() as u32,
            worktree: Worktree::at(lease.path().to_string_lossy(), branch),
            reused: !lease.made(),
        }),
    }
}

pub(crate) fn open(slots: &SlotPool, job: &str) -> bool {
    pool(slots).open_for(&Holder::job(job))
}

pub(crate) fn standing(slots: &SlotPool, slot: u32, job: &str) -> SlotStanding {
    match pool(slots).state(slot as usize) {
        SlotState::Unmade | SlotState::NotACheckout => SlotStanding::Gone,
        SlotState::Held { holder, .. } if holder == Holder::job(job) => SlotStanding::Held,
        SlotState::Held { holder, .. } => SlotStanding::HeldBy(holder.said()),
        SlotState::Free | SlotState::Abandoned { .. } => SlotStanding::Free,
        SlotState::Busy => SlotStanding::HeldBy(String::from("a lease under way")),
        SlotState::Stranded { branch, why, .. } => SlotStanding::HeldBy(format!(
            "nobody: {branch}'s holder is gone and it holds {why}"
        )),
    }
}

pub(crate) fn release(slots: &SlotPool, slot: u32, job: &str) -> Result<(), SlotKept> {
    pool(slots)
        .release_held(slot as usize, &Holder::job(job))
        .map(|_| ())
        .map_err(|refused| SlotKept(refused.said()))
}

pub(crate) fn completed(slots: &SlotPool, slot: u32, job: &str) {
    let _ = pool(slots).mark_completed(slot as usize, &Holder::job(job));
}

pub(crate) fn readings(slots: &SlotPool) -> Vec<SlotReading> {
    pool(slots).readings()
}
