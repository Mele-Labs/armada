//! `armada worktree`: lease a warm slot from the repository's pool, give it
//! back, and list who holds each. `adapters::leasing` is the pool; this reads
//! the Manifest for its size and base, and says what happened.
//!
//! **The path is the only thing on stdout**, so `path=$(armada worktree lease
//! x)` works; everything else goes to stderr.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use adapters::leasing::{Full, Holder, Lease, LeaseRefused, Pool, Slot, SlotState};
use config::Manifest;
use fleet::{CopyOnWrite, NotCloned, TheVolume};

use crate::setup::MANIFEST;

/// The base a Manifest that names none is leased from.
const BASE_UNSTATED: &str = "main";

/// The pool of the repository `within` belongs to, sized by its root
/// `armada.yml`. A repository with none declares no pool.
pub fn pool_of(within: &Path) -> Result<Pool, String> {
    let root = Pool::root_of(within)?;
    let path = root.join(MANIFEST);
    let manifest = Manifest::load(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    let seeds = manifest
        .seed()
        .map(|seed| seed.paths().to_vec())
        .unwrap_or_default();
    Ok(Pool::at(
        &root,
        manifest.worktrees().get() as usize,
        manifest.base().unwrap_or(BASE_UNSTATED),
        seeds,
    ))
}

/// `armada worktree lease <branch>`. Waits while every slot is held, and says
/// so once.
pub fn lease(within: &Path, branch: &str) -> u8 {
    let pool = match pool_of(within) {
        Ok(pool) => pool,
        Err(why) => return refused(&why),
    };
    let Some(holder) = Holder::the_caller() else {
        return refused(
            "the process this was run from could not be read, so nothing could hold the lease",
        );
    };
    let mut told = false;
    let seed = |from: &Path, to: &Path| {
        TheVolume.clone_tree(from, to).map_err(|why| match why {
            NotCloned::Unsupported { why } | NotCloned::Failed { why } => why,
        })
    };
    let leased = pool.lease(branch, &holder, now(), &seed, |full| {
        if !told {
            told = true;
            waiting(full, pool.count());
        }
    });
    match leased {
        Ok(lease) => {
            said(&lease, branch);
            println!("{}", lease.path().display());
            0
        }
        Err(LeaseRefused::BranchHoldsWork { branch, commits }) => refused(&format!(
            "{branch} already exists with {commits} commits on neither the remote nor the base. \
             A lease cuts its branch fresh from the base, so it will not reset this one: \
             land or push it, or lease a new name"
        )),
        Err(LeaseRefused::Vcs(why)) => refused(&why),
    }
}

/// `armada worktree release [<path>]`, the checkout standing in by default.
pub fn release(within: &Path, path: Option<PathBuf>) -> u8 {
    let pool = match pool_of(within) {
        Ok(pool) => pool,
        Err(why) => return refused(&why),
    };
    let path = path.unwrap_or_else(|| within.to_path_buf());
    match pool.release(&path) {
        Ok(released) => {
            eprintln!(
                "slot-{} is free. {} stays as a branch, and the slot's build stays warm",
                released.slot, released.branch
            );
            0
        }
        Err(why) => refused(&why.said()),
    }
}

/// `armada worktree --status`: one line per slot.
pub fn status(within: &Path) -> u8 {
    let pool = match pool_of(within) {
        Ok(pool) => pool,
        Err(why) => return refused(&why),
    };
    let now = now();
    for slot in pool.status() {
        println!("{}", line(&slot, now));
    }
    0
}

fn line(slot: &Slot, now: u64) -> String {
    let name = format!("slot-{}", slot.number);
    let path = slot.path.display();
    match &slot.state {
        SlotState::Unmade => format!("{name}  not made yet; the next lease makes it"),
        SlotState::Free => format!("{name}  free     {path}"),
        SlotState::Busy => format!("{name}  being taken or given back  {path}"),
        SlotState::NotACheckout => {
            format!("{name}  {path} is not a checkout, so nothing leases it until it is removed")
        }
        SlotState::Held {
            branch,
            holder,
            since,
            kept: None,
        } => format!(
            "{name}  held     {path}  {branch}  by {} for {}",
            holder.said(),
            ago(now, *since)
        ),
        SlotState::Held {
            branch,
            holder,
            since,
            kept: Some(why),
        } => format!(
            "{name}  kept     {path}  {branch}  by {} for {}, which ended and could not give it back: {why}",
            holder.said(),
            ago(now, *since)
        ),
        SlotState::Abandoned { branch, since } => format!(
            "{name}  free     {path}  {branch}'s holder is gone after {}, and nothing is unlanded",
            ago(now, *since)
        ),
        SlotState::Stranded { branch, since, why } => format!(
            "{name}  stranded {path}  {branch}'s holder is gone after {}, and it holds {why}",
            ago(now, *since)
        ),
    }
}

/// Said once, when a lease first finds every slot unavailable.
fn waiting(full: &Full, count: usize) {
    eprintln!("waiting for a worktree slot: {count} of {count} held");
    for slot in &full.slots {
        if let SlotState::Stranded { .. } | SlotState::NotACheckout = slot.state {
            eprintln!("  {}", line(slot, now()));
        }
    }
}

fn said(lease: &Lease, branch: &str) {
    if let Some(why) = lease.unfetched() {
        eprintln!("the base was not fetched, so {branch} is cut from what was last fetched: {why}");
    }
    if let Some(left) = lease.reclaimed_from() {
        eprintln!(
            "slot-{} was taken back from {left}, whose holder is gone",
            lease.slot()
        );
    }
    if let Some(commit) = lease.seeded_from() {
        eprintln!(
            "slot-{} is new, and its build was cloned from the seed at {}",
            lease.slot(),
            commit.get(..10).unwrap_or(commit)
        );
    }
    eprintln!("slot-{} is on {branch}, cut from the base", lease.slot());
}

fn ago(now: u64, since: u64) -> String {
    let minutes = now.saturating_sub(since) / 60;
    match minutes {
        0..=59 => format!("{minutes}m"),
        _ => format!("{}h{:02}m", minutes / 60, minutes % 60),
    }
}

/// The one read of the clock, at the edge, for the record's `since`.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

fn refused(why: &str) -> u8 {
    eprintln!("worktree: {why}");
    1
}
