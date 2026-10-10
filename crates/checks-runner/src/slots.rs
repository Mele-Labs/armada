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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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

/// Set on an `armada check` the merge line starts, so its ask goes
/// [`CheckSlots::ahead`] of every other.
pub const AHEAD_ENV: &str = "ARMADA_CHECK_AHEAD";

/// The file an ask ahead holds while it waits. Every other ask that finds it
/// held waits too, so a freed slot goes to the ask ahead.
const AHEAD: &str = "ahead";

/// `count` slots in `dir`.
///
/// **The count is shared by every clone**, so [`CheckSlots::resize`] on the
/// one a Fleet holds reaches every place handing them out.
#[derive(Clone, Debug)]
pub struct CheckSlots {
    dir: PathBuf,
    count: Arc<AtomicUsize>,
    ahead: bool,
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
    /// The merge line is waiting [`CheckSlots::ahead`], so nothing was
    /// counted: `in_use` is every slot.
    pub behind_the_line: bool,
}

impl fmt::Display for InUse {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.wants {
            1 => out.write_str("waiting for a Check slot")?,
            wants => write!(out, "waiting for {wants} Check slots")?,
        }
        match self.behind_the_line {
            true => out.write_str(": the merge line asked first"),
            false => write!(out, ": {} of {} in use", self.in_use, self.of),
        }
    }
}

impl CheckSlots {
    /// At least one: a machine with no slot would run no Check.
    pub fn at(dir: impl AsRef<Path>, count: usize) -> CheckSlots {
        CheckSlots {
            dir: dir.as_ref().to_path_buf(),
            count: Arc::new(AtomicUsize::new(count.max(1))),
            ahead: false,
        }
    }

    /// The same slots, asked for ahead of every ordinary ask: the merge line's,
    /// so a turn does not queue behind agents' own runs. It jumps the wait, never
    /// a holder — a running Check keeps its slots.
    pub fn ahead(mut self) -> CheckSlots {
        self.ahead = true;
        self
    }

    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Put a new count in force, at least one, from the next ask. **A holder
    /// keeps what it holds**: past a smaller count its slot is simply never
    /// offered again once it is given back, so nothing running is stopped.
    pub fn resize(&self, count: usize) {
        self.count.store(count.max(1), Ordering::Relaxed);
    }

    /// Take `wants` slots now, or say how many are in use and hold none.
    ///
    /// **Clamped to the slots there are**, so a Check wider than the machine
    /// takes every one rather than waiting for room that never comes. The
    /// error is the filesystem refusing, never contention.
    pub fn try_take(&self, wants: usize) -> io::Result<Result<Held, InUse>> {
        let count = self.count();
        let wants = wants.clamp(1, count);
        std::fs::create_dir_all(&self.dir)?;
        if !self.ahead && self.someone_waits_ahead()? {
            return Ok(Err(InUse {
                in_use: count,
                of: count,
                wants,
                behind_the_line: true,
            }));
        }
        let mut files = Vec::with_capacity(wants);
        let mut in_use = 0;
        for slot in 0..count {
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
            of: count,
            wants,
            behind_the_line: false,
        }))
    }

    /// Take `wants` slots, waiting as long as it takes.
    ///
    /// `waiting` hears every look that found too few free, so a caller can say
    /// a pause is a queue rather than a hang. Safe to drop while waiting:
    /// nothing is held until it returns.
    pub async fn take(&self, wants: usize, mut waiting: impl FnMut(InUse)) -> io::Result<Held> {
        // Held until this returns, so every ordinary ask waits behind it.
        let _first = match self.ahead {
            true => Some(self.wait_ahead().await?),
            false => None,
        };
        loop {
            match self.try_take(wants)? {
                Ok(held) => return Ok(held),
                Err(in_use) => waiting(in_use),
            }
            tokio::time::sleep(LOOK_AGAIN).await;
        }
    }

    fn ahead_file(&self) -> io::Result<File> {
        std::fs::create_dir_all(&self.dir)?;
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.dir.join(AHEAD))
    }

    /// Whether an ask ahead is waiting now. Shared and let go at once, so two
    /// ordinary asks never hold each other up here.
    fn someone_waits_ahead(&self) -> io::Result<bool> {
        match self.ahead_file()?.try_lock_shared() {
            Ok(()) => Ok(false),
            Err(TryLockError::WouldBlock) => Ok(true),
            Err(TryLockError::Error(why)) => Err(why),
        }
    }

    /// The `ahead` file, held exclusively: behind another ask ahead, if there
    /// is one, and never behind an ordinary ask for longer than its look.
    async fn wait_ahead(&self) -> io::Result<File> {
        let file = self.ahead_file()?;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) => tokio::time::sleep(LOOK_AGAIN).await,
                Err(TryLockError::Error(why)) => return Err(why),
            }
        }
    }
}
