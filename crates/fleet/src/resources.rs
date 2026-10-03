//! Reading the machine for one Job: the processes it holds, what they are
//! burning, the disk its worktree has taken, and when anything last moved.
//!
//! **Read on demand and never on the turn loop.** Fleet turns every 250ms, and
//! a process table plus a directory walk per Job per turn is a cost paid
//! constantly to answer a question asked rarely. [`crate::footprint`] pays a
//! throttled version of that bill because a live file list is what a person
//! watches; nobody watches a memory figure change.
//!
//! **What counts as this Job's is written down here**, because otherwise the
//! figure has no referent. It is the process Fleet recorded at the spawn and
//! everything descended from it — so a Drone's own session counts, and so does
//! a build it started. A Check that Fleet ran, and a preparation command Fleet
//! ran, are Fleet's children rather than the Drone's and are **not** counted.
//! `Held::None` is the honest answer during those spans, and it is a stated
//! answer rather than an empty list.
//!
//! **The base checkout is not this Job's disk**, by that same rule. One
//! checkout serves every Job on a commit, so charging it to whichever Job was
//! asked about would report a figure four Jobs each claim in full — and killing
//! that Job would free none of it. `crate::basing` gives it back.
//!
//! **Both readings are bounded and both shell out**, which is
//! [`crate::headroom`]'s precedent and its argument: `ps` and `du` are one
//! spelling on darwin and Linux, need no `unsafe` and no platform crate.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, PoisonError};
use std::time::{Duration, UNIX_EPOCH};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId, JobStatus, Timestamp};
use tokio::process::Command;

use crate::adrift::Adrift;
use crate::clock::rfc3339_utc;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder, StartedAt};
use crate::transcript::log_of;

/// A Job's log files, and which of them a writer holds.
pub(crate) mod logs;

/// How long a reading of the machine may take before it is given up on.
///
/// **A bound on the answer, not on the walk.** A 1.0 GB worktree is the case
/// this exists for and `du` on one is seconds, not milliseconds; Bridge's own
/// command timeout is five, and an act a person presses when they are already
/// worried must not be the thing that then hangs. What crosses when the bound
/// is spent is the figure's absence, said — `WorktreeOnDisk::bytes`.
///
/// **`kill_on_drop` is right here and was wrong in `#428`.** There the timeout
/// bounded a request and the child was the work, so killing it destroyed an
/// install nobody had finished. Here the child *is* the measurement, and one
/// that outlived its answer would be a `du` per press with nothing reading it.
pub(crate) const LOOK: Duration = Duration::from_secs(3);

/// How long a worktree's size is served before it is walked again: `du` on a
/// 1–3 GB worktree is 1.1–1.9 s cold, and Bridge reads this every 10 s.
pub(crate) const SIZED_FOR: Duration = Duration::from_secs(30);

/// The last size each worktree walked to, and when, by path. Never written
/// down: a size read back after a restart would be as old as the restart.
#[derive(Default)]
pub(crate) struct Sizes {
    by_path: std::sync::Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<Option<(Timestamp, u64)>>>>>,
}

impl Sizes {
    /// The size kept for `path` and when it was walked, if that is inside
    /// [`SIZED_FOR`], or what `walk` finds now. A walk that finds nothing is
    /// not kept, so the next read walks.
    pub(crate) async fn of<F, Walk>(
        &self,
        path: &str,
        now: &Timestamp,
        walk: F,
    ) -> Option<(Timestamp, u64)>
    where
        F: FnOnce() -> Walk,
        Walk: Future<Output = Option<u64>>,
    {
        let kept = self
            .by_path
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(path.to_string())
            .or_default()
            .clone();
        // Held across the walk, so a second read of this path waits for the
        // first one's `du` rather than starting its own.
        let mut kept = kept.lock().await;
        if let Some((at, _)) = &*kept {
            if elapsed(at, now) < SIZED_FOR {
                return kept.clone();
            }
        }
        let bytes = walk().await?;
        *kept = Some((now.clone(), bytes));
        kept.clone()
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// What this Job holds on this machine, now.
    ///
    /// **Nothing here refuses on a reading that will not come.** A `ps` that
    /// will not run, a `du` that runs long and a log that is not there are all
    /// answered as the absence they are, because an examination that 500s is
    /// one a person presses once and never again. The one refusal is the store
    /// declining to say what process it recorded, which is the daemon being
    /// unable to read its own record.
    pub(crate) async fn job_resources(&self, job: &Job) -> Result<ipc::JobResources, Adrift> {
        let recorded = self
            .store()
            .lock()
            .await
            .drone_process(job.id())
            .map_err(Adrift::Reading)?;
        let (held, processes) = match &recorded {
            None => (ipc::Held::None, Vec::new()),
            Some(process) => match holder_of(process.pid) {
                Err(_) => (ipc::Held::Unreadable, Vec::new()),
                Ok(Holder::Vacant) => (ipc::Held::Gone, Vec::new()),
                Ok(Holder::Held(started)) if started.as_str() != process.started_at => {
                    (ipc::Held::Replaced, Vec::new())
                }
                Ok(Holder::Held(_)) => (ipc::Held::Running, self.below(process.pid).await),
            },
        };
        let kept = recorded
            .as_ref()
            .map(|process| ipc::DroneId::from(&process.drone_id));
        let mut processes: Vec<ipc::JobProcess> = processes
            .into_iter()
            .map(|row| ipc::JobProcess {
                drone_id: kept.clone(),
                ..row
            })
            .collect();
        processes.extend(self.crew_processes(job.id()).await);
        let records_root = self.served_by(job)?.records_root().to_string();
        let tree: Vec<u32> = processes.iter().map(|process| process.pid).collect();
        Ok(ipc::JobResources {
            job_id: job.id().into(),
            read_at: (&self.now()).into(),
            held,
            worktree: self.sized_worktree(job).await,
            wrote_last_at: wrote_last(&log_of(&records_root, &job.handle())),
            logs: logs::of(&records_root, &job.handle(), &tree).await,
            processes,
        })
    }

    /// The recorded process and everything under it, that one first.
    ///
    /// One `ps` over the whole table rather than a walk per generation: the
    /// tree is built here from parent ids, so the number of processes Armada
    /// spawns has no bearing on the number of children Fleet does.
    /// Each Drone beside the kept one, and what it started, each row naming it.
    async fn crew_processes(&self, job: &JobId) -> Vec<ipc::JobProcess> {
        let crew: Vec<(core_model::DroneId, u32)> = self
            .crew_at_work()
            .into_iter()
            .filter(|(of, _, _)| of == job)
            .map(|(_, drone, pid)| (drone, pid))
            .collect();
        if crew.is_empty() {
            return Vec::new();
        }
        let Some(said) = table().await else {
            return Vec::new();
        };
        crew.iter()
            .flat_map(|(drone, pid)| {
                descended(&said, *pid)
                    .into_iter()
                    .map(move |row| ipc::JobProcess {
                        drone_id: Some(drone.into()),
                        ..row
                    })
            })
            .collect()
    }

    /// Where `pid` is one of this Job's Drones beside the kept one, or under
    /// one: the Drone, and whether it is the Drone's own process, with every
    /// pid under `pid`, `pid` first.
    pub(crate) async fn beside_under(
        &self,
        job: &JobId,
        pid: u32,
    ) -> Option<(core_model::DroneId, bool, Vec<u32>)> {
        let crew: Vec<(core_model::DroneId, u32)> = self
            .crew_at_work()
            .into_iter()
            .filter(|(of, _, _)| of == job)
            .map(|(_, drone, root)| (drone, root))
            .collect();
        if crew.is_empty() {
            return None;
        }
        let said = table().await?;
        crew.into_iter().find_map(|(drone, root)| {
            descended(&said, root)
                .iter()
                .any(|one| one.pid == pid)
                .then(|| {
                    let under = descended(&said, pid).iter().map(|one| one.pid).collect();
                    (drone, root == pid, under)
                })
        })
    }

    async fn below(&self, root: u32) -> Vec<ipc::JobProcess> {
        // A process table that will not read is an empty list beside a `held`
        // that says the pid is alive, which is the one shape a surface must
        // draw as a question rather than as an answer.
        table()
            .await
            .map(|said| descended(&said, root))
            .unwrap_or_default()
    }

    /// The Job's process tree, read now for an act on it rather than for a
    /// screen: [`job_resources`](Fleet::job_resources)'s reading, with
    /// everything but `held: running` answered as no tree at all.
    ///
    /// **An act needs the proof a screen can do without.** A gone, replaced or
    /// unreadable Drone has no tree a pid can be found in, so every pid is
    /// refused against it — which is the point: what a kill may reach is what
    /// this finds, never what the renderer named.
    pub(crate) async fn tree_now(&self, job: &JobId) -> Result<Option<Tree>, Adrift> {
        let recorded = self
            .store()
            .lock()
            .await
            .drone_process(job)
            .map_err(Adrift::Reading)?;
        let Some(recorded) = recorded else {
            return Ok(None);
        };
        let started = match holder_of(recorded.pid) {
            Ok(Holder::Held(started)) if started.as_str() == recorded.started_at => started,
            _ => return Ok(None),
        };
        Ok(table().await.map(|said| Tree {
            root: recorded.pid,
            started,
            said,
        }))
    }

    /// The Job's checkout and what it has taken, or nothing where there is no
    /// checkout to read.
    async fn sized_worktree(&self, job: &Job) -> Option<ipc::WorktreeOnDisk> {
        let worktree = self.worktree_of(job).ok().flatten()?;
        let size = self
            .sizes()
            .of(worktree.path(), &self.now(), || taken(worktree.path()))
            .await;
        Some(ipc::WorktreeOnDisk {
            measured_at: size.as_ref().map(|(at, _)| at.into()),
            bytes: size.map(|(_, bytes)| bytes),
            path: worktree.path().to_string(),
            branch: worktree.branch().to_string(),
        })
    }
}

/// One `ps` over the whole table, or nothing where it did not answer inside
/// [`LOOK`].
async fn table() -> Option<String> {
    let said = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,pcpu=,rss=,etime=,comm="])
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(LOOK, said).await {
        Ok(Ok(out)) if out.status.success() => {
            Some(String::from_utf8_lossy(&out.stdout).into_owned())
        }
        _ => None,
    }
}

/// A Job's process tree at one instant: the recorded Drone, proved to be the
/// process that was recorded, and the one table everything under it is cut
/// from.
pub(crate) struct Tree {
    /// The Drone Fleet recorded at the spawn.
    pub(crate) root: u32,
    /// When the process at `root` started, which matched the record.
    started: StartedAt,
    said: String,
}

impl Tree {
    /// `pid` and everything under it, parents before children — or nothing,
    /// where `pid` is not in this Job's tree.
    pub(crate) fn under(&self, pid: u32) -> Vec<u32> {
        let held = descended(&self.said, self.root);
        if !held.iter().any(|one| one.pid == pid) {
            return Vec::new();
        }
        descended(&self.said, pid)
            .iter()
            .map(|one| one.pid)
            .collect()
    }

    /// Whether the process at `root` is still the Drone this tree was read
    /// under: alive, and started when it did.
    pub(crate) fn root_remains(&self) -> bool {
        matches!(holder_of(self.root), Ok(Holder::Held(now)) if now == self.started)
    }
}

/// What one directory holds, in bytes, or nothing where the walk did not finish
/// inside [`LOOK`].
///
/// `du -s -k` is the same two flags on both platforms and fixes the block size
/// at 1024, which darwin otherwise takes from the environment.
async fn taken(path: &str) -> Option<u64> {
    let said = Command::new("du")
        .args(["-s", "-k"])
        .arg(path)
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(LOOK, said).await.ok()?.ok()?;
    // `du` exits non-zero on a directory it could not descend into and still
    // prints a total for what it did read, so the reading is taken off stdout
    // rather than gated on the status.
    measured(&String::from_utf8_lossy(&out.stdout))
}

/// The total off `du -s -k`, in bytes.
pub(crate) fn measured(said: &str) -> Option<u64> {
    let last = said.lines().rfind(|line| !line.trim().is_empty())?;
    let kibibytes: u64 = last.split_whitespace().next()?.parse().ok()?;
    Some(kibibytes * 1024)
}

/// When a file was last written, as an instant.
///
/// **Not [`Clock`](crate::Clock)'s business and not a violation of it.** The
/// rule is that Fleet reads one clock; this reads a timestamp off a directory
/// entry, which is a fact about a file rather than what time it is.
fn wrote_last(at: &Path) -> Option<ipc::Instant> {
    let written = std::fs::metadata(at).ok()?.modified().ok()?;
    let millis = written.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64;
    Some(ipc::Instant::carried(rfc3339_utc(millis)))
}

/// One row of `ps -A -o pid=,ppid=,pcpu=,rss=,etime=,comm=`.
struct Row {
    pid: u32,
    parent: u32,
    cpu: f64,
    resident: u64,
    running_for: String,
    command: String,
}

/// `root` and everything descended from it, breadth-first, `root` first.
///
/// **Breadth-first rather than sorted**, so the shape of the tree survives into
/// the list: a Drone with three children reads as a Drone with three children,
/// and a build eight levels down reads as the deep thing it is.
pub(crate) fn descended(said: &str, root: u32) -> Vec<ipc::JobProcess> {
    let rows: Vec<Row> = said.lines().filter_map(row).collect();
    let mut taken: Vec<ipc::JobProcess> = Vec::new();
    let mut frontier = vec![root];
    while let Some(pid) = frontier.pop() {
        let Some(row) = rows.iter().find(|row| row.pid == pid) else {
            continue;
        };
        taken.push(ipc::JobProcess {
            pid: row.pid,
            command: row.command.clone(),
            cpu_percent: row.cpu,
            memory_bytes: row.resident * 1024,
            running_for: row.running_for.clone(),
            recorded: row.pid == root,
            drone_id: None,
        });
        // A process reparented to itself, or a table read mid-fork, could
        // otherwise walk forever. Nothing already taken is queued again.
        for child in rows.iter().filter(|other| other.parent == pid) {
            if child.pid != pid && !taken.iter().any(|seen| seen.pid == child.pid) {
                frontier.push(child.pid);
            }
        }
    }
    taken
}

/// One line, or nothing where it is short — `ps` walks a live table and a
/// process that exits mid-walk can leave a partial row.
///
/// `comm` is last because it is the only field that can hold a space, so the
/// rest are read by position and it takes what is left.
fn row(line: &str) -> Option<Row> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let parent = fields.next()?.parse().ok()?;
    let cpu = fields.next()?.parse().ok()?;
    let resident = fields.next()?.parse().ok()?;
    let running_for = fields.next()?.to_string();
    let command = fields.collect::<Vec<&str>>().join(" ");
    (!command.is_empty()).then_some(Row {
        pid,
        parent,
        cpu,
        resident,
        running_for,
        command,
    })
}

/// Whether this status expects a Drone to be at work on the machine.
///
/// **The whole of what makes "no process" a fault rather than a fact.** A Job
/// at its approval gate holds nothing and is right to; a Job that reads
/// `running` and holds nothing is the state the wedged Job of 4 Sep 2026 was
/// in, and nothing said so.
///
/// `piloted` is not among them: a person holds that worktree and whatever they
/// are running is theirs.
pub(crate) fn expects_a_drone(status: JobStatus) -> bool {
    matches!(status, JobStatus::Running)
}

/// How long ago, in whole seconds, or nothing where either instant will not
/// parse.
pub(crate) fn since(at: &ipc::Instant, now: &Timestamp) -> Option<u64> {
    let at = Timestamp::from_rfc3339(at.as_str()).epoch_millis()?;
    let now = now.epoch_millis()?;
    (now >= at).then(|| ((now - at) / 1_000) as u64)
}
