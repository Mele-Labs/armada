//! A hosted session's lease, as `Vcs` asks for it: the pool, held by the Session's id.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use adapter_traits::{
    RescueRefused, ReviewBay, SlotAfterMerge, SlotChange, SlotKept, SlotLeased, SlotParkRefused,
    SlotParked, SlotPool,
    SlotReading, SlotRefused, SlotRescue, SlotRescued, SlotStanding, StrandedWork, Worktree,
    WorktreeSpec,
};

use super::{Holder, LeaseRefused, Leased, ParkRefused, Parked, Pool, SlotState, Unshaped};

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
    Err(String::from("Fleet seeds a hosted job's slot"))
}

pub(crate) fn lease(
    slots: &SlotPool,
    spec: &WorktreeSpec,
    holder: &str,
) -> Result<SlotLeased, LeaseRefused> {
    let since = since();
    let branch = spec.branch();
    match pool(slots).try_lease(&branch, &Holder::hosted(holder), since, &not_here)? {
        Leased::Full(_) => Ok(SlotLeased::Full),
        Leased::Took(lease) => Ok(SlotLeased::Took {
            slot: lease.slot() as u32,
            worktree: Worktree::at(lease.path().to_string_lossy(), branch),
            reused: !lease.made(),
        }),
    }
}

/// A slot onto `branch`, which exists, at its tip: the other half of a park.
pub(crate) fn lease_existing(
    slots: &SlotPool,
    branch: &str,
    holder: &str,
) -> Result<SlotLeased, LeaseRefused> {
    match pool(slots).try_lease_existing(branch, &Holder::hosted(holder), since(), &not_here)? {
        Leased::Full(_) => Ok(SlotLeased::Full),
        Leased::Took(lease) => Ok(SlotLeased::Took {
            slot: lease.slot() as u32,
            worktree: Worktree::at(lease.path().to_string_lossy(), branch),
            reused: !lease.made(),
        }),
    }
}

/// What slot `slot` holds now that the pull request from `merged` has merged.
pub(crate) fn after_merge(slots: &SlotPool, slot: u32, merged: &str) -> SlotAfterMerge {
    pool(slots).after_merge(slot as usize, merged)
}

/// Read here, at the edge: only `--status` reads it, to say for how long.
fn since() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// The branch a review's lease cuts at the pull request's head. Its own name, so two reads of
/// one pull request never contend for a branch git lets one checkout have.
fn review_branch(holder: &str) -> String {
    format!("review/{holder}")
}

/// A slot with pull request `pull_request`'s head checked out, held by `holder`.
pub(crate) fn lease_review(
    slots: &SlotPool,
    pull_request: u64,
    holder: &str,
) -> Result<ReviewBay, String> {
    let root = Path::new(slots.repo_root());
    let branch = review_branch(holder);
    let refspec = format!("+refs/pull/{pull_request}/head:refs/heads/{branch}");
    super::git::git(root, &["fetch", "--quiet", "--no-tags", "origin", &refspec])?;
    let forgotten = || {
        let _ = super::git::git(root, &["branch", "-D", "--quiet", &branch]);
    };
    match pool(slots).try_lease_existing(&branch, &Holder::hosted(holder), since(), &not_here) {
        Ok(Leased::Took(lease)) => {
            match super::git::read(lease.path(), &["rev-parse", "HEAD"]) {
                Ok(head) => Ok(ReviewBay::Took {
                    slot: lease.slot() as u32,
                    path: lease.path().to_string_lossy().into_owned(),
                    head,
                }),
                Err(why) => {
                    let _ = release_review(slots, lease.slot() as u32, holder);
                    Err(why)
                }
            }
        }
        Ok(Leased::Full(_)) => {
            forgotten();
            Ok(ReviewBay::Full)
        }
        Err(refused) => {
            forgotten();
            Err(refused.said())
        }
    }
}

/// Give back a slot [`lease_review`] leased. A read leaves nothing worth keeping, so whatever is
/// in the tree is dropped first and the pool's own refusal for uncommitted files cannot hold it.
pub(crate) fn release_review(slots: &SlotPool, slot: u32, holder: &str) -> Result<(), String> {
    let root = Path::new(slots.repo_root());
    let here = pool(slots).path_of(slot as usize);
    let mut off_the_head = false;
    if super::git::git_ok(&here, &["rev-parse", "--git-dir"]) {
        let _ = super::git::git(&here, &["checkout", "--quiet", "--force", "HEAD", "--", "."]);
        let _ = super::git::git(&here, &["clean", "--quiet", "-fd"]);
        // Off the pull request's commit before the slot is given back, so that deleting the lease's
        // branch leaves the slot's HEAD on a commit the base names and not on one nothing does.
        off_the_head = super::git::git(&here, &["switch", "--detach", "--quiet", slots.base()]).is_ok();
    }
    let released = pool(slots)
        .release_held(slot as usize, &Holder::hosted(holder))
        .map(|_| ())
        .map_err(|refused| refused.said());
    if off_the_head {
        let _ = super::git::git(root, &["branch", "-D", "--quiet", &review_branch(holder)]);
    }
    released
}

/// Commit the Session's work to its branch and give the slot back.
pub(crate) fn park(slots: &SlotPool, slot: u32, holder: &str) -> Result<SlotParked, SlotParkRefused> {
    parked_as(slots, slot, &Holder::hosted(holder))
}

/// [`park`] for a slot a hosted session leased. **It holds it under its own
/// id**, so the WIP commit is told whose it is. A session rewrites its own
/// branch inside its slot, so the work goes to the branch the slot is on, and
/// what it left running there is ended first.
pub(crate) fn park_hosted(
    slots: &SlotPool,
    slot: u32,
    session: &str,
) -> Result<SlotParked, SlotParkRefused> {
    answered(pool(slots).park_session(
        slot as usize,
        &Holder::hosted(session),
        &format!("job {session}"),
    ))
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

/// Write a slot an agent session holds as `to`'s, while `from` still holds it.
pub(crate) fn hand(slots: &SlotPool, slot: u32, from: &str, to: &str) -> Result<(), SlotParkRefused> {
    match pool(slots).hand(slot as usize, from, to) {
        Ok(()) => Ok(()),
        Err(ParkRefused::HeldByAnother(now)) => Err(SlotParkRefused::HolderChanged(now)),
        Err(ParkRefused::Busy) => Err(SlotParkRefused::Busy),
        Err(ParkRefused::NotASlot(_) | ParkRefused::NotLeased(_)) => Err(SlotParkRefused::NotLeased),
        Err(ParkRefused::Release(refused)) => Err(SlotParkRefused::Release(refused.said())),
        Err(ParkRefused::Vcs(why)) => Err(SlotParkRefused::Vcs(why)),
    }
}

/// [`park_hosted`] for a slot a terminal session stood in, once the process
/// that leased it has ended. Refused while that process still runs.
pub(crate) fn park_ended_session(
    slots: &SlotPool,
    slot: u32,
    session: &str,
) -> Result<SlotParked, SlotParkRefused> {
    answered(pool(slots).park_left(slot as usize, &format!("job {session}")))
}

fn parked_as(
    slots: &SlotPool,
    slot: u32,
    holder: &Holder,
) -> Result<SlotParked, SlotParkRefused> {
    answered(pool(slots).park(slot as usize, holder))
}

fn answered(parked: Result<Parked, ParkRefused>) -> Result<SlotParked, SlotParkRefused> {
    match parked {
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
        Err(ParkRefused::Release(refused)) => Err(SlotParkRefused::Release(refused.said())),
        Err(ParkRefused::Vcs(why)) => Err(SlotParkRefused::Vcs(why)),
    }
}

pub(crate) fn standing(slots: &SlotPool, slot: u32, session: &str) -> SlotStanding {
    match pool(slots).state(slot as usize) {
        SlotState::Unmade | SlotState::NotACheckout => SlotStanding::Gone,
        SlotState::Held { holder, .. } if holder == Holder::hosted(session) => SlotStanding::Held,
        SlotState::Held { holder, .. } => SlotStanding::HeldBy(holder.said()),
        SlotState::Free | SlotState::Abandoned { .. } => SlotStanding::Free,
        SlotState::Busy => SlotStanding::HeldBy(String::from("a lease under way")),
        SlotState::Stranded { branch, why, .. } => SlotStanding::HeldBy(format!(
            "nobody: {branch}'s holder is gone and it holds {why}"
        )),
    }
}

pub(crate) fn release(slots: &SlotPool, slot: u32, holder: &str) -> Result<(), SlotKept> {
    pool(slots)
        .release_held(slot as usize, &Holder::hosted(holder))
        .map(|_| ())
        .map_err(|refused| SlotKept(refused.said()))
}

pub(crate) fn readings(slots: &SlotPool) -> Vec<SlotReading> {
    pool(slots).readings()
}

/// [`readings`] that leaves a slot alone when nothing has touched it.
pub(crate) fn readings_kept(slots: &SlotPool, full: bool) -> Vec<SlotReading> {
    super::watching::readings(&pool(slots), full)
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

pub(crate) fn session_work(slots: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused> {
    pool(slots).session_work(slot as usize)
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
