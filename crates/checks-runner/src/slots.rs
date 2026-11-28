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
//! **A machine short of memory runs one Check at a time, and Checks start one
//! at a time** ([`CheckSlots::watching`]), so no Job's agent has to wait on
//! the machine by hand.

use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

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

/// How often a reading of the machine and of the limits is used before both are
/// read again. A wait looks every [`LOOK_AGAIN`]; the kernel's pressure level and
/// the one-minute load barely move in this time.
const READING_KEPT: Duration = Duration::from_secs(10);

/// The file whose modified time is when the last Check on the machine started,
/// and whose lock is held while one starts, so two asks never both start in
/// the same spacing.
const STARTED: &str = "started";

/// `count` slots in `dir`.
///
/// **The count is shared by every clone**, so [`CheckSlots::resize`] on the
/// one a Fleet holds reaches every place handing them out.
#[derive(Clone, Debug)]
pub struct CheckSlots {
    dir: PathBuf,
    count: Arc<AtomicUsize>,
    ahead: bool,
    gate: Option<Gate>,
}

/// Which of the kernel's three load averages a load limit reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadWindow {
    One,
    Five,
    Fifteen,
}

impl LoadWindow {
    /// The window a setting in minutes names: 1, 5 or 15, and any other
    /// number the window at or under it, so no saved number fails to read.
    pub fn of_minutes(minutes: i64) -> LoadWindow {
        match minutes {
            ..=4 => LoadWindow::One,
            5..=14 => LoadWindow::Five,
            _ => LoadWindow::Fifteen,
        }
    }

    fn of(self, load: [f64; 3]) -> f64 {
        match self {
            LoadWindow::One => load[0],
            LoadWindow::Five => load[1],
            LoadWindow::Fifteen => load[2],
        }
    }
}

impl fmt::Display for LoadWindow {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            LoadWindow::One => "one-minute",
            LoadWindow::Five => "five-minute",
            LoadWindow::Fifteen => "fifteen-minute",
        })
    }
}

/// What the gate holds Checks to: `limits.*` in settings.json, read live.
/// `docs/concepts/manifest.md`, *How many Checks run at once*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateLimits {
    /// The kernel's memory pressure level at or over which memory is short:
    /// 1 normal, 2 warn, 4 critical. Zero never calls it short.
    pub pressure_level: u32,
    /// The load at or over which the machine is short. Zero reads no load.
    pub load_limit: u32,
    pub load_window: LoadWindow,
    /// How many slots may be in use for another Check to start while short.
    pub while_short: usize,
    /// At most one Check starts in this while another runs. Zero spaces none.
    pub spacing: Duration,
}

impl GateLimits {
    /// What makes the machine short in `reading`, memory first. A figure
    /// that was not read makes nothing short.
    pub fn short(&self, reading: &Reading) -> Option<Short> {
        if let Some(level) = reading.pressure.filter(|level| self.pressure_level > 0 && *level >= self.pressure_level) {
            return Some(Short::Memory { level, at: self.pressure_level });
        }
        let load = self.load_window.of(reading.load?);
        (self.load_limit > 0 && load.is_finite() && load >= f64::from(self.load_limit)).then(|| Short::Load {
            load: load.round() as u32,
            limit: self.load_limit,
            window: self.load_window,
        })
    }
}

/// The machine as the gate reads it. `None` where a figure would not say.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reading {
    /// The one-, five- and fifteen-minute load averages.
    pub load: Option<[f64; 3]>,
    /// The kernel's memory pressure level.
    pub pressure: Option<u32>,
}

/// The machine's load averages and memory pressure level, off `sysctl`, at
/// utility priority. Each figure is `None` where it would not say, which is
/// anywhere but darwin.
pub fn machine_reading() -> Reading {
    let said = |name: &str| {
        let out = crate::Priority::Low
            .spawning("sysctl")
            .args(["-n", name])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|out| out.status.success())?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    Reading {
        load: said("vm.loadavg").as_deref().and_then(load_averages),
        pressure: said("kern.memorystatus_vm_pressure_level").and_then(|level| level.parse().ok()),
    }
}

/// `{ 7.22 5.98 4.54 }`, as `sysctl -n vm.loadavg` says it.
fn load_averages(said: &str) -> Option<[f64; 3]> {
    let mut each = said.trim().trim_matches(|c| c == '{' || c == '}').split_whitespace().map(str::parse::<f64>);
    Some([each.next()?.ok()?, each.next()?.ok()?, each.next()?.ok()?])
}

/// What reads the limits in force and the machine, together.
type ReadGate = dyn Fn() -> (GateLimits, Reading) + Send + Sync;

/// The reader and its last reading, shared by every clone.
#[derive(Clone)]
struct Gate {
    read: Arc<ReadGate>,
    last: Arc<Mutex<Option<(Instant, GateLimits, Reading)>>>,
}

impl fmt::Debug for Gate {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Gate").finish_non_exhaustive()
    }
}

impl Gate {
    fn look(&self) -> (GateLimits, Reading) {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        match *last {
            Some((at, limits, reading)) if at.elapsed() < READING_KEPT => (limits, reading),
            _ => {
                let (limits, reading) = (self.read)();
                *last = Some((Instant::now(), limits, reading));
                (limits, reading)
            }
        }
    }
}

/// Why the machine is short, as an ask found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Short {
    /// The kernel's pressure level, at or over the level that counts.
    Memory { level: u32, at: u32 },
    /// The load in the window read, rounded to whole, at or over the limit.
    Load { load: u32, limit: u32, window: LoadWindow },
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
    pub why: Waits,
}

/// Why an ask waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waits {
    /// Too few slots are free.
    Full,
    /// The merge line is waiting [`CheckSlots::ahead`], so nothing was
    /// counted: `in_use` is every slot.
    BehindTheLine,
    /// The machine is short and `while_short` slots or more are in use, so
    /// this one waits for the machine to recover or a Check to end.
    Short { short: Short, while_short: usize },
    /// Another Check started within `spacing`, or is starting now.
    Spaced { spacing: Duration },
}

impl fmt::Display for InUse {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.wants {
            1 => out.write_str("waiting for a Check slot")?,
            wants => write!(out, "waiting for {wants} Check slots")?,
        }
        let run = |out: &mut fmt::Formatter<'_>, while_short: usize| match while_short {
            1 => out.write_str(", so Checks run one at a time"),
            n => write!(out, ", so Checks start only while fewer than {n} slots are in use"),
        };
        match self.why {
            Waits::Full => write!(out, ": {} of {} in use", self.in_use, self.of),
            Waits::BehindTheLine => out.write_str(": the merge line asked first"),
            Waits::Short { short: Short::Memory { level, at }, while_short } => {
                write!(out, ": memory is short, at the kernel's pressure level {level}, at or over {at}")?;
                run(out, while_short)
            }
            Waits::Short { short: Short::Load { load, limit, window }, while_short } => {
                write!(out, ": the {window} load is {load}, at or over {limit}")?;
                run(out, while_short)
            }
            Waits::Spaced { spacing } => write!(
                out,
                ": a Check started in the last {} seconds, and one starts at a time so each shows before the next",
                spacing.as_secs()
            ),
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
            gate: None,
        }
    }

    /// The same slots, gated by the limits and the machine `read` says, read
    /// at most every [`READING_KEPT`]. While the machine is
    /// [`GateLimits::short`], an ordinary ask takes slots only while fewer
    /// than `while_short` are in use, and says why it waits. **At most one
    /// Check starts in a `spacing` while another runs**, so a gate that opens
    /// never lets every waiting Check in at once: seven did at 04:42 on 28 Nov
    /// 2026, into a machine already short of memory, and the load went from
    /// 17 to 49 while the cores busy fell from 9.7 to 7.
    ///
    /// **The first Check always starts**, as a short machine slows Checks and
    /// never stops them: work from outside Checks would otherwise hold every
    /// one back for good. **The merge line's ask is never gated**, for
    /// [`CheckSlots::ahead`]'s reason.
    pub fn watching(mut self, read: impl Fn() -> (GateLimits, Reading) + Send + Sync + 'static) -> CheckSlots {
        self.gate = Some(Gate { read: Arc::new(read), last: Arc::new(Mutex::new(None)) });
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
        let waits = |in_use, why| Ok(Err(InUse { in_use, of: count, wants, why }));
        if !self.ahead && self.someone_waits_ahead()? {
            return waits(count, Waits::BehindTheLine);
        }
        let looked = self.gate.as_ref().filter(|_| !self.ahead).map(Gate::look);
        let short = looked.and_then(|(limits, reading)| limits.short(&reading));
        let while_short = looked.map_or(1, |(limits, _)| limits.while_short.max(1));
        let spacing = looked.map_or(Duration::ZERO, |(limits, _)| limits.spacing);
        // Held while this ask starts, so two never both start in one spacing.
        let started = self.open(STARTED)?;
        match started.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return waits(0, Waits::Spaced { spacing }),
            Err(TryLockError::Error(why)) => return Err(why),
        }
        // Every slot is looked at, so a Check held anywhere is seen.
        let mut files = Vec::with_capacity(count);
        let mut in_use = 0;
        for slot in 0..count {
            let file = self.open(&format!("slot-{slot}"))?;
            match file.try_lock() {
                Ok(()) => files.push(file),
                Err(TryLockError::WouldBlock) => in_use += 1,
                Err(TryLockError::Error(why)) => return Err(why),
            }
        }
        if let Some(short) = short.filter(|_| in_use >= while_short) {
            return waits(in_use, Waits::Short { short, while_short });
        }
        if files.len() < wants {
            return waits(in_use, Waits::Full);
        }
        if in_use > 0 && !spacing.is_zero() && !spaced_since(&started, spacing)? {
            return waits(in_use, Waits::Spaced { spacing });
        }
        started.set_modified(SystemTime::now())?;
        files.truncate(wants);
        Ok(Ok(Held { _files: files }))
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

    /// The checkout at `root`, held for one Check at a time, waiting as long as
    /// it takes. Taken before the machine's slots, so a second Check in a
    /// checkout waits holding none of them.
    ///
    /// **One Check per checkout**, since two in one checkout share its build
    /// directory and the second only queues on cargo's lock while holding a
    /// slot the machine could give another. 386 agent runs of 14 to 21 Nov
    /// 2026, 26.3 h, were a second run of the same Check in the same slot.
    ///
    /// `waiting` hears once that another Check holds it. Safe to drop while
    /// waiting: nothing is held until it returns.
    pub async fn take_checkout(&self, root: &Path, mut waiting: impl FnMut()) -> io::Result<Held> {
        std::fs::create_dir_all(&self.dir)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.dir.join(checkout_file(root)))?;
        let mut said = false;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Held { _files: vec![file] }),
                Err(TryLockError::WouldBlock) if !std::mem::replace(&mut said, true) => waiting(),
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(why)) => return Err(why),
            }
            tokio::time::sleep(LOOK_AGAIN).await;
        }
    }

    /// The file `name` in the slot directory, made where it is missing.
    fn open(&self, name: &str) -> io::Result<File> {
        std::fs::create_dir_all(&self.dir)?;
        OpenOptions::new().create(true).truncate(false).write(true).open(self.dir.join(name))
    }

    fn ahead_file(&self) -> io::Result<File> {
        self.open(AHEAD)
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

/// The file that holds the checkout at `root`: its path, made a file name, so
/// every clone and worktree has its own beside the machine's slots.
fn checkout_file(root: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let name: String = root
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    format!("checkout-{name}")
}

/// Whether the last Check on the machine started `spacing` ago or more, by the
/// modified time of `started`. A time ahead of the clock counts as long ago,
/// so a clock set back never holds every Check.
fn spaced_since(started: &File, spacing: Duration) -> io::Result<bool> {
    let at = started.metadata()?.modified()?;
    Ok(SystemTime::now().duration_since(at).map_or(true, |since| since >= spacing))
}
