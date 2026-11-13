//! What Fleet drops from a build folder, in which order, and when. The decisions
//! are here and pure: a list of build folders in, an ordered list of acts out.
//! `find` reads the disk into that list, and `act` carries the acts out behind
//! cargo's lock. The policy is the owner's of 13 Nov 2026,
//! `.armada/documents/disk-reclaim-policy.html`.

mod act;
mod find;

pub use act::{drop_first, reclaim, shed_free_slots};
pub use find::{build_folders_in, every_checkout, inventory, mark_landed};

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// A pass under pressure stops this far above the floor, so it does not empty everything.
pub const MARGIN: u64 = 20 * 1024 * 1024 * 1024;
/// A free slot unbuilt this long loses its incremental caches, and after [`WHOLE_AFTER`] the folder.
pub const INCREMENTAL_AFTER: Duration = Duration::from_secs(2 * 60 * 60);
pub const WHOLE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
/// Under pressure a folder built more recently than this keeps its incremental caches.
const BUILDING_LATELY: Duration = Duration::from_secs(60 * 60);
/// Under pressure a held slot's folder goes whole only once unbuilt this long.
const HELD_IDLE: Duration = Duration::from_secs(6 * 60 * 60);

/// Whose build folder it is, which decides what may be done to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The repository's own checkout. **Never removed whole**: the running Fleet comes from it.
    Main,
    /// `.armada/preview`. **Never removed whole**: the running Fleet and Bridge come from it.
    Preview,
    FreeSlot,
    HeldSlot,
    /// A Job's or agent's worktree, a base or a land tree.
    Other,
}

/// One `target` that holds `CACHEDIR.TAG`, with what the plan needs to know of it.
#[derive(Clone, Debug)]
pub struct BuildFolder {
    pub path: PathBuf,
    pub checkout: PathBuf,
    pub role: Role,
    pub last_built: SystemTime,
    pub has_incremental: bool,
    /// The checkout's HEAD is already in the base. Read by [`mark_landed`], which costs a git call each.
    pub landed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Incremental,
    Whole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub folder: PathBuf,
    pub how: How,
}

/// What a pass under pressure did, for a surface to say: how many acts it carried out, what the
/// volume gained over it, and the folders it left, in the order the next pass would drop them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reclaimed {
    pub acts: usize,
    pub freed: u64,
    pub left: Vec<PathBuf>,
}

fn idle(folder: &BuildFolder, now: SystemTime) -> Duration {
    now.duration_since(folder.last_built).unwrap_or_default()
}

fn oldest_first<'a>(mut folders: Vec<&'a BuildFolder>) -> Vec<&'a BuildFolder> {
    folders.sort_by_key(|folder| folder.last_built);
    folders
}

fn act(folder: &BuildFolder, how: How) -> Act {
    Act {
        folder: folder.path.clone(),
        how,
    }
}

/// What a free slot sheds on the regular sweep: incremental caches after
/// [`INCREMENTAL_AFTER`], the whole folder after [`WHOLE_AFTER`]. Nothing else sheds by age.
pub fn staged_plan(folders: &[BuildFolder], now: SystemTime) -> Vec<Act> {
    folders
        .iter()
        .filter(|folder| folder.role == Role::FreeSlot)
        .filter_map(|folder| match idle(folder, now) {
            quiet if quiet > WHOLE_AFTER => Some(act(folder, How::Whole)),
            quiet if quiet > INCREMENTAL_AFTER && folder.has_incremental => {
                Some(act(folder, How::Incremental))
            }
            _ => None,
        })
        .collect()
}

/// Everything a short disk may drop, in the order to drop it; the caller stops when there is room.
///
/// 1. Incremental caches of folders not built in the last hour, oldest first.
/// 2. Whole folders of checkouts whose HEAD the base has, oldest first. **Not a held slot's**: its
///    Job has not ended and a branch with no commits yet is trivially in the base.
/// 3. Whole folders of free slots, least recently built first.
/// 4. Whole folders of held slots idle for six hours, least recently built first.
///
/// Main and the preview appear in step 1 only.
pub fn pressure_plan(folders: &[BuildFolder], now: SystemTime) -> Vec<Act> {
    let all = || folders.iter();
    let incremental = oldest_first(
        all()
            .filter(|folder| folder.has_incremental && idle(folder, now) > BUILDING_LATELY)
            .collect(),
    );
    let landed = oldest_first(
        all()
            .filter(|folder| folder.landed && matches!(folder.role, Role::Other | Role::FreeSlot))
            .collect(),
    );
    let free = oldest_first(
        all()
            .filter(|folder| folder.role == Role::FreeSlot && !folder.landed)
            .collect(),
    );
    let held = oldest_first(
        all()
            .filter(|folder| folder.role == Role::HeldSlot && idle(folder, now) > HELD_IDLE)
            .collect(),
    );
    incremental
        .into_iter()
        .map(|folder| act(folder, How::Incremental))
        .chain(
            landed
                .into_iter()
                .chain(free)
                .chain(held)
                .map(|folder| act(folder, How::Whole)),
        )
        .collect()
}
