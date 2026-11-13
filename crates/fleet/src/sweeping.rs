//! The build-output sweep. **Every minute** Fleet reads the free space under each
//! repository it serves, and under `limits.diskReclaimFloorGib` drops build output
//! in a fixed order until there is room; **every interval** it also trims every
//! build folder of every checkout and sheds free slots' builds by age. The rules
//! are `adapters::leasing::trim_target`'s, the function a release calls, and
//! `adapters::leasing::disk`'s order. The policy is the owner's of 13 Nov 2026,
//! `.armada/documents/disk-reclaim-policy.html`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use adapter_traits::{Delivery, Vcs, WorkProduct, AgentHarness};
use adapters::leasing::disk::{inventory, mark_landed, reclaim, shed_free_slots};
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

/// The lines for what one look did. `free_at` reads the free bytes on the volume holding a path.
pub(crate) fn look(reach: &Reach, trim_due: bool, trim: Trim, free_at: &dyn Fn(&Path) -> Option<u64>) -> Vec<String> {
    let mut lines = Vec::new();
    for (root, base) in &reach.roots {
        let short = reach.floor > 0 && free_at(root).is_some_and(|free| free < reach.floor);
        if short {
            let mut folders = inventory(root, base, &[]);
            mark_landed(&mut folders, base);
            reclaim(
                &folders,
                SystemTime::now(),
                reach.floor,
                &mut || free_at(root),
                &mut |line| lines.push(line.to_string()),
            );
        }
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
    }
    lines
}

/// Look every minute, trim every `every` (to the nearest minute), and hand `said` the lines of
/// what was removed whole, since each costs a cold rebuild. **The first look is at start**, so a
/// Fleet started on a short disk reclaims before its admission floor holds every Job a minute.
fn keep_sweeping(
    reach: impl Fn() -> Reach + Send + 'static,
    machine: Arc<dyn Machine>,
    every: Duration,
    trim: Trim,
    said: impl Fn(&str) + Send + 'static,
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
            let Ok(lines) = tokio::task::spawn_blocking(move || {
                look(&reach, trim_due, trim, &|path| machine.disk_free_at(path).map(|free| free.count()))
            })
            .await
            else {
                return;
            };
            for line in lines {
                said(&line);
            }
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
        keep_sweeping(move || fleet.sweep_reach(), machine, every, trim, said)
    }

    fn sweep_reach(&self) -> Reach {
        let floor = self.machine_settings().now().get(config::settings::DISK_RECLAIM_FLOOR_GIB);
        Reach {
            floor: Bytes::gibibytes(u64::try_from(floor).unwrap_or(0)).count(),
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
