//! Which slots of a pool changed, so a reader that keeps the pool's readings
//! reads those and leaves the rest alone.
//!
//! **What is watched, and what is not.** The repository's own git directory
//! (every slot's `HEAD` and `index` live under `worktrees/<name>/`, and the refs
//! every slot is measured against are shared), and the work tree of a slot
//! nobody holds, where an edit changes nothing under `.git`. A held slot's work
//! tree is not watched: its reading never asks git for status, and a build
//! there writes `target/` thousands of events a minute. Measured in
//! `docs/spikes/032-what-moves-when-a-slot-changes.md`.
//!
//! **Events are a hint, never the truth.** A holder that dies leaves no file
//! behind to notice, and a stream restarts as a slot is added to it, so every
//! reading is also read again when the reader says `full`.
//!
//! **A full read asks `git status` of a free slot only where the watch cannot
//! vouch for the last answer.** `status` is the one expensive question (71% of
//! a core each, in a large repository), and an edit to a free slot's work tree
//! is what the watch reports. Everything else a reading holds (HEAD, the
//! refs, the record) is in process and read every time. The answer is read
//! again where the watch was not armed before this read, or heard that slot,
//! or lost events (an error, or a rescan flag), or restarted its stream (a path
//! added or removed: the events of that moment are the ones it drops), or
//! heard an edit under a directory it ignores that git does not, or this
//! adapter acted on the pool. See `Trust` below.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{SlotHeld, SlotReading};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::record::Record;
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
    /// Paths changed under a directory this watch ignores (a build, an
    /// install) in a free slot. Not an edit unless git says the path counts:
    /// asked at the next read, in process.
    unwatched: BTreeMap<usize, BTreeSet<PathBuf>>,
}

/// The most paths kept per slot under `Moves::unwatched`; past it the slot
/// counts as touched, which costs one `git status` and not a list that grows
/// with a build.
const MOST_UNWATCHED: usize = 32;

enum Touched {
    Nothing,
    All,
    Slot(usize),
    Unwatched(usize, PathBuf),
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
                // What every slot's `status` leaves out.
                (Some("info"), ["exclude"]) => Touched::All,
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
        if !self.free.contains(number) || first == Some(".git") {
            return Touched::Nothing;
        }
        match build || installed {
            false => Touched::Slot(*number),
            true => Touched::Unwatched(*number, path.to_path_buf()),
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
                        Touched::Unwatched(number, path) => {
                            let paths = state.moves.unwatched.entry(number).or_default();
                            paths.insert(path);
                            if paths.len() > MOST_UNWATCHED {
                                state.moves.unwatched.remove(&number);
                                state.moves.slots.insert(number);
                                touched = true;
                            }
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
    /// What `git status` said of each free slot when it was last asked.
    statuses: BTreeMap<usize, Vec<String>>,
    /// Slots whose `statuses` entry the watch cannot vouch for: the stream
    /// restarted since it was read.
    distrust: BTreeSet<usize>,
    acts: u64,
    /// The slots the last read went to git for.
    reread: Vec<usize>,
    /// The slots the last read asked `git status` of.
    status_asked: Vec<usize>,
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
            statuses: BTreeMap::new(),
            distrust: BTreeSet::new(),
            acts: ACTS.load(Ordering::Acquire),
            reread: Vec::new(),
            status_asked: Vec::new(),
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
        let armed_before = self.armed.clone();
        // Watched before it is read, so an edit between the two is heard.
        if self.arm(pool, &todo) {
            self.distrust.extend(armed_before.iter().copied());
        }
        let trusted = self.trusted(pool, &moved, &bays, &armed_before, acts);
        let mut asked = Vec::new();
        for read in pool.readings_trusting(&todo, &trusted) {
            let number = read.reading.slot as usize;
            match &read.status {
                Some(Ok(files)) => {
                    self.statuses.insert(number, files.clone());
                }
                _ => {
                    self.statuses.remove(&number);
                }
            }
            if read.status.is_some() && read.asked_git {
                asked.push(number);
            }
            if read.asked_git {
                self.distrust.remove(&number);
            }
            self.kept.insert(number, read.reading);
        }
        self.acts = acts;
        self.kept.retain(|n, _| bays.contains(n));
        self.statuses.retain(|n, _| bays.contains(n));
        if self.settle(pool, &bays) {
            // The stream restarted after these were read.
            self.distrust.extend(self.armed.iter().copied());
        }
        self.reread = todo;
        self.status_asked = asked;
        bays.iter().filter_map(|n| self.kept.get(n).cloned()).collect()
    }

    /// The slots whose last `git status` the watch vouches for, with it.
    ///
    /// **Every case that could let a missed edit stand past the deadline is a
    /// reason to leave a slot out**: nothing lost or rescanned, the watch armed
    /// since before the last read, no restart of the stream since, nothing
    /// heard of the slot, no edit under an ignored directory that git counts,
    /// and nothing this adapter did to the pool.
    fn trusted(
        &mut self,
        pool: &Pool,
        moved: &Moves,
        bays: &[usize],
        armed_before: &BTreeSet<usize>,
        acts: u64,
    ) -> BTreeMap<usize, Vec<String>> {
        if moved.all || self.watcher.is_none() || acts != self.acts {
            return BTreeMap::new();
        }
        let counted = counted_edits(pool, &moved.unwatched);
        armed_before
            .iter()
            .filter(|n| bays.contains(n) && self.armed.contains(n))
            .filter(|n| !moved.slots.contains(n) && !counted.contains(n) && !self.distrust.contains(n))
            .filter_map(|n| Some((*n, self.statuses.get(n)?.clone())))
            .collect()
    }

    /// **No call into the watcher is made with the state locked.** Adding or
    /// removing a path waits on the thread that delivers events, and that
    /// thread may be in `heard`, waiting for the lock: each waits on the other.
    ///
    /// True where a path was added: the stream is rebuilt, and an edit in the
    /// moment it was is one it never reports.
    fn arm(&mut self, pool: &Pool, numbers: &[usize]) -> bool {
        let Some(watcher) = self.watcher.as_mut() else {
            return false;
        };
        let mut added = false;
        let mut found = Vec::new();
        for &number in numbers {
            let path = canonical(&pool.path_of(number));
            if let Ok(repo) = git2::Repository::open(&path) {
                found.push((canonical(repo.path()), path, number));
            }
        }
        {
            let mut state = self.shared.state();
            for (gitdir, _, number) in &found {
                state.gitdirs.retain(|(_, n)| n != number);
                state.gitdirs.push((gitdir.clone(), *number));
            }
        }
        for (_, path, number) in found {
            // A held slot's work tree is not watched, so it is not armed to be
            // let go again at the end of this read: that would restart the
            // stream twice for a slot nobody asked it about.
            if self.armed.contains(&number) || held_now(pool, number) {
                continue;
            }
            // Counted before it is watched, so an edit as it starts is heard.
            {
                let mut state = self.shared.state();
                state.trees.push((path.clone(), number));
                state.free.insert(number);
            }
            added = true;
            if watcher.watch(&path, RecursiveMode::Recursive).is_ok() {
                self.armed.insert(number);
            } else {
                let mut state = self.shared.state();
                state.trees.retain(|(_, n)| *n != number);
                state.free.remove(&number);
            }
        }
        added
    }

    /// Stop watching the work tree of every slot the reading shows held.
    ///
    /// True where one was: the stream restarts, as it does when one is added.
    fn settle(&mut self, pool: &Pool, bays: &[usize]) -> bool {
        let Some(watcher) = self.watcher.as_mut() else {
            return false;
        };
        let mut removed = false;
        for number in self.armed.clone() {
            let free = bays.contains(&number)
                && self
                    .kept
                    .get(&number)
                    .is_some_and(|one| matches!(one.held, SlotHeld::Free | SlotHeld::Stranded(_)));
            if free {
                continue;
            }
            removed = true;
            let _ = watcher.unwatch(&canonical(&pool.path_of(number)));
            self.armed.remove(&number);
            let mut state = self.shared.state();
            state.trees.retain(|(_, n)| *n != number);
            state.free.remove(&number);
        }
        removed
    }
}

/// Whether something alive holds slot `number`, from its record alone.
fn held_now(pool: &Pool, number: usize) -> bool {
    Record::read(&pool.record_path(number)).is_some_and(|record| record.holder.alive())
}

/// The slots among `unwatched` with a path git does not ignore: an edit
/// under `target` or `node_modules` that a repository without those in its
/// `.gitignore` shows in `status`, and the watch does not report. A path git
/// cannot be asked about counts.
fn counted_edits(pool: &Pool, unwatched: &BTreeMap<usize, BTreeSet<PathBuf>>) -> BTreeSet<usize> {
    unwatched
        .iter()
        .filter(|(number, paths)| {
            let top = canonical(&pool.path_of(**number));
            let Ok(repo) = git2::Repository::open(&top) else {
                return true;
            };
            paths.iter().any(|path| {
                let inside = path.strip_prefix(&top).unwrap_or(path);
                !repo.is_path_ignored(inside).unwrap_or(false)
            })
        })
        .map(|(number, _)| *number)
        .collect()
}

/// The slots the last kept read of `root`'s pool went to git for.
#[cfg(test)]
pub(crate) fn last_reread(root: &Path) -> Vec<usize> {
    let all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    all.get(&canonical(root)).map(|watch| watch.reread.clone()).unwrap_or_default()
}

/// Say `event` to `root`'s watch as the file system would.
#[cfg(test)]
pub(crate) fn heard_by(root: &Path, event: notify::Result<Event>) {
    let all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(watch) = all.get(&canonical(root)) {
        watch.shared.heard(event);
    }
}

/// Take the watcher from `root`'s pool, as a file system that would not be
/// watched leaves it.
#[cfg(test)]
pub(crate) fn unwatched(root: &Path) {
    let mut all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(watch) = all.get_mut(&canonical(root)) {
        watch.watcher = None;
    }
}

/// The slots the last kept read of `root`'s pool asked `git status` of.
#[cfg(test)]
pub(crate) fn last_status_asked(root: &Path) -> Vec<usize> {
    let all = WATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    all.get(&canonical(root)).map(|watch| watch.status_asked.clone()).unwrap_or_default()
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
