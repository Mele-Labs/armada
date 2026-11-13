//! The build-output sweep. **Every minute** Fleet reads the free space under each
//! repository it serves, and under `limits.diskReclaimFloorGib` drops build output
//! in a fixed order until there is room; **every interval** it also trims every
//! build folder of every checkout and sheds free slots' builds by age. The rules
//! are `adapters::leasing::trim_target`'s, the function a release calls, and
//! `adapters::leasing::disk`'s order. The policy is the owner's of 13 Nov 2026,
//! `.armada/documents/disk-reclaim-policy.html`.

use std::path::{Path, PathBuf};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use adapter_traits::{Delivery, Vcs, WorkProduct, AgentHarness};
use adapters::leasing::disk::{inventory, mark_landed, reclaim, shed_free_slots, Reclaimed};
use adapters::leasing::{sweep_repository, Trim, Trimmed};
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::headroom::{Bytes, Machine};

/// How often free space is read. It is `df`, and cheap.
const LOOK_EVERY: Duration = Duration::from_secs(60);

/// What one look covers: each served root with the branch its slots lease from, and the free
/// space under which to reclaim (zero never reclaims).
pub struct Reach {
    pub roots: Vec<(PathBuf, String)>,
    pub floor: u64,
}

/// How many build folders a look keeps of what a pass would drop next.
const NEXT_KEPT: usize = 10;

/// What one look did and found.
pub(crate) struct Looked {
    /// What was removed whole, and what a pass came to, each as a line.
    pub lines: Vec<String>,
    /// Each root's disk after the look.
    pub seen: Vec<Seen>,
}

/// One root's disk after a look.
pub(crate) struct Seen {
    pub root: PathBuf,
    /// Free bytes after any reclaim. `None` where the volume would not answer.
    pub free: Option<u64>,
    /// What the volume gained over a pass, where one dropped anything.
    pub freed: Option<u64>,
    /// What a pass would drop next, at most [`NEXT_KEPT`]: empty unless free is still under the
    /// reclaim floor.
    pub next: Vec<PathBuf>,
}

/// What the last looks left for the wire: the last pass that dropped anything, and what the one
/// after it would drop. Set by the sweep, read by `get_health`.
#[derive(Default)]
pub(crate) struct DiskState {
    pub last_pass: Option<(String, u64)>,
    pub next: Vec<PathBuf>,
}

/// One pass under pressure over `root`, whose slots lease from `base`: nothing unless free is
/// under `floor`, and a floor of zero is off. `armada disk reclaim` runs it with no Fleet.
pub fn reclaim_root(root: &Path, base: &str, floor: u64, free_at: &dyn Fn(&Path) -> Option<u64>, said: &mut dyn FnMut(&str)) -> Option<Reclaimed> {
    if floor == 0 || !free_at(root).is_some_and(|free| free < floor) {
        return None;
    }
    let mut folders = inventory(root, base, &[]);
    mark_landed(&mut folders, base);
    reclaim(&folders, SystemTime::now(), floor, &mut || free_at(root), said)
}

/// One look. `free_at` reads the free bytes on the volume holding a path.
pub(crate) fn look(reach: &Reach, trim_due: bool, trim: Trim, free_at: &dyn Fn(&Path) -> Option<u64>) -> Looked {
    let mut lines = Vec::new();
    let mut seen = Vec::new();
    for (root, base) in &reach.roots {
        let passed = reclaim_root(root, base, reach.floor, free_at, &mut |line| lines.push(line.to_string()));
        if trim_due {
            shed_free_slots(&inventory(root, base, &[]), SystemTime::now(), &mut |line| {
                lines.push(line.to_string());
            });
            lines.extend(
                sweep_repository(root, trim, SystemTime::now(), &[])
                    .into_iter()
                    .filter(|(_, done)| *done == Trimmed::Removed)
                    .map(|(target, _)| {
                        format!("{} was over its size ceiling and was removed", target.display())
                    }),
            );
        }
        let free = free_at(root);
        let next = match &passed {
            Some(passed) if free.is_some_and(|free| free < reach.floor) => passed.left.iter().take(NEXT_KEPT).cloned().collect(),
            _ => Vec::new(),
        };
        seen.push(Seen { root: root.clone(), free, freed: passed.as_ref().filter(|passed| passed.acts > 0).map(|passed| passed.freed), next });
    }
    Looked { lines, seen }
}

/// Look every minute, trim every `every` (to the nearest minute), and hand `looked` what each look
/// did, whose lines say what was removed whole, since each costs a cold rebuild. **The first look is at start**, so a
/// Fleet started on a short disk reclaims before its admission floor holds every Job a minute.
fn keep_sweeping<F: Future<Output = ()> + Send>(
    reach: impl Fn() -> Reach + Send + 'static,
    machine: Arc<dyn Machine>,
    every: Duration,
    trim: Trim,
    looked: impl Fn(Looked) -> F + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(LOOK_EVERY);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut trimmed = Instant::now();
        loop {
            ticker.tick().await;
            let trim_due = trimmed.elapsed() >= every;
            if trim_due {
                trimmed = Instant::now();
            }
            let reach = reach();
            let machine = Arc::clone(&machine);
            let Ok(done) = tokio::task::spawn_blocking(move || {
                look(&reach, trim_due, trim, &|path| machine.disk_free_at(path).map(|free| free.count()))
            })
            .await
            else {
                return;
            };
            looked(done).await;
        }
    })
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
    /// Sweep every served repository, as the module says. **The floor is read live**, each look,
    /// from settings.json as Fleet holds it, so a save applies within a minute.
    pub fn keep_sweeping(
        self: &Arc<Self>,
        every: Duration,
        trim: Trim,
        said: impl Fn(&str) + Send + 'static,
    ) -> JoinHandle<()> {
        let fleet = Arc::clone(self);
        let machine = Arc::clone(self.machine());
        let after = Arc::clone(self);
        keep_sweeping(move || fleet.sweep_reach(), machine, every, trim, move |done| {
            for line in &done.lines {
                said(line);
            }
            let fleet = Arc::clone(&after);
            async move { fleet.disk_looked(&done).await }
        })
    }

    /// What a look found: kept for `get_health`, and, where a repository's disk is under the hold
    /// floor, said once in Your turn (`crate::operator`'s disk card).
    pub(crate) async fn disk_looked(self: &Arc<Self>, looked: &Looked) {
        let first = self.repositories().served().first().map(|served| PathBuf::from(served.root()));
        if let Some(seen) = first.and_then(|root| looked.seen.iter().find(|seen| seen.root == root)) {
            let mut state = self.disk_state().lock().expect("the disk lock is not poisoned");
            if let Some(freed) = seen.freed {
                state.last_pass = Some((self.now().as_str().to_string(), freed));
            }
            state.next = seen.next.clone();
        }
        for seen in &looked.seen {
            self.disk_card_for(seen).await;
        }
    }

    /// `limits.diskReclaimFloorGib` in bytes, as settings.json holds it now. Zero is off.
    pub(crate) fn reclaim_floor(&self) -> u64 {
        let floor = self.machine_settings().now().get(config::settings::DISK_RECLAIM_FLOOR_GIB);
        Bytes::gibibytes(u64::try_from(floor).unwrap_or(0)).count()
    }

    fn sweep_reach(&self) -> Reach {
        Reach {
            floor: self.reclaim_floor(),
            roots: self
                .repositories()
                .every()
                .iter()
                .map(|repository| {
                    let base = repository.manifest().and_then(|manifest| manifest.base()).unwrap_or("main");
                    (PathBuf::from(repository.root()), base.to_string())
                })
                .collect(),
        }
    }
}
