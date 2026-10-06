//! `setup.auto_release` and `setup.auto_release_grace_minutes`: whether Fleet
//! may pause a parked Job to free a worktree slot, and how long it leaves a Job
//! alone first. `docs/concepts/fleet.md`, *A paused Job gives its slot back*.

use std::num::NonZeroU32;

use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// A parked Job whose last event is younger than this is left alone: Fleet
/// cannot see what Bridge shows, so a Job a person just opened may be in use.
const GRACE_UNSTATED: NonZeroU32 = NonZeroU32::new(15).expect("fifteen is not zero");

/// What the repository allows Fleet to do about a full pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoRelease {
    /// On unless the file says `false`.
    pub on: bool,
    /// Minutes since a Job's last event before it may be taken. Fifteen where
    /// the file says nothing.
    pub grace_minutes: NonZeroU32,
}

impl AutoRelease {
    pub(super) fn unstated() -> AutoRelease {
        AutoRelease {
            on: true,
            grace_minutes: GRACE_UNSTATED,
        }
    }
}

/// Both keys, read from the root only: the pool is the root's, and a second
/// value in a workspace's file is one nothing reads.
pub(super) fn read(
    table: &mut Table<'_>,
    in_a_workspace: bool,
    out: &mut Vec<Refusal>,
) -> Option<AutoRelease> {
    let mut read = AutoRelease::unstated();
    let mut sound = true;
    for key in ["auto_release", "auto_release_grace_minutes"] {
        let Some(value) = table.optional(key) else {
            continue;
        };
        if in_a_workspace {
            out.push(Refusal::new(table.at(key), Fault::RootOnly));
            sound = false;
            continue;
        }
        let at = table.at(key);
        if key == "auto_release" {
            match yaml::flag(&at, value, out) {
                Some(on) => read.on = on,
                None => sound = false,
            }
        } else {
            match yaml::positive(&at, value, out).and_then(NonZeroU32::new) {
                Some(minutes) => read.grace_minutes = minutes,
                None => sound = false,
            }
        }
    }
    sound.then_some(read)
}
