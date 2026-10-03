//! A pool slot a Job holds: given back by the pool's own rules under
//! `--force`, and otherwise left held and named.
//!
//! **A slot is never removed here.** The pool reuses it, so the only way out is
//! a release, which refuses a dirty tree or commits on neither the remote nor
//! the base — `docs/concepts/fleet.md`, *A Job's slot*.

use std::path::Path;

use adapter_traits::slot_path;
use adapters::leasing::{holder_of, Holder, SlotState};
use adapters::UnmergedWork;

/// A slot a Job still holds after the clean, and the Job that holds it.
#[derive(Debug)]
pub struct SlotHeld {
    pub job_id: String,
    pub title: String,
    pub slot: u32,
    pub path: String,
    pub why: Holding,
}

/// Why a Job's slot is still held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holding {
    /// The Job has not ended, so the slot is its work in progress. Neither form
    /// of the clean touches it.
    Live,
    /// The Job completed and holds the slot until a person clears it.
    /// `--force` gives it back.
    Completed,
    /// The Job ended and the pool refused its release, for the reason it
    /// wrote on the slot. `--force` asks again.
    Kept(String),
    /// `--force` asked and the pool refused, for this reason.
    Refused(String),
}

/// Give back slot `slot` where `job` holds it and has ended, and `unmerged`
/// is `--force`.
///
/// `None` where the Job does not hold it: a slot given back is another
/// holder's, and the Job's derived path stands in for it. `Some(Ok)` once the
/// slot is released and its branch is free to delete.
pub(super) fn give_back(
    root: &Path,
    slot: u32,
    job: &str,
    unmerged: UnmergedWork,
) -> Option<Result<(), Holding>> {
    let holder = Holder::job(job);
    if holder_of(Path::new(&slot_path(&root.to_string_lossy(), slot))) != Some(holder.clone()) {
        return None;
    }
    let pool = match crate::leasing::pool_of(root) {
        Ok(pool) => pool,
        Err(why) => return Some(Err(Holding::Refused(why))),
    };
    // The slot's record, not the Job's status, says it has ended: it is what
    // the pool itself reads, and `--status` shows it as `done` or `kept`.
    let SlotState::Held {
        completed, kept, ..
    } = pool.state(slot as usize)
    else {
        return None;
    };
    Some(match (completed || kept.is_some(), unmerged) {
        (false, _) => Err(Holding::Live),
        (true, UnmergedWork::Keep) => Err(kept.map_or(Holding::Completed, Holding::Kept)),
        (true, UnmergedWork::Delete) => pool
            .release_held(slot as usize, &holder)
            .map(|_| ())
            .map_err(|refused| Holding::Refused(refused.said())),
    })
}
