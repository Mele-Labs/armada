//! The machine's Check slots: one budget taken by every process that runs a
//! Manifest's Check, whoever started it. `docs/concepts/manifest.md`, *How
//! many Checks run at once*.
//!
//! **An `flock` on one of `count` shared files, never a pid file**: the kernel
//! lets go when the holder dies, however it dies, so nothing is reclaimed.
//!
//! **All or none.** A Check wanting three slots takes three or waits holding
//! nothing, so two wide Checks never each hold half of what the other needs.
//! The cost is order: nothing here queues.

use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Set on everything a slot-holding Check starts. A Check started inside one —
/// a suite running `armada check` on a fixture — runs under its parent's slots,
/// or a machine whose every slot a suite holds deadlocks on its children.
pub const HELD_ENV: &str = "ARMADA_CHECK_SLOTS_HELD";

/// How often a waiting ask looks again. A Check takes seconds at the least.
const LOOK_AGAIN: Duration = Duration::from_millis(200);

/// Whether this process runs inside a Check that already holds its slots.
pub fn already_held() -> bool {
    std::env::var_os(HELD_ENV).is_some()
}

/// `count` slots in `dir`.
#[derive(Clone, Debug)]
pub struct CheckSlots {
    dir: PathBuf,
    count: usize,
}

/// Every slot an ask wanted. Dropping it closes the files, which lets the
/// locks go.
#[must_use = "the slots are given back the moment this is dropped"]
#[derive(Debug)]
pub struct Held {
    _files: Vec<File>,
}

/// What an ask found when it could not take what it wanted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InUse {
    /// Slots some other ask holds right now.
    pub in_use: usize,
    /// Slots there are.
    pub of: usize,
    /// Slots this ask wants, already clamped to `of`.
    pub wants: usize,
}

impl fmt::Display for InUse {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.wants {
            1 => out.write_str("waiting for a Check slot")?,
            wants => write!(out, "waiting for {wants} Check slots")?,
        }
        write!(out, ": {} of {} in use", self.in_use, self.of)
    }
}

impl CheckSlots {
    /// At least one: a machine with no slot would run no Check.
    pub fn at(dir: impl AsRef<Path>, count: usize) -> CheckSlots {
        CheckSlots {
            dir: dir.as_ref().to_path_buf(),
            count: count.max(1),
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// Take `wants` slots now, or say how many are in use and hold none.
    ///
    /// **Clamped to the slots there are**, so a Check wider than the machine
    /// takes every one rather than waiting for room that never comes. The
    /// error is the filesystem refusing, never contention.
    pub fn try_take(&self, wants: usize) -> io::Result<Result<Held, InUse>> {
        let wants = wants.clamp(1, self.count);
        std::fs::create_dir_all(&self.dir)?;
        let mut files = Vec::with_capacity(wants);
        let mut in_use = 0;
        for slot in 0..self.count {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(self.dir.join(format!("slot-{slot}")))?;
            match file.try_lock() {
                Ok(()) => files.push(file),
                Err(TryLockError::WouldBlock) => in_use += 1,
                Err(TryLockError::Error(why)) => return Err(why),
            }
            if files.len() == wants {
                return Ok(Ok(Held { _files: files }));
            }
        }
        Ok(Err(InUse {
            in_use,
            of: self.count,
            wants,
        }))
    }

    /// Take `wants` slots, waiting as long as it takes.
    ///
    /// `waiting` hears every look that found too few free, so a caller can say
    /// a pause is a queue rather than a hang. Safe to drop while waiting:
    /// nothing is held until it returns.
    pub async fn take(&self, wants: usize, mut waiting: impl FnMut(InUse)) -> io::Result<Held> {
        loop {
            match self.try_take(wants)? {
                Ok(held) => return Ok(held),
                Err(in_use) => waiting(in_use),
            }
            tokio::time::sleep(LOOK_AGAIN).await;
        }
    }
}
