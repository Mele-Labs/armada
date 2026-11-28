//! On a loaded machine, one Job's builds run and every other Job's wait stopped, taking turns.
//! `docs/concepts/manifest.md`, *Builds take turns on a loaded machine*.
//!
//! **Stopped, never ended.** A held compiler gets `SIGSTOP` and keeps all it has done; `SIGCONT`
//! picks it up where it was. Eight builds sharing twelve cores at a load of 37 each crawled for
//! over an hour (20 Nov 2026); one running alone finishes, and the next takes its turn.
//!
//! **What is held is a Job's compilers and nothing else**: `rustc`, `rustdoc` and `clippy-driver`
//! in the Job's process tree or carrying its `ARMADA_SESSION`. **Never `cargo`**: it holds the
//! package-cache lock while it resolves, and a stopped one held it for every Job's cargo until
//! their Checks spent 900 seconds on `Blocking waiting for file lock on package cache` (21 Nov
//! 2026). A cargo whose compilers are stopped waits on them, holding nothing another build needs,
//! and a stopped cargo in a Job's tree, which only an earlier Fleet stopped, is continued. A Check's are left running, as Checks
//! already run one at a time under load and a held one would spend its budget waiting. A build
//! outside every Job, such as one a person started, is not Fleet's to stop.
//!
//! **The turn goes to the prioritised Job while it builds**, and otherwise round the Jobs that
//! are building, oldest first, [`TURN`] each. A Job with no compiler running gives its turn up at
//! once. Under the limit, or with one Job building, everything held runs again.
//!
//! **Loaded is the machine short by [`fleet::limits::gate_limits`](crate::limits::gate_limits)**,
//! the rule the machine's Check slots keep: memory at the kernel's pressure level, or a load
//! over its limit where one is set. Short memory stays so for a [`TURN`] after the pressure eases.
//!
//! **A Job is told both ways** ([`Turns::jobs_held`]), so its agent waits on a quiet build
//! rather than starting it again: the owner's choice of 20 Nov 2026.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

/// How long a Job's builds run before the next building Job's do, while none is prioritised.
pub(crate) const TURN: Duration = Duration::from_secs(5 * 60);

/// How often Fleet looks: a compiler a running build starts is held within this.
pub(crate) const EVERY: Duration = Duration::from_secs(5);

/// The processes held: a build's compilers. A linker or build script they started finishes what it
/// is doing and waits on its stopped parent, and the cargo that started them waits on them.
const COMPILERS: [&str; 3] = ["rustc", "rustdoc", "clippy-driver"];

/// The build driver, never held: stopped, it keeps the package-cache lock from every other build.
const DRIVER: &str = "cargo";

/// How far up the parent chain a compiler is followed to its Job: an agent's shell, a package
/// runner, a test runner and cargo itself sit between them.
const DEEPEST: usize = 32;

/// A Job whose builds may take a turn, in the order turns go round: oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Contender {
    pub job: String,
    /// The Job's agent, or the keeper that started it: what its builds descend from.
    pub roots: Vec<u32>,
    pub prioritised: bool,
}

/// What to send one process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Signal {
    Stop(u32),
    Continue(u32),
}

/// One line of `ps -axo pid=,ppid=,stat=,comm=`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    pid: u32,
    ppid: u32,
    stopped: bool,
    compiler: bool,
    driver: bool,
}

/// The command [`Turns::look`] reads.
pub(crate) const LISTING: [&str; 2] = ["-axo", "pid=,ppid=,stat=,comm="];

/// `comm` is the executable's path, which may hold a space, so it is the rest of the line.
fn rows(listing: &str) -> Vec<Row> {
    listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            let stat = fields.next()?;
            let comm = fields.collect::<Vec<_>>().join(" ");
            let name = comm.rsplit('/').next().unwrap_or_default();
            Some(Row { pid, ppid, stopped: stat.starts_with('T'), compiler: COMPILERS.contains(&name), driver: name == DRIVER })
        })
        .collect()
}

/// Who holds the turn, and what this Fleet stopped.
#[derive(Debug, Default)]
pub(crate) struct Turns {
    leader: Option<(String, Instant)>,
    /// Each compiler this Fleet stopped, and the Job it is of.
    held: BTreeMap<u32, String>,
    /// When memory was last found short.
    pressed: Option<Instant>,
}

impl Turns {
    /// The Job whose builds run, while any are held.
    pub(crate) fn leader(&self) -> Option<&str> {
        self.leader.as_ref().map(|(job, _)| job.as_str())
    }

    /// Whether builds take turns: the machine `short`, or short of memory within a [`TURN`].
    /// **The turn's wait is the hysteresis**: held compilers' pages go out to swap and the
    /// pressure eases, and letting every build go at once then would only bring it back.
    /// Measured 20 Nov 2026: swap used stays near full long after the pressure has gone, and
    /// `memory_pressure` said 30% free while the Mac thrashed; the level read 2.
    pub(crate) fn loaded(&mut self, short: Option<checks_runner::Short>, now: Instant) -> bool {
        if matches!(short, Some(checks_runner::Short::Memory { .. })) {
            self.pressed = Some(now);
        }
        short.is_some() || self.pressed.is_some_and(|at| now.saturating_duration_since(at) < TURN)
    }

    /// The Jobs whose builds are held now: told when they join this, and again when they leave it.
    pub(crate) fn jobs_held(&self) -> BTreeSet<String> {
        self.held.values().cloned().collect()
    }

    /// What each Job's compilers in `listing` should be sent, the load `loaded` or not.
    ///
    /// `in_a_check` says whether a process runs inside a Check, which is never held; `of_job`
    /// whether a process outside every Job's tree carries that Job's id. **A stopped compiler of a
    /// Job that should run is continued even where this Fleet did not stop it**, so a Fleet that
    /// ended while holding builds lets them go at its next start.
    pub(crate) fn look(
        &mut self,
        loaded: bool,
        contenders: &[Contender],
        listing: &str,
        in_a_check: impl Fn(u32) -> bool,
        of_job: impl Fn(u32, &str) -> bool,
        now: Instant,
        turn: Duration,
    ) -> Vec<Signal> {
        let rows = rows(listing);
        let parents: HashMap<u32, u32> = rows.iter().map(|row| (row.pid, row.ppid)).collect();
        let roots: HashMap<u32, &str> = contenders
            .iter()
            .flat_map(|one| one.roots.iter().map(move |root| (*root, one.job.as_str())))
            .collect();
        // A cargo an earlier Fleet stopped, as this one never stops one.
        let mut said: Vec<Signal> = rows
            .iter()
            .filter(|row| row.driver && row.stopped && by_descent(row.pid, &parents, &roots).is_some())
            .map(|row| Signal::Continue(row.pid))
            .collect();
        let compilers: Vec<(&Row, &str)> = rows
            .iter()
            .filter(|row| row.compiler)
            .filter_map(|row| {
                let job = by_descent(row.pid, &parents, &roots).or_else(|| {
                    contenders.iter().map(|one| one.job.as_str()).find(|job| of_job(row.pid, job))
                })?;
                (!in_a_check(row.pid)).then_some((row, job))
            })
            .collect();
        let building: BTreeSet<&str> = compilers.iter().map(|(_, job)| *job).collect();

        self.leader = if loaded && building.len() > 1 { self.next(contenders, &building, now, turn) } else { None };
        let leader = self.leader().map(str::to_string);
        let alive: BTreeSet<u32> = compilers.iter().map(|(row, _)| row.pid).collect();
        self.held.retain(|pid, _| alive.contains(pid));

        for (row, job) in compilers {
            let runs = leader.as_deref().is_none_or(|leader| leader == job);
            if runs {
                self.held.remove(&row.pid);
                if row.stopped {
                    said.push(Signal::Continue(row.pid));
                }
            } else {
                self.held.insert(row.pid, job.to_string());
                if !row.stopped {
                    said.push(Signal::Stop(row.pid));
                }
            }
        }
        said
    }

    /// Every process this Fleet holds, given up: what a Fleet that stops sends `SIGCONT`.
    pub(crate) fn released(&mut self) -> Vec<Signal> {
        self.leader = None;
        std::mem::take(&mut self.held).into_keys().map(Signal::Continue).collect()
    }

    /// Who leads now: the prioritised Job while it builds; else the leader until its turn is
    /// up or it stops building; else the next building Job after it in `contenders`, wrapping
    /// round to the oldest.
    fn next(&self, contenders: &[Contender], building: &BTreeSet<&str>, now: Instant, turn: Duration) -> Option<(String, Instant)> {
        if let Some(first) = contenders.iter().find(|one| one.prioritised && building.contains(one.job.as_str())) {
            return Some(match &self.leader {
                Some((job, since)) if *job == first.job => (job.clone(), *since),
                _ => (first.job.clone(), now),
            });
        }
        if let Some((job, since)) = &self.leader {
            if building.contains(job.as_str()) && now.saturating_duration_since(*since) < turn {
                return Some((job.clone(), *since));
            }
        }
        let after = self
            .leader
            .as_ref()
            .and_then(|(job, _)| contenders.iter().position(|one| one.job == *job))
            .map_or(0, |at| at + 1);
        (0..contenders.len())
            .map(|step| &contenders[(after + step) % contenders.len()])
            .find(|one| building.contains(one.job.as_str()))
            .map(|one| (one.job.clone(), now))
    }
}

/// The Job whose root `pid` descends from, walking parents.
fn by_descent<'a>(pid: u32, parents: &HashMap<u32, u32>, roots: &HashMap<u32, &'a str>) -> Option<&'a str> {
    let mut at = pid;
    for _ in 0..DEEPEST {
        if let Some(job) = roots.get(&at) {
            return Some(job);
        }
        at = *parents.get(&at)?;
        if at <= 1 {
            return None;
        }
    }
    None
}

/// Send each signal. A process already gone is not an error: it ended between the listing and
/// now, which is the ordinary case.
#[allow(unsafe_code)]
pub(crate) fn send(signals: &[Signal]) {
    for signal in signals {
        let (pid, which) = match *signal {
            Signal::Stop(pid) => (pid, libc::SIGSTOP),
            Signal::Continue(pid) => (pid, libc::SIGCONT),
        };
        let Ok(pid) = libc::pid_t::try_from(pid) else { continue };
        if pid <= 1 {
            continue;
        }
        // SAFETY: `kill` is a plain system call over two integers. The pid was read from the
        // process table a moment ago as a compiler in a Job's tree, and STOP and CONT end
        // nothing, so a pid reused in between is paused or woken, never lost.
        unsafe {
            libc::kill(pid, which);
        }
    }
}

#[cfg(test)]
pub(crate) fn rows_for_test(listing: &str) -> Vec<(u32, u32, bool, bool)> {
    rows(listing).into_iter().map(|row| (row.pid, row.ppid, row.stopped, row.compiler)).collect()
}
