//! The pool of warm worktrees a repository leases out, and takes back.
//!
//! A slot is a permanent checkout at `<repo>/.armada/slots/slot-<n>`, leased
//! onto a new branch with its build kept, and released only when nothing on it
//! would be lost. `docs/concepts/fleet.md`, *Worktree slots*, has the design.
//!
//! **The holder is recorded, and never a lock held open.** The command that
//! leases exits at once, so an `flock` would let go before the caller had read
//! the path. The lock beside each slot only makes one take or one release at a
//! time; who holds the slot is the record under it — a process, or one of
//! Fleet's Jobs by its id.
//!
//! **Past 500 lines, and kept whole**: take, release and status each read a
//! slot's state under the same lock and record, and a split along them would
//! put one invariant in three files.

mod answers;
mod git;
pub(crate) mod jobs;
mod record;

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{BaseSpec, SLOT_ROOT as SLOTS};

pub use answers::{Full, Lease, LeaseRefused, Leased, ReleaseRefused, Released, Slot, SlotState};
use git::{count, git, git_ok};
pub use record::Holder;
use record::Record;

/// Kept across every lease: what a build or an index writes and the next
/// build reads. `setup.seed.paths` joins these.
const WARM: &[&str] = &["target", "node_modules", ".gitnexus"];

/// How often a waiting lease looks again.
const LOOK_AGAIN: Duration = Duration::from_millis(200);

/// Copies a warm build directory into a new slot, copy-on-write. Injected,
/// because the clone is a system call this crate is not allowed to make.
pub type Seed<'a> = &'a dyn Fn(&Path, &Path) -> Result<(), String>;

/// Who the record beside the slot at `path` names, alive or not; `None` for a
/// slot nobody holds, and for a path that is no slot.
pub fn holder_of(path: &Path) -> Option<Holder> {
    let mut record = path.as_os_str().to_owned();
    record.push(".lease");
    Record::read(Path::new(&record)).map(|record| record.holder)
}

/// One repository's pool.
#[derive(Clone, Debug)]
pub struct Pool {
    root: PathBuf,
    count: usize,
    base: String,
    keep: Vec<String>,
    seeds: Vec<String>,
}

impl Pool {
    /// `count` slots under `root`, the repository's own checkout, leased from
    /// `base`. `seeds` is `setup.seed.paths`: cloned into a new slot, and kept
    /// across leases with the rest of [`WARM`].
    pub fn at(root: &Path, count: usize, base: &str, seeds: Vec<String>) -> Pool {
        let mut keep: Vec<String> = WARM.iter().map(|path| (*path).to_string()).collect();
        for path in &seeds {
            if !keep.contains(path) {
                keep.push(path.clone());
            }
        }
        Pool {
            root: root.to_path_buf(),
            count: count.max(1),
            base: base.to_string(),
            keep,
            seeds,
        }
    }

    /// The checkout every worktree of the repository at `within` belongs to.
    pub fn root_of(within: &Path) -> Result<PathBuf, String> {
        let common = git(
            within,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        Path::new(&common)
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("{common} has no parent, so it is no checkout's git directory"))
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn path_of(&self, slot: usize) -> PathBuf {
        self.root.join(SLOTS).join(format!("slot-{slot}"))
    }

    /// Take a slot, waiting as long as it takes. `waiting` hears each look
    /// that found none.
    pub fn lease(
        &self,
        branch: &str,
        holder: &Holder,
        since: u64,
        seed: Seed<'_>,
        mut waiting: impl FnMut(&Full),
    ) -> Result<Lease, LeaseRefused> {
        loop {
            match self.try_lease(branch, holder, since, seed)? {
                Leased::Took(lease) => return Ok(lease),
                Leased::Full(full) => waiting(&full),
            }
            std::thread::sleep(LOOK_AGAIN);
        }
    }

    /// Take a slot now, or say why every one is unavailable.
    pub fn try_lease(
        &self,
        branch: &str,
        holder: &Holder,
        since: u64,
        seed: Seed<'_>,
    ) -> Result<Leased, LeaseRefused> {
        // A Job asking again is handed its own slot as it stands: resetting it
        // would throw away the work a Drone left there.
        if let Holder::Job(_) = holder {
            if let Some(slot) = self.held_by(holder) {
                return Ok(Leased::Took(Lease {
                    slot,
                    path: self.path_of(slot),
                    reclaimed_from: None,
                    seeded_from: None,
                    unfetched: None,
                    made: false,
                }));
            }
        }
        let unfetched = self.fetched().err();
        let from = self.base_ref();
        let existing = format!("refs/heads/{branch}");
        if git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &existing]) {
            let commits = self.unlanded(&self.root, &existing);
            if commits > 0 {
                return Err(LeaseRefused::BranchHoldsWork {
                    branch: branch.to_string(),
                    commits,
                });
            }
        }

        let mut slots = Vec::with_capacity(self.count);
        let mut unmade = None;
        for number in 1..=self.count {
            let path = self.path_of(number);
            let Some(lock) = self.locked(number).map_err(LeaseRefused::Vcs)? else {
                slots.push(Slot {
                    number,
                    path,
                    state: SlotState::Busy,
                });
                continue;
            };
            let state = self.state_of(number);
            let reclaimed_from = match &state {
                SlotState::Free => None,
                SlotState::Abandoned { branch, .. } => Some(branch.clone()),
                SlotState::Unmade if unmade.is_none() => {
                    unmade = Some(number);
                    slots.push(Slot {
                        number,
                        path,
                        state,
                    });
                    continue;
                }
                _ => {
                    slots.push(Slot {
                        number,
                        path,
                        state,
                    });
                    continue;
                }
            };
            let lease = self.point(number, branch, &from, holder, since, None)?;
            drop(lock);
            return Ok(Leased::Took(Lease {
                reclaimed_from,
                unfetched,
                ..lease
            }));
        }

        let Some(number) = unmade else {
            return Ok(Leased::Full(Full { slots }));
        };
        // Looked at again under its lock: another lease may have made it.
        let lock = self.locked(number).map_err(LeaseRefused::Vcs)?;
        if lock.is_none() || self.state_of(number) != SlotState::Unmade {
            return Ok(Leased::Full(Full { slots }));
        }
        let path = self.path_of(number);
        git(
            &self.root,
            &[
                "worktree",
                "add",
                "--detach",
                "--quiet",
                &path.to_string_lossy(),
                &from,
            ],
        )
        .map_err(LeaseRefused::Vcs)?;
        let seeded_from = self.seeded(&path, seed);
        let lease = self.point(number, branch, &from, holder, since, seeded_from)?;
        drop(lock);
        Ok(Leased::Took(Lease {
            unfetched,
            made: true,
            ..lease
        }))
    }

    /// Give a slot back: refused while it holds anything not on the remote or
    /// the base, and otherwise detached so its branch is free to land.
    pub fn release(&self, path: &Path) -> Result<Released, ReleaseRefused> {
        let wanted = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let Some(number) = (1..=self.count).find(|n| {
            let slot = self.path_of(*n);
            wanted == slot.canonicalize().unwrap_or(slot)
        }) else {
            return Err(ReleaseRefused::NotASlot(path.to_path_buf()));
        };
        self.give_back(number, None)
    }

    /// Give back slot `number`, which `holder` must hold. Refused for a Job,
    /// the refusal is written onto the slot, so `--status` says why it stays
    /// held.
    pub fn release_held(&self, number: usize, holder: &Holder) -> Result<Released, ReleaseRefused> {
        self.give_back(number, Some(holder))
    }

    fn give_back(
        &self,
        number: usize,
        holder: Option<&Holder>,
    ) -> Result<Released, ReleaseRefused> {
        let slot = self.path_of(number);
        if !slot.exists() {
            return Err(ReleaseRefused::NotASlot(slot));
        }
        let lock = self.lock_file(number).map_err(ReleaseRefused::Vcs)?;
        lock.lock()
            .map_err(|why| ReleaseRefused::Vcs(why.to_string()))?;
        let Some(mut record) = Record::read(&self.record_path(number)) else {
            return Err(ReleaseRefused::NotLeased(slot));
        };
        if holder.is_some_and(|holder| *holder != record.holder) {
            return Err(ReleaseRefused::HeldByAnother(record.holder.said()));
        }
        let refused = match git::dirty(&slot) {
            Err(why) => Some(ReleaseRefused::Vcs(why)),
            Ok(files) if !files.is_empty() => Some(ReleaseRefused::Dirty {
                path: slot.clone(),
                files,
            }),
            Ok(_) => match self.unlanded(&slot, "HEAD") {
                0 => None,
                commits => Some(ReleaseRefused::Unlanded {
                    branch: record.branch.clone(),
                    commits,
                }),
            },
        };
        if let Some(refused) = refused {
            if let Holder::Job(_) = record.holder {
                record.kept = Some(refused.said());
                record
                    .write(&self.record_path(number))
                    .map_err(ReleaseRefused::Vcs)?;
            }
            return Err(refused);
        }
        git(&slot, &["switch", "--detach", "--quiet"]).map_err(ReleaseRefused::Vcs)?;
        Record::clear(&self.record_path(number)).map_err(ReleaseRefused::Vcs)?;
        Ok(Released {
            slot: number,
            branch: record.branch,
        })
    }

    /// The slot `holder` holds, if it holds one.
    pub fn held_by(&self, holder: &Holder) -> Option<usize> {
        (1..=self.count).find(|number| {
            Record::read(&self.record_path(*number)).is_some_and(|record| &record.holder == holder)
        })
    }

    /// Whether a lease for `holder` would take a slot now: one it holds, or
    /// one free, abandoned or not yet made. Read without the locks, as
    /// [`status`](Pool::status) is.
    pub fn open_for(&self, holder: &Holder) -> bool {
        self.held_by(holder).is_some()
            || (1..=self.count).any(|number| {
                matches!(
                    self.state_of(number),
                    SlotState::Free | SlotState::Abandoned { .. } | SlotState::Unmade
                )
            })
    }

    /// One slot and what holds it.
    pub fn state(&self, number: usize) -> SlotState {
        self.state_of(number)
    }

    /// Every slot and what holds it. Read without the locks, so a take under
    /// way reads as whatever it had reached.
    pub fn status(&self) -> Vec<Slot> {
        (1..=self.count)
            .map(|number| Slot {
                number,
                path: self.path_of(number),
                state: self.state_of(number),
            })
            .collect()
    }

    fn state_of(&self, number: usize) -> SlotState {
        let path = self.path_of(number);
        if !path.exists() {
            return SlotState::Unmade;
        }
        if !git::is_checkout(&path) {
            return SlotState::NotACheckout;
        }
        let record = Record::read(&self.record_path(number));
        if let Some(Record {
            branch,
            holder,
            since,
            kept,
        }) = record.as_ref().filter(|record| record.holder.alive())
        {
            return SlotState::Held {
                branch: branch.clone(),
                holder: holder.clone(),
                since: *since,
                kept: kept.clone(),
            };
        }
        // **Asked of a slot with no record too.** A record lost or cut short
        // must not make a tree holding work read as free.
        let why = match git::dirty(&path) {
            Err(why) => Some(why),
            Ok(files) if !files.is_empty() => {
                Some(format!("{} uncommitted, first {}", files.len(), files[0]))
            }
            // A released slot sits detached on a branch that still names its
            // commits, so a lease loses nothing unless HEAD is all that does.
            Ok(_) if record.is_none() => {
                match count(&path, &["HEAD", "--not", "--branches", "--remotes"]) {
                    0 => None,
                    commits => Some(format!("{commits} commits no branch names")),
                }
            }
            Ok(_) => match self.unlanded(&path, "HEAD") {
                0 => None,
                commits => Some(format!(
                    "{commits} commits on neither the remote nor {}",
                    self.base
                )),
            },
        };
        let (branch, since) = match record {
            Some(record) => (record.branch, record.since),
            None if why.is_none() => return SlotState::Free,
            None => (
                git(&path, &["branch", "--show-current"]).unwrap_or_default(),
                0,
            ),
        };
        match why {
            None => SlotState::Abandoned { branch, since },
            Some(why) => SlotState::Stranded { branch, since, why },
        }
    }

    /// Put slot `number` on `branch` at the base, cleaned of all but the warm
    /// paths, and record who holds it. The caller holds the slot's lock.
    fn point(
        &self,
        number: usize,
        branch: &str,
        from: &str,
        holder: &Holder,
        since: u64,
        seeded_from: Option<String>,
    ) -> Result<Lease, LeaseRefused> {
        let path = self.path_of(number);
        if !git::is_checkout(&path) {
            return Err(LeaseRefused::Vcs(format!(
                "{} is not a checkout git knows; remove it and the next lease makes it again",
                path.display()
            )));
        }
        // `--no-track`, so the new branch has no upstream and a bare `git push`
        // cannot reach the base.
        git(
            &path,
            &["switch", "--quiet", "--no-track", "-C", branch, from],
        )
        .map_err(LeaseRefused::Vcs)?;
        let mut clean = vec!["clean", "-fdx", "--quiet"];
        for kept in &self.keep {
            clean.extend(["-e", kept.as_str()]);
        }
        git(&path, &clean).map_err(LeaseRefused::Vcs)?;
        Record {
            branch: branch.to_string(),
            holder: holder.clone(),
            since,
            kept: None,
        }
        .write(&self.record_path(number))
        .map_err(LeaseRefused::Vcs)?;
        Ok(Lease {
            slot: number,
            path,
            reclaimed_from: None,
            seeded_from,
            unfetched: None,
            made: false,
        })
    }

    /// Clone each seed path from the warmest base checkout into a new slot.
    /// Answers the commit it came from, or `None` where the slot starts cold.
    fn seeded(&self, slot: &Path, seed: Seed<'_>) -> Option<String> {
        let commit = self.warmest_base()?;
        let base = PathBuf::from(
            BaseSpec::at(&self.root.to_string_lossy(), &commit)
                .ok()?
                .path(),
        );
        let mut any = false;
        for path in &self.seeds {
            let (from, to) = (base.join(path), slot.join(path));
            if from.is_dir() && !to.exists() && seed(&from, &to).is_ok() {
                any = true;
            }
        }
        any.then_some(commit)
    }

    /// The base checkout whose seed is warm, preferring the base's own commit
    /// and otherwise the one warmed last.
    fn warmest_base(&self) -> Option<String> {
        let root = self.root.to_string_lossy();
        let head = git(&self.root, &["rev-parse", &self.base_ref()]).ok();
        let mut warm: Vec<(std::time::SystemTime, String)> =
            std::fs::read_dir(self.root.join(".armada/bases"))
                .ok()?
                .flatten()
                .filter_map(|entry| {
                    let commit = entry.file_name().to_string_lossy().into_owned();
                    let marker = BaseSpec::at(&root, &commit).ok()?.seed_marker();
                    let warmed = std::fs::metadata(marker).ok()?.modified().ok()?;
                    Some((warmed, commit))
                })
                .collect();
        if let Some(head) = head.filter(|head| warm.iter().any(|(_, c)| c == head)) {
            return Some(head);
        }
        warm.sort();
        warm.pop().map(|(_, commit)| commit)
    }

    /// Fetch the base from `origin`, where there is one.
    fn fetched(&self) -> Result<(), String> {
        if !git_ok(&self.root, &["remote", "get-url", "origin"]) {
            return Ok(());
        }
        git(
            &self.root,
            &["fetch", "--quiet", "--no-tags", "origin", &self.base],
        )
        .map(|_| ())
    }

    /// The remote's base where this machine has fetched one, else the local
    /// branch.
    fn base_ref(&self) -> String {
        let remote = format!("refs/remotes/origin/{}", self.base);
        match git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &remote]) {
            true => remote,
            false => format!("refs/heads/{}", self.base),
        }
    }

    /// Commits reachable from `rev` in `at` that are on no remote and not on
    /// the local base. A count git cannot give reads as one: unknown is not
    /// landed.
    fn unlanded(&self, at: &Path, rev: &str) -> usize {
        let local = format!("refs/heads/{}", self.base);
        let mut args = vec![rev, "--not", "--remotes"];
        if git_ok(at, &["rev-parse", "--verify", "--quiet", &local]) {
            args.push(&local);
        }
        count(at, &args)
    }

    fn record_path(&self, number: usize) -> PathBuf {
        self.root.join(SLOTS).join(format!("slot-{number}.lease"))
    }

    fn lock_file(&self, number: usize) -> Result<File, String> {
        std::fs::create_dir_all(self.root.join(SLOTS)).map_err(|why| why.to_string())?;
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .append(true)
            .open(self.record_path(number))
            .map_err(|why| why.to_string())
    }

    /// The slot's lock, or `None` where another take or release has it.
    fn locked(&self, number: usize) -> Result<Option<File>, String> {
        let file = self.lock_file(number)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(why)) => Err(why.to_string()),
        }
    }
}
