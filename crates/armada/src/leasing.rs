//! `armada worktree`: lease a warm slot from the repository's pool, give it
//! back, and list who holds each. `adapters::leasing` is the pool; this reads
//! the Manifest for its size and base, and says what happened.
//!
//! **The path is the only thing on stdout**, so `path=$(armada worktree lease
//! x)` works; everything else goes to stderr.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use adapters::leasing::{Full, Holder, Lease, Pool, Slot, SlotState, Unshaped};
use config::Manifest;
use fleet::{CopyOnWrite, NotCloned, TheVolume};

use crate::setup::MANIFEST;
use crate::slot_asking::Asking;

/// The base a Manifest that names none is leased from.
const BASE_UNSTATED: &str = "main";

/// The pool of the repository `within` belongs to, sized by its root Manifest:
/// its `armada.yml` with Kit's copy laid over it, as `armada check` reads it. A
/// repository with neither declares no pool. Its workspaces are not read, so
/// one that will not load leaves leasing alone.
pub fn pool_of(within: &Path) -> Result<Pool, String> {
    pool_over(within, crate::manifests::kit_copy)
}

/// [`pool_of`], with `copy` finding Kit's copy of the root's `armada.yml`, or none.
pub(crate) fn pool_over(
    within: &Path,
    copy: impl FnOnce(&Path) -> Option<Arc<dyn config::KitCopy>>,
) -> Result<Pool, String> {
    let root = Pool::root_of(within)?;
    let path = root.join(MANIFEST);
    let manifest = match copy(&root) {
        Some(copy) => Manifest::load_over_copy(&path, copy),
        None => Manifest::load(&path),
    }
    .map_err(|why| format!("{}: {why}", path.display()))?;
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

/// `armada worktree lease [--existing] <branch>`. Waits while every slot is
/// held, and says so once. **Run by a hosted Session**, it also tells Fleet, which asks the
/// owner whether to wait or add a slot; with no Fleet to ask it waits alone, as ever.
pub fn lease(within: &Path, branch: &str, existing: bool) -> u8 {
    let asker = || {
        let session = std::env::var(Holder::SESSION_ENV).ok().filter(|id| !id.is_empty())?;
        Asking::for_session(&session)
    };
    lease_asking(within, branch, existing, asker)
}

/// [`lease`], with `asker` finding who to tell once every slot proves held. It is not called
/// for a lease that finds a slot at once.
pub(crate) fn lease_asking(
    within: &Path,
    branch: &str,
    existing: bool,
    asker: impl FnOnce() -> Option<Asking>,
) -> u8 {
    let pool = match pool_of(within) {
        Ok(pool) => pool.leased_for(std::env::var(Holder::AGENT_SESSION_ENV).ok()),
        Err(why) => return refused(&why),
    };
    let hosted = std::env::var(Holder::SESSION_ENV).ok();
    let Some(holder) = Holder::lessee(hosted.as_deref()) else {
        return refused(
            "the process this was run from could not be read, so nothing could hold the lease",
        );
    };
    let mut told = false;
    let mut asker = Some(asker);
    let mut asking: Option<Asking> = None;
    let seed = |from: &Path, to: &Path| {
        TheVolume.clone_tree(from, to).map_err(|why| match why {
            NotCloned::Unsupported { why } | NotCloned::Failed { why } => why,
        })
    };
    let mut wait = |full: &Full| {
        if !told {
            told = true;
            waiting(full, pool.count());
        }
        if let Some(find) = asker.take() {
            asking = find();
        }
        if let Some(asking) = asking.as_mut() {
            asking.look();
        }
    };
    let leased = match existing {
        true => pool.lease_existing(branch, &holder, now(), &seed, &mut wait),
        false => pool.lease(branch, &holder, now(), &seed, &mut wait),
    };
    // The call comes off the owner's list before the path is printed.
    drop(asking);
    match leased {
        Ok(lease) => {
            said(&lease, branch);
            println!("{}", lease.path().display());
            0
        }
        Err(other) => refused(&other.said()),
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
                "slot-{} is free. {} stays as a branch with its commits, and the slot's build stays warm. Nothing was pushed",
                released.slot, released.branch
            );
            0
        }
        Err(why) => refused(&why.said()),
    }
}

/// `armada worktree add`, `remove`, `close` or `open`: the pool changed on
/// this machine, beside the slots, which Fleet reads as this does.
pub fn reshape(within: &Path, change: impl FnOnce(&Pool) -> Result<String, Unshaped>) -> u8 {
    let pool = match pool_of(within) {
        Ok(pool) => pool,
        Err(why) => return refused(&why),
    };
    match change(&pool) {
        Ok(said) => {
            eprintln!("{said}");
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

/// The branch a slot is on, as git has it checked out, which is where the work is. The lease
/// records the branch the slot was cut for, and a session may have moved off it. Both are said
/// where they differ; `detached` is a checkout on no branch.
fn on_branch(slot: &Slot, leased: &str) -> String {
    let head = match adapters::leasing::checked_out(&slot.path) {
        Some(head) => head,
        None if slot.path.join(".git").exists() => String::from("detached"),
        None => return leased.to_string(),
    };
    match head == leased {
        true => head,
        false => format!("{head} (leased as {leased})"),
    }
}

pub(crate) fn line(slot: &Slot, now: u64) -> String {
    let name = match slot.closed {
        true => format!("slot-{} closed", slot.number),
        false => format!("slot-{}", slot.number),
    };
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
            ..
        } => format!(
            "{name}  held     {path}  {on}  by {} for {}",
            holder.said(),
            ago(now, *since),
            on = on_branch(slot, branch),
        ),
        SlotState::Abandoned { branch, since } => format!(
            "{name}  free     {path}  {on}  the holder is gone after {}, and nothing is unlanded",
            ago(now, *since),
            on = on_branch(slot, branch),
        ),
        SlotState::Stranded { branch, since, why } => format!(
            "{name}  stranded {path}  {on}  the holder is gone after {}, and it holds {why}",
            ago(now, *since),
            on = on_branch(slot, branch),
        ),
    }
}

/// Said once, when a lease first finds every slot unavailable.
fn waiting(full: &Full, count: usize) {
    let closed = full.slots.iter().filter(|slot| slot.closed).count();
    match closed {
        0 => eprintln!("waiting for a worktree slot: {count} of {count} held"),
        _ => eprintln!("waiting for a worktree slot: {closed} of {count} closed, the rest held"),
    }
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
    match lease.cut_from_base() {
        false => eprintln!("slot-{} is on {branch}, at its tip", lease.slot()),
        true => eprintln!("slot-{} is on {branch}, cut from the base", lease.slot()),
    }
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
