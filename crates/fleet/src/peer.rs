//! Which Drone is on the other end of a connection.
//!
//! **The Drone is not asked, because no scheme where it holds its own identity
//! survives.** `docs/spikes/011-what-can-one-drone-reach.md` measured all five
//! — a port, a socket path, a token in a file, in the environment, in argv —
//! against a Drone holding `Write` and a `cargo` rule, which together are
//! native code at the operator's uid. So the identity is the connection, which
//! `docs/spikes/010-can-a-drone-be-identified.md` told two Drones apart by, on
//! one listener and one config file.
//!
//! **The pair, never the port.** A local port is not unique on a host, so a
//! lookup keyed on the peer's port alone names the wrong pid whenever it meets
//! the impostor first — deterministically, not as a race, which is why a test
//! that passes against it proves nothing. Matching `insi_lport` *and*
//! `insi_fport` was right in every ordering.
//! `docs/spikes/012-peer-identity-under-concurrency.md` holds that
//! reproduction, the four routes it timed — `proc_pidfdinfo` at 22µs against
//! `lsof` at 64ms, and it is the only one that can match a pair — and the 384
//! connections engineered to lose their peer, of which **none came back naming
//! another process**.
//!
//! Absent is therefore the only failure, and [`NotACaller`] is a refusal rather
//! than a guess. It is why the lookup runs on the tool call rather than at
//! accept — a call being served is a connection that is open — and why spike
//! 10's `curl` bypass, from a pid Fleet did not spawn, matches nothing.

use std::collections::BTreeMap;

use api::Caller;
use core_model::{DroneId, JobId, TaskId};

/// Why a call could not be attributed to a Drone.
///
/// **One variant, because there is one answer.** Whether the peer exited, or was
/// never a Drone, or was a `curl` the Drone started, the fact Fleet can state is
/// the same: nothing it spawned holds that connection. Splitting it would invite
/// a caller to treat one of them as softer, and none of them is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotACaller;

impl std::fmt::Display for NotACaller {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(
            "this call did not arrive on a connection held by a Drone this Fleet started, \
             so there is no Job to record it against",
        )
    }
}

impl std::error::Error for NotACaller {}

/// Which process is working which Job.
///
/// **A second place a pid is held, and it earns its keep.** The pid is also
/// inside the working slot, on the [`DroneSession`](crate::DroneSession) — and
/// it cannot be read from there to answer this question, because reading it
/// means taking that slot's lock, and the lookup does not know yet *which* slot
/// to take. Awaiting each of them in turn would put one Drone's tool call behind
/// another Drone's `cargo nextest`, which is the single working slot arriving
/// back under a new name.
///
/// It is written where the record's own `assigned_drone` is written — the spawn
/// puts a row in, the departure takes it out — so the two cannot drift without
/// a departure having gone unrecorded, which `crate::boundary` already refuses
/// to let happen quietly.
#[derive(Debug, Default)]
/// A Job's kept Drone, and each beside it (slice 5), whose calls are named.
pub struct Drones {
    kept: BTreeMap<JobId, u32>,
    crew: BTreeMap<DroneId, (JobId, u32)>,
    /// The task each live task Drone is on, the kept one's included.
    tasks: BTreeMap<JobId, BTreeMap<DroneId, TaskId>>,
    /// Tasks a Drone beside the kept one did not hand in, which the kept
    /// Drone takes rather than another beside it.
    kept_only: std::collections::BTreeSet<(JobId, TaskId)>,
}

impl Drones {
    /// A Drone started on this Job, as this process.
    pub fn arrived(&mut self, job: &JobId, pid: u32) {
        self.kept.insert(job.clone(), pid);
    }

    /// The kept Drone on this Job has gone, and the task it was on is free.
    pub fn left(&mut self, job: &JobId) {
        self.kept.remove(job);
        if let Some(tasks) = self.tasks.get_mut(job) {
            let crew = &self.crew;
            tasks.retain(|drone, _| crew.contains_key(drone));
        }
    }

    /// A Drone started beside this Job's kept one, as this process.
    pub fn joined(&mut self, job: &JobId, drone: &DroneId, pid: u32) {
        self.crew.insert(drone.clone(), (job.clone(), pid));
    }

    /// A Drone beside a Job's kept one has gone.
    pub fn parted(&mut self, job: &JobId, drone: &DroneId) {
        self.crew.remove(drone);
        if let Some(tasks) = self.tasks.get_mut(job) {
            tasks.remove(drone);
        }
    }

    /// This Drone of this Job is on this task.
    pub fn on_task(&mut self, job: &JobId, drone: &DroneId, task: TaskId) {
        self.tasks
            .entry(job.clone())
            .or_default()
            .insert(drone.clone(), task);
    }

    /// This task waits for the Job's kept Drone.
    pub fn leave_for_kept(&mut self, job: &JobId, task: TaskId) {
        self.kept_only.insert((job.clone(), task));
    }

    /// Whether this task waits for the Job's kept Drone.
    pub fn left_for_kept(&self, job: &JobId, task: TaskId) -> bool {
        self.kept_only.contains(&(job.clone(), task))
    }

    /// Every task a live Drone of this Job is on.
    pub fn live_tasks(&self, job: &JobId) -> Vec<TaskId> {
        self.tasks
            .get(job)
            .map_or_else(Vec::new, |tasks| tasks.values().copied().collect())
    }

    /// The task a live Drone of this Job is on.
    pub fn task_of(&self, job: &JobId, drone: &DroneId) -> Option<TaskId> {
        self.tasks.get(job)?.get(drone).copied()
    }

    /// Every kept Drone this Fleet is holding, as pid and Job.
    pub fn each(&self) -> Vec<(JobId, u32)> {
        self.kept
            .iter()
            .map(|(job, pid)| (job.clone(), *pid))
            .collect()
    }

    /// Every Drone beside a kept one, as Job, Drone and pid.
    pub fn crew(&self) -> Vec<(JobId, DroneId, u32)> {
        self.crew
            .iter()
            .map(|(drone, (job, pid))| (job.clone(), drone.clone(), *pid))
            .collect()
    }
}

/// Whether a process holds a particular TCP connection.
///
/// **A trait so the matching can be tested against something other than the
/// kernel**, and implemented once — [`Kernel`] — because there is one right
/// answer and it is a syscall.
pub trait PeerOf: Send + Sync {
    /// Whether `pid` holds a TCP socket whose local port is `from` and whose
    /// foreign port is `to`.
    ///
    /// **Both, never one.** A local port is not unique on a host; see this
    /// module's header for the measurement, and for what matching on one alone
    /// gets wrong.
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool;

    /// The processes `pid` started that are still running. Empty on any
    /// failure, the direction [`held_within`] may fail in.
    fn children(&self, pid: u32) -> Vec<u32>;
}

/// How deep and how wide [`held_within`] walks. A Helm session's relay is its
/// agent's child; the bound is there so a runaway tree cannot stall a call.
const DEEPEST: usize = 6;
const MOST: usize = 256;

/// Whether a call arrived on a connection held by one of `roots` or by a
/// process one of them started. `#941`.
///
/// **Descendants, because a Helm session reaches the door through a relay its
/// agent started**, and that relay holds the socket. A process outside the tree
/// cannot join it: a parent is the kernel's record, not a process's claim.
pub fn held_within(caller: &Caller, served_on: u16, roots: &[u32], peers: &dyn PeerOf) -> bool {
    let Some(from) = caller.port() else {
        return false;
    };
    let mut seen: Vec<u32> = Vec::new();
    let mut level = roots.to_vec();
    for _ in 0..DEEPEST {
        let mut below = Vec::new();
        for pid in level {
            if pid == 0 || seen.contains(&pid) || seen.len() >= MOST {
                continue;
            }
            if peers.holds(pid, from, served_on) {
                return true;
            }
            seen.push(pid);
            below.extend(peers.children(pid));
        }
        if below.is_empty() {
            break;
        }
        level = below;
    }
    false
}

/// Which Job a call belongs to, over the Drones handed in.
///
/// `None` where no Drone holds that connection — see [`NotACaller`], and see
/// the header for why absent is the only failure this can have.
pub fn attributed(
    caller: &Caller,
    served_on: u16,
    drones: &[(JobId, u32)],
    peers: &dyn PeerOf,
) -> Option<JobId> {
    let from = caller.port()?;
    drones
        .iter()
        .find(|(_, pid)| peers.holds(*pid, from, served_on))
        .map(|(job, _)| job.clone())
}

pub use self::kernel::Kernel;

#[cfg(target_vendor = "apple")]
mod kernel;

#[cfg(not(target_vendor = "apple"))]
mod kernel {
    /// Nothing to ask. Armada is a macOS application — see
    /// `docs/contracts/system-architecture.md` — and this exists so the
    /// workspace still builds where somebody is reading it rather than running
    /// it. **It answers `false` to everything**, which makes every call
    /// unattributable, which is the refusal rather than a wrong Job.
    #[derive(Debug, Default)]
    pub struct Kernel;

    impl super::PeerOf for Kernel {
        fn holds(&self, _pid: u32, _from: u16, _to: u16) -> bool {
            false
        }

        fn children(&self, _pid: u32) -> Vec<u32> {
            Vec::new()
        }
    }
}
