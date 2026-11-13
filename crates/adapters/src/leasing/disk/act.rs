//! Carrying a plan out. **Every removal holds cargo's lock on the folder first** and is skipped
//! where a build holds it: [`trim_target`](crate::leasing::trim_target)'s guarantee, kept for each.

use std::time::SystemTime;

use super::{pressure_plan, staged_plan, Act, BuildFolder, How, MARGIN};
use crate::leasing::trim::{locks, remove_incremental, size_of};

/// Whether the act was done: not where a build holds the folder or it is already gone.
fn perform(act: &Act) -> bool {
    if !act.folder.is_dir() {
        return false;
    }
    let Some(_held) = locks(&act.folder) else {
        return false;
    };
    match act.how {
        How::Incremental => remove_incremental(&act.folder),
        How::Whole => {
            let _ = std::fs::remove_dir_all(&act.folder);
        }
    }
    true
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// The regular sweep's shed of free slots' builds by age. One line to `said` for each folder
/// removed whole.
pub fn shed_free_slots(folders: &[BuildFolder], now: SystemTime, said: &mut dyn FnMut(&str)) {
    for act in staged_plan(folders, now) {
        let size = (act.how == How::Whole).then(|| size_of(&act.folder));
        if perform(&act) && act.how == How::Whole {
            said(&format!(
                "{} was a free slot's build unbuilt for a day and was removed whole, {} freed",
                act.folder.display(),
                gib(size.unwrap_or_default())
            ));
        }
    }
}

/// A pass under pressure. `free` reads the free bytes on the volume, and is read before the pass
/// and after every act: **it stops at `floor` plus [`MARGIN`], and does nothing while free is at or
/// over `floor`.** One line to `said` for each folder removed whole and one for the pass.
pub fn reclaim(
    folders: &[BuildFolder],
    now: SystemTime,
    floor: u64,
    free: &mut dyn FnMut() -> Option<u64>,
    said: &mut dyn FnMut(&str),
) {
    let Some(started_at) = free() else {
        return;
    };
    if started_at >= floor {
        return;
    }
    let stop_at = floor.saturating_add(MARGIN);
    let (mut last, mut acts) = (started_at, 0usize);
    for act in pressure_plan(folders, now) {
        if !perform(&act) {
            continue;
        }
        acts += 1;
        let Some(now_free) = free() else {
            break;
        };
        if act.how == How::Whole {
            said(&format!(
                "{} was removed whole to make room, {} freed",
                act.folder.display(),
                gib(now_free.saturating_sub(last))
            ));
        }
        last = now_free;
        if now_free >= stop_at {
            break;
        }
    }
    if acts > 0 {
        said(&format!(
            "disk pressure: {} free under the {} floor, {} acts, {} free after",
            gib(started_at),
            gib(floor),
            acts,
            gib(last)
        ));
    }
}
