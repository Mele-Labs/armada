//! Which slots of a pool changed, so a reader that keeps the pool's readings
//! reads those and leaves the rest alone.
//!
//! **What is watched, and what is not.** The repository's own git directory
//! (every slot's `HEAD` and `index` live under `worktrees/<name>/`, and the refs
//! every slot is measured against are shared), and the work tree of a slot
//! nobody holds, where an edit changes nothing under `.git`. A held slot's work
//! tree is not watched: its reading never asks git for status, and a build
//! there writes `target/` thousands of events a minute. Measured in
//! `docs/spikes/slot-events.md`.
//!
//! **Events are a hint, never the truth.** A holder that dies leaves no file
//! behind to notice, and a stream restarts as a slot is added to it, so every
//! reading is also read again when the reader says `full`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{SlotHeld, SlotReading};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::Pool;
use crate::worktree::CHANGES;

/// What this adapter has itself done to a slot, a checkout or a branch.
static ACTS: AtomicU64 = AtomicU64::new(0);

/// An act of this adapter: it does not wait to be told by the file system.
pub(crate) fn acted() {
    ACTS.fetch_add(1, Ordering::Release);
}

/// Directly under a slot, never a change to what the slot holds.
const NOT_WORK: &[&str] = &["target", ".git", ".gitnexus"];

/// Read by the thread notify delivers on, and by whoever reads the pool.
#[derive(Default)]
struct State {
    moves: Moves,
    /// A slot's git directory, and its number.
    gitdirs: Vec<(PathBuf, usize)>,
    /// A slot's work tree, where it is watched.
    trees: Vec<(PathBuf, usize)>,
    /// Slots whose work tree edits count.
    free: BTreeSet<usize>,
}

#[derive(Default)]
struct Moves {
    /// Something every slot is measured against moved, or events were lost.
    all: bool,
    slots: BTreeSet<usize>,
}

enum Touched {
    Nothing,
    All,
    Slot(usize),
}

impl State {
    fn classify(&self, common: &Path, base: &str, path: &Path) -> Touched {
        if let Some((_, number)) = self.gitdirs.iter().find(|(dir, _)| path.starts_with(dir)) {
            return match is_lock(path) {
                true => Touched::Nothing,
                false => Touched::Slot(*number),
            };
        }
        if let Ok(inside) = path.strip_prefix(common) {
            let mut parts = inside.components().filter_map(|part| part.as_os_str().to_str());
            if is_lock(path) {
                return Touched::Nothing;
            }
            // Only the base's own moving changes what a slot reads as behind.
            // A branch or a tag moving does not, and an edge it does touch (a
            // deleted branch leaves a free slot's commits unnamed) is the
            // deadline's.
            return match (parts.next(), parts.collect::<Vec<_>>().as_slice()) {
                (Some("packed-refs"), _) => Touched::All,
                (Some("refs"), ["heads", name @ ..]) if name.join("/") == base => Touched::All,
                (Some("refs"), ["remotes", _, name @ ..]) if name.join("/") == base => {
                    Touched::All
                }
                // A worktree this has not met.
                (Some("worktrees"), [_, ..]) => Touched::All,
                _ => Touched::Nothing,
            };
        }
        let Some((tree, number)) = self.trees.iter().find(|(tree, _)| path.starts_with(tree))
        else {
            return Touched::Nothing;
        };
        let Ok(inside) = path.strip_prefix(tree) else {
            return Touched::Nothing;
        };
        let mut names = inside.components().filter_map(|part| part.as_os_str().to_str());
        let first = names.next();
        let build = first.is_some_and(|name| NOT_WORK.contains(&name));
        let installed = inside.components().any(|part| part.as_os_str() == "node_modules");
        match self.free.contains(number) && !build && !installed {
            true => Touched::Slot(*number),
            false => Touched::Nothing,
        }
    }
}

fn is_lock(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "lock")
}

struct Shared {
    common: PathBuf,
    base: String,
    state: Mutex<State>,
}

impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn heard(&self, event: notify::Result<Event>) {
        let mut state = self.state();
        let mut touched = false;
        match event {
            Err(_) => {
                state.moves.all = true;
                touched = true;
            }
            Ok(event) if matches!(event.kind, EventKind::Access(_)) => {}
            Ok(event) => {
                if event.need_rescan() {
                    state.moves.all = true;
                    touched = true;
                }
                for path in &event.paths {
                    match state.classify(&self.common, &self.base, path) {
                        Touched::Nothing => {}
                        Touched::All => {
                            state.moves.all = true;
                            touched = true;
                        }
                        Touched::Slot(number) => {
                            state.moves.slots.insert(number);
                            touched = true;
                        }
                    }
                }
            }
        }
        drop(state);
        if touched {
            // What a reader waiting on `Vcs::changes` wakes for.
            CHANGES.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn take(&self) -> Moves {
        std::mem::take(&mut self.state().moves)
    }
}

struct PoolWatch {
    shared: Arc<Shared>,
    /// `None` where the file system would not be watched: every read is then whole.
    watcher: Option<RecommendedWatcher>,
    armed: BTreeSet<usize>,
    kept: BTreeMap<usize, SlotReading>,
    acts: u64,
    /// The slots the last read went to git for.
    reread: Vec<usize>,
}

/// One pool's watch, by the repository it is cut from.
static WATCHES: Mutex<BTreeMap<PathBuf, PoolWatch>> = Mutex::new(BTreeMap::new());

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The readings of `pool`: every slot where `full`, nothing has been kept, or
/// something every slot depends on moved; otherwise only the slots touched.
pub(super) fn readings(pool: &Pool, full: bool) -> Vec<SlotReading> {
    let key = canonical(&pool.root);
    let taken = WATCHES.lock().unwrap_or_else(PoisonError::into_inner).remove(&key);
    let mut watch = taken.unwrap_or_else(|| PoolWatch::new(&key, &pool.base));
    let read = watch.read(pool, full);
    WATCHES.lock().unwrap_or_else(PoisonError::into_inner).insert(key, watch);
    read
}

impl PoolWatch {
    fn new(root: &Path, base: &str) -> PoolWatch {
        let common = git2::Repository::open(root)
            .map(|repo| canonical(repo.path()))
            .unwrap_or_else(|_| root.join(".git"));
        let shared = Arc::new(Shared {
            common: common.clone(),
            base: base.to_string(),
            state: Mutex::default(),
        });
        let heard = Arc::clone(&shared);
        let watcher = notify::recommended_watcher(move |event| heard.heard(event))
            .ok()
            .and_then(|mut watcher| {
                watcher.watch(&common, RecursiveMode::Recursive).ok()?;
                Some(watcher)
            });
        PoolWatch {
            shared,
            watcher,
            armed: BTreeSet::new(),
            kept: BTreeMap::new(),
            acts: ACTS.load(Ordering::Acquire),
            reread: Vec::new(),
        }
    }

    fn read(&mut self, pool: &Pool, full: bool) -> Vec<SlotReading> {
        let acts = ACTS.load(Ordering::Acquire);
        let moved = self.shared.take();
        let bays = pool.bays();
        let everything = full || moved.all || self.watcher.is_none() || acts != self.acts;
        let todo: Vec<usize> = bays
            .iter()
            .copied()
            .filter(|n| everything || moved.slots.contains(n) || !self.kept.contains_key(n))
            .collect();
        // Watched before it is read, so an edit between the two is heard.
        self.arm(pool, &todo);
        for reading in pool.readings_of(&todo) {
            self.kept.insert(reading.slot as usize, reading);
        }
        self.acts = acts;
        self.kept.retain(|n, _| bays.contains(n));
        self.settle(pool, &bays);
        self.reread = todo;
        bays.iter().filter_map(|n| self.kept.get(n).cloned()).collect()
    }

    fn arm(&mut self, pool: &Pool, numbers: &[usize]) {
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        let mut state = self.shared.state();
        for &number in numbers {
            let path = canonical(&pool.path_of(number));
            let Ok(repo) = git2::Repository::open(&path) else {
                continue;
            };
            let gitdir = canonical(repo.path());
            state.gitdirs.retain(|(_, n)| *n != number);
            state.gitdirs.push((gitdir, number));
            if self.armed.contains(&number) || watcher.watch(&path, RecursiveMode::Recursive).is_err()
            {
                continue;
            }
            self.armed.insert(number);
            state.trees.push((path, number));
            state.free.insert(number);
        }
    }

    /// Stop watching the work tree of every slot the reading shows held.
    fn settle(&mut self, pool: &Pool, bays: &[usize]) {
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        let mut state = self.shared.state();
        for number in self.armed.clone() {
            let free = bays.contains(&number)
                && self
                    .kept
                    .get(&number)
                    .is_some_and(|one| matches!(one.held, SlotHeld::Free | SlotHeld::Stranded(_)));
            if free {
                continue;
            }
            let path = canonical(&pool.path_of(number));
            let _ = watcher.unwatch(&path);
            self.armed.remove(&number);
            state.trees.retain(|(_, n)| *n != number);
            state.free.remove(&number);
        }
    }
}

/// The slots the last kept read of `root`'s pool went to git for.
#[cfg(test)]
pub(crate) fn last_reread(root: &Path) -> Vec<usize> {
    let all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    all.get(&canonical(root)).map(|watch| watch.reread.clone()).unwrap_or_default()
}

/// What the file system has told `root`'s pool since it was last read: whether
/// every slot, and which.
#[cfg(test)]
pub(crate) fn heard_of(root: &Path) -> (bool, Vec<usize>) {
    let all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    all.get(&canonical(root)).map_or((false, Vec::new()), |watch| {
        let state = watch.shared.state();
        (state.moves.all, state.moves.slots.iter().copied().collect())
    })
}
