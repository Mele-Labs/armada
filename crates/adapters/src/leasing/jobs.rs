//! A Job's lease, as `Vcs` asks for it: the pool, held by the Job's id.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use adapter_traits::{
    RescueRefused, SlotChange, SlotKept, SlotLeased, SlotParkRefused, SlotParked, SlotPool,
    SlotReading, SlotRefused, SlotRescue, SlotRescued, SlotStanding, StrandedWork, Worktree,
    WorktreeSpec,
};

use super::{Holder, LeaseRefused, Leased, ParkRefused, Pool, SlotState, Unshaped};

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
    let since = since();
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

/// Read here, at the edge: only `--status` reads it, to say for how long.
fn since() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// A slot onto a branch that exists, at its tip: the other half of a park.
pub(crate) fn lease_existing(
    slots: &SlotPool,
    branch: &str,
    job: &str,
) -> Result<SlotLeased, LeaseRefused> {
    match pool(slots).try_lease_existing(branch, &Holder::job(job), since(), &not_here)? {
        Leased::Full(_) => Ok(SlotLeased::Full),
        Leased::Took(lease) => Ok(SlotLeased::Took {
            slot: lease.slot() as u32,
            worktree: Worktree::at(lease.path().to_string_lossy(), branch),
            reused: !lease.made(),
        }),
    }
}

/// Commit the Job's work to its branch and give the slot back.
pub(crate) fn park(slots: &SlotPool, slot: u32, job: &str) -> Result<SlotParked, SlotParkRefused> {
    parked_as(slots, slot, &Holder::job(job))
}

/// [`park`] for a slot an agent session holds, only while it is still held by
/// the holder the person was shown.
pub(crate) fn release_session(
    slots: &SlotPool,
    slot: u32,
    expected: &str,
) -> Result<SlotParked, SlotParkRefused> {
    match pool(slots).state(slot as usize) {
        SlotState::Held {
            holder: holder @ Holder::Process { .. },
            ..
        } if holder.said() == expected => parked_as(slots, slot, &holder),
        SlotState::Held { holder, .. } => Err(SlotParkRefused::HolderChanged(holder.said())),
        SlotState::Free | SlotState::Abandoned { .. } => {
            Err(SlotParkRefused::HolderChanged(String::from("nobody")))
        }
        SlotState::Busy => Err(SlotParkRefused::Busy),
        _ => Err(SlotParkRefused::NotLeased),
    }
}

fn parked_as(slots: &SlotPool, slot: u32, holder: &Holder) -> Result<SlotParked, SlotParkRefused> {
    match pool(slots).park(slot as usize, holder) {
        Ok(parked) => {
            let (commit, files) = match parked.committed {
                Some(committed) => (Some(committed.commit), committed.files),
                None => (None, Vec::new()),
            };
            Ok(SlotParked {
                branch: parked.branch,
                commit,
                files,
            })
        }
        Err(ParkRefused::NotASlot(_) | ParkRefused::NotLeased(_)) => {
            Err(SlotParkRefused::NotLeased)
        }
        Err(ParkRefused::HeldByAnother(who)) => Err(SlotParkRefused::HeldByAnother(who)),
        Err(ParkRefused::Busy) => Err(SlotParkRefused::Busy),
        Err(ParkRefused::OnNoBranch) => Err(SlotParkRefused::OnNoBranch),
        Err(ParkRefused::OnTheBase(base)) => Err(SlotParkRefused::OnTheBase(base)),
        Err(ParkRefused::OnAnotherBranch { leased, on }) => {
            Err(SlotParkRefused::OnAnotherBranch { leased, on })
        }
        Err(ParkRefused::Release(refused)) => Err(SlotParkRefused::Release(refused.said())),
        Err(ParkRefused::Vcs(why)) => Err(SlotParkRefused::Vcs(why)),
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

pub(crate) fn change(slots: &SlotPool, change: SlotChange) -> Result<u32, SlotRefused> {
    let pool = pool(slots);
    let changed = match change {
        SlotChange::Add => pool.add(),
        SlotChange::Remove(n) => pool.remove(n as usize).map(|()| n as usize),
        SlotChange::Close(n) => pool.close(n as usize).map(|()| n as usize),
        SlotChange::Open(n) => pool.open(n as usize).map(|()| n as usize),
    };
    changed.map(|n| n as u32).map_err(|why| match why {
        Unshaped::NoSuchSlot(n) => SlotRefused::NoSuchSlot(n as u32),
        Unshaped::Held(holder) => SlotRefused::Held(holder),
        Unshaped::Stranded(why) => SlotRefused::Stranded(why),
        Unshaped::Busy => SlotRefused::Busy,
        Unshaped::NotACheckout => SlotRefused::NotACheckout,
        Unshaped::Dirty(files) => SlotRefused::Dirty(files),
        Unshaped::LastSlot => SlotRefused::LastSlot,
        Unshaped::Vcs(why) => SlotRefused::Vcs(why),
    })
}

pub(crate) fn stranded_work(slots: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused> {
    pool(slots).stranded_work(slot as usize)
}

pub(crate) fn stranded_diff(slots: &SlotPool, slot: u32) -> Result<String, RescueRefused> {
    pool(slots).stranded_diff(slot as usize)
}

pub(crate) fn rescue(
    slots: &SlotPool,
    slot: u32,
    rescue: SlotRescue,
) -> Result<SlotRescued, RescueRefused> {
    pool(slots).rescue(slot as usize, rescue)
}
