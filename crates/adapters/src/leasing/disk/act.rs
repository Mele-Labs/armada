//! Carrying a plan out. **Every removal holds cargo's lock on the folder first** and is skipped
//! where a build holds it: [`trim_target`](crate::leasing::trim_target)'s guarantee, kept for each.

use std::path::PathBuf;
use std::time::SystemTime;

use super::{pressure_plan, staged_plan, Act, BuildFolder, How, Reclaimed, MARGIN};
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
/// over `floor`.** One line to `said` for each folder removed whole and one for the pass. `None`
/// where there was no pass: free is unread, or at or over `floor`.
pub fn reclaim(
    folders: &[BuildFolder],
    now: SystemTime,
    floor: u64,
    free: &mut dyn FnMut() -> Option<u64>,
    said: &mut dyn FnMut(&str),
) -> Option<Reclaimed> {
    let started_at = free()?;
    if started_at >= floor {
        return None;
    }
    let stop_at = floor.saturating_add(MARGIN);
    let (mut last, mut acts) = (started_at, 0usize);
    let plan = pressure_plan(folders, now);
    let mut done = vec![false; plan.len()];
    for (at, act) in plan.iter().enumerate() {
        if !perform(act) {
            continue;
        }
        done[at] = true;
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
    // What was not done: the acts the stop came before and those a build held, whose folder is still there.
    let left = plan.iter().zip(&done).filter(|(act, done)| !**done && act.folder.is_dir()).map(|(act, _)| act.clone());
    Some(Reclaimed { acts, freed: last.saturating_sub(started_at), left: distinct(left) })
}

/// Each act's folder once, in the order they first appear.
fn distinct(acts: impl IntoIterator<Item = Act>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for act in acts {
        if !folders.contains(&act.folder) {
            folders.push(act.folder);
        }
    }
    folders
}

/// The first `count` folders `pressure_plan` names, each dropped as the plan would drop it and
/// behind the same lock, a folder a build holds left. The folders dropped, in order.
pub fn drop_first(folders: &[BuildFolder], now: SystemTime, count: usize) -> Vec<PathBuf> {
    let plan = pressure_plan(folders, now);
    let named: Vec<PathBuf> = distinct(plan.iter().cloned()).into_iter().take(count).collect();
    named
        .into_iter()
        .filter(|folder| plan.iter().find(|act| &act.folder == folder).is_some_and(perform))
        .collect()
}
