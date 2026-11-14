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
//!
//! **A loaded machine runs one Check at a time** ([`CheckSlots::under_load`]),
//! so no Job's agent has to wait on the load average by hand.

use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

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

/// The fifteen-minute load at or over which the machine runs one Check at a
/// time. The owner's number for his 18-core Mac, kept by hand in every brief
/// until 14 Nov 2026, and the one Fleet's fetch of origin waits under.
pub const LOAD_LIMIT: u32 = 24;

/// How long a load reading is used before it is read again. The fifteen-minute
/// average barely moves in this time, and a wait looks every [`LOOK_AGAIN`].
const LOAD_KEPT: Duration = Duration::from_secs(10);

/// `count` slots in `dir`.
///
/// **The count is shared by every clone**, so [`CheckSlots::resize`] on the
/// one a Fleet holds reaches every place handing them out.
#[derive(Clone, Debug)]
pub struct CheckSlots {
    dir: PathBuf,
    count: Arc<AtomicUsize>,
    ahead: bool,
    load: Option<LoadGate>,
}

/// What reads the machine's fifteen-minute load. `None` where it would not say.
type ReadLoad = dyn Fn() -> Option<f64> + Send + Sync;

/// The limit, its reader, and the last reading, shared by every clone.
#[derive(Clone)]
struct LoadGate {
    limit: u32,
    read: Arc<ReadLoad>,
    last: Arc<Mutex<Option<(Instant, Option<f64>)>>>,
}

impl fmt::Debug for LoadGate {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("LoadGate").field("limit", &self.limit).finish_non_exhaustive()
    }
}

impl LoadGate {
    /// The load, where it is at or over the limit. A reading that fails gates
    /// nothing: a machine that cannot be read must not hold every Check back.
    fn over(&self) -> Option<Loaded> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let load = match *last {
            Some((at, load)) if at.elapsed() < LOAD_KEPT => load,
            _ => {
                let load = (self.read)();
                *last = Some((Instant::now(), load));
                load
            }
        };
        let load = load.filter(|load| load.is_finite() && *load >= f64::from(self.limit))?;
        Some(Loaded {
            load: load.round() as u32,
            limit: self.limit,
        })
    }
}

/// The machine's fifteen-minute load, off `sysctl -n vm.loadavg`, which says
/// `{ 7.22 5.98 4.54 }`. Read at utility priority. `None` where it would not
/// say, which is anywhere but darwin.
pub fn fifteen_minute_load() -> Option<f64> {
    let out = crate::Priority::Low
        .spawning("sysctl")
        .args(["-n", "vm.loadavg"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&out.stdout);
    said.trim()
        .trim_matches(|c| c == '{' || c == '}')
        .split_whitespace()
        .nth(2)?
        .parse()
        .ok()
}

/// The load an ask found the machine at, rounded to whole, and the limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub load: u32,
    pub limit: u32,
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
    /// The machine is at or over its load limit and another Check runs, so
    /// this one waits for the load to fall or that Check to end.
    pub loaded: Option<Loaded>,
}

impl fmt::Display for InUse {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.wants {
            1 => out.write_str("waiting for a Check slot")?,
            wants => write!(out, "waiting for {wants} Check slots")?,
        }
        match (self.behind_the_line, self.loaded) {
            (true, _) => out.write_str(": the merge line asked first"),
            (false, Some(Loaded { load, limit })) => write!(
                out,
                ": the fifteen-minute load is {load}, at or over {limit}, so Checks run one at a time"
            ),
            (false, None) => write!(out, ": {} of {} in use", self.in_use, self.of),
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
            load: None,
        }
    }

    /// The same slots, run one at a time while the fifteen-minute load is at
    /// or over `limit`: an ordinary ask then takes slots only where no other
    /// Check holds one, and says why it waits. `read` says the load.
    ///
    /// **The first Check always starts**, as a short machine slows Checks and
    /// never stops them: load from outside Checks would otherwise hold every
    /// one back for good. **The merge line's ask is never gated**, for
    /// [`CheckSlots::ahead`]'s reason.
    pub fn under_load(
        mut self,
        limit: u32,
        read: impl Fn() -> Option<f64> + Send + Sync + 'static,
    ) -> CheckSlots {
        self.load = Some(LoadGate {
            limit,
            read: Arc::new(read),
            last: Arc::new(Mutex::new(None)),
        });
        self
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
                loaded: None,
            }));
        }
        let loaded = match (&self.load, self.ahead) {
            (Some(gate), false) => gate.over(),
            _ => None,
        };
        let mut files = Vec::with_capacity(count);
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
            // Loaded, every slot is looked at, so a Check held anywhere is seen.
            if files.len() == wants && loaded.is_none() {
                return Ok(Ok(Held { _files: files }));
            }
        }
        if loaded.is_some() && in_use == 0 {
            files.truncate(wants);
            return Ok(Ok(Held { _files: files }));
        }
        Ok(Err(InUse {
            in_use,
            of: count,
            wants,
            behind_the_line: false,
            loaded: loaded.filter(|_| in_use > 0),
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
