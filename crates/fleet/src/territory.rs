//! What a repository holds and where each agent has changed it, for the flight deck's map.
//! `docs/concepts/fleet.md`, *The territory*.
//!
//! **Git answers and Fleet keeps the answer a while.** The directories are kept against the base head
//! they were read at, so a read at an unchanged head asks git for the head alone. The touches are
//! kept [`KEPT_FOR`] as a whole. Memory only: a restart reads again.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::territory::{self as git, Directory};
use api::Refusal;
use ipc::{
    AreaKid, AreasOfRepository, HolderTouches, ManifestId, RepositoryArea, RepositoryAreas, SlotHolderKind, SlotHolding,
    SlotTouches, TouchesInRepository,
};
use tokio::time::Instant;

use crate::daemon::Fleet;

/// How many directories a repository is listed with, counting each top directory and each inside one.
const MOST_AREAS: usize = 40;

/// How many file names a directory is listed with. Every read at an active head sends them all, so
/// this keeps this repository's answer near 64 KB; the directory's `files` stays exact past it.
const MOST_NAMES: usize = 100;

/// How many paths a holder is listed with.
const MOST_PATHS: usize = 200;

/// How long the touches of every slot are kept. Bridge asks every 35 seconds.
pub const KEPT_FOR: Duration = Duration::from_secs(20);

struct Tree {
    head: String,
    areas: Vec<RepositoryArea>,
    omitted: u32,
}

#[derive(Default)]
pub(crate) struct Territory {
    trees: Mutex<BTreeMap<String, Tree>>,
    touched: Mutex<Option<(Instant, SlotTouches)>>,
    /// One read of the touches at a time, so a second asker finds the first's answer kept.
    reading: tokio::sync::Mutex<()>,
    tree_reads: AtomicU64,
    slot_reads: AtomicU64,
}

impl Territory {
    /// How many times git was asked for a tree. A read at a kept head is not one.
    #[cfg(test)]
    pub(crate) fn tree_reads(&self) -> u64 {
        self.tree_reads.load(Ordering::Relaxed)
    }

    /// How many slots git was asked about.
    #[cfg(test)]
    pub(crate) fn slot_reads(&self) -> u64 {
        self.slot_reads.load(Ordering::Relaxed)
    }
}

/// The largest directories, a top directory with the ones inside it that made the list, and how many
/// were left out. **A top directory holds at least as many files as any directory inside it and sorts
/// before it on a tie**, so a directory is listed only where its top directory is.
pub(crate) fn capped(mut directories: Vec<Directory>) -> (Vec<RepositoryArea>, u32) {
    directories.sort_by(|one, other| other.files.cmp(&one.files).then_with(|| one.path.cmp(&other.path)));
    let omitted = directories.len().saturating_sub(MOST_AREAS) as u32;
    let mut tops: Vec<RepositoryArea> = Vec::new();
    for directory in directories.into_iter().take(MOST_AREAS) {
        match directory.path.split_once('/') {
            None => tops.push(RepositoryArea {
                dir: directory.path,
                files: directory.files,
                kids: Vec::new(),
                names: directory.names,
            }),
            Some((top, name)) => {
                if let Some(area) = tops.iter_mut().find(|area| area.dir == top) {
                    area.kids.push(AreaKid { name: name.to_string(), files: directory.files, names: directory.names });
                }
            }
        }
    }
    // A top with kids leaves naming to them, so no file is named twice.
    for area in tops.iter_mut().filter(|area| !area.kids.is_empty()) {
        area.names.clear();
    }
    (tops, omitted)
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
    /// Every served repository's directories, read at its base's head. One git cannot name a head
    /// for is left out.
    pub(crate) async fn repository_areas(&self) -> Result<RepositoryAreas, Refusal> {
        let mut repositories = Vec::new();
        for served in self.repositories().served() {
            let manifest = served.manifest().id().as_str().to_string();
            let root = PathBuf::from(served.root());
            let base = served.manifest().base().unwrap_or("main").to_string();
            let held = Arc::clone(self.territory());
            let read = tokio::task::spawn_blocking({
                let manifest = manifest.clone();
                move || held.tree_of(&manifest, &root, &base)
            })
            .await;
            if let Ok(Some((head, areas, omitted))) = read {
                repositories.push(AreasOfRepository {
                    manifest_id: ManifestId::carried(manifest),
                    root: served.root().to_string(),
                    head,
                    areas,
                    omitted,
                });
            }
        }
        Ok(RepositoryAreas { repositories })
    }

    /// What each Job or hosted Session holding a slot has changed in it. The slots and their holders
    /// are `list_worktrees`' answer, so this adds no pool read of its own.
    pub(crate) async fn slot_touches(&self) -> Result<SlotTouches, Refusal> {
        if let Some(kept) = self.territory().kept() {
            return Ok(kept);
        }
        let _one_at_a_time = self.territory().reading.lock().await;
        if let Some(kept) = self.territory().kept() {
            return Ok(kept);
        }
        let held = self.worktrees_answer().await?;
        let reads: Vec<Read> = held
            .slots
            .iter()
            .filter_map(|slot| {
                let (kind, id) = match &slot.held {
                    SlotHolding::Job { job_id, kept: None, completed: false, .. } => {
                        (SlotHolderKind::Job, job_id.as_str().to_string())
                    }
                    SlotHolding::HostedSession { session_id, .. } => (SlotHolderKind::Session, session_id.clone()),
                    _ => return None,
                };
                Some(Read {
                    manifest: slot.manifest_id.as_str().to_string(),
                    kind,
                    id,
                    slot: slot.slot,
                    path: PathBuf::from(&slot.path),
                    base: slot.base.clone(),
                })
            })
            .collect();
        let territory = Arc::clone(self.territory());
        let touches = tokio::task::spawn_blocking(move || territory.touches_of(reads))
            .await
            .unwrap_or(SlotTouches { repositories: Vec::new() });
        self.territory().keep(touches.clone());
        Ok(touches)
    }
}

/// One slot to read and who holds it.
struct Read {
    manifest: String,
    kind: SlotHolderKind,
    id: String,
    slot: u32,
    path: PathBuf,
    base: String,
}

impl Territory {
    /// The repository's directories at its base's head: kept where the head is the one they were read
    /// at, otherwise read.
    fn tree_of(&self, manifest: &str, root: &std::path::Path, base: &str) -> Option<(String, Vec<RepositoryArea>, u32)> {
        let head = git::head_of(root, base).ok()?;
        {
            let trees = self.trees.lock().expect("not poisoned");
            if let Some(kept) = trees.get(manifest).filter(|kept| kept.head == head) {
                return Some((head, kept.areas.clone(), kept.omitted));
            }
        }
        self.tree_reads.fetch_add(1, Ordering::Relaxed);
        let (areas, omitted) = capped(git::directories_at(root, &head, MOST_NAMES).ok()?);
        self.trees
            .lock()
            .expect("not poisoned")
            .insert(manifest.to_string(), Tree { head: head.clone(), areas: areas.clone(), omitted });
        Some((head, areas, omitted))
    }

    /// What each slot's holder changed, by repository. A slot git cannot read is left out.
    fn touches_of(&self, reads: Vec<Read>) -> SlotTouches {
        let mut by_repository: BTreeMap<String, Vec<HolderTouches>> = BTreeMap::new();
        for read in reads {
            self.slot_reads.fetch_add(1, Ordering::Relaxed);
            let Ok(mut paths) = git::changed_since(&read.path, &read.base) else {
                continue;
            };
            if paths.is_empty() {
                continue;
            }
            let omitted = paths.len().saturating_sub(MOST_PATHS) as u32;
            paths.truncate(MOST_PATHS);
            by_repository.entry(read.manifest).or_default().push(HolderTouches {
                kind: read.kind,
                id: read.id,
                slot: read.slot,
                paths,
                omitted,
            });
        }
        SlotTouches {
            repositories: by_repository
                .into_iter()
                .map(|(manifest, holders)| TouchesInRepository { manifest_id: ManifestId::carried(manifest), holders })
                .collect(),
        }
    }

    fn kept(&self) -> Option<SlotTouches> {
        let touched = self.touched.lock().expect("not poisoned");
        touched.as_ref().filter(|(at, _)| at.elapsed() < KEPT_FOR).map(|(_, kept)| kept.clone())
    }

    fn keep(&self, touches: SlotTouches) {
        *self.touched.lock().expect("not poisoned") = Some((Instant::now(), touches));
    }
}
