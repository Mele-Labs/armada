//! The pool of warm worktrees a repository leases out, and takes back.
//!
//! A slot is a permanent checkout at `<repo>/.armada/slots/slot-<n>`, leased
//! onto a new branch (or an existing one, at its tip) with its build kept, and
//! released only when nothing on it would be lost: committed on its branch.
//! `docs/concepts/fleet.md`, *Worktree slots*, has the design.
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
mod existing;
mod git;
mod handing;
mod history;
pub(crate) mod hosted;
mod parking;
mod reading;
mod record;
mod reclaiming;
mod rescue;
mod saving;
mod shape;
pub(crate) mod strays;
mod trim;
pub(crate) mod watching;

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_traits::{BaseSpec, SLOT_ROOT as SLOTS};

pub use answers::{
    Committed, Full, Lease, LeaseRefused, Leased, ParkRefused, Parked, ReleaseRefused, Released,
    Slot, SlotState,
};
use existing::Onto;
use git::{count, git, git_ok};
pub use history::{checkouts_in, checkouts_of, turns_of, Checkout, Turn, KEPT_DAYS};
pub use record::Holder;
pub use strays::{end_pids, listening_ports};
use record::Record;
pub use shape::Unshaped;
pub(crate) use watching::acted;
pub use trim::{sweep_repository, trim_target, Trim, Trimmed, CEILING_GIB, TRIM_AFTER_DAYS};

/// Kept across every lease: what a build or an index writes and the next
/// build reads. `setup.seed.paths` joins these.
const WARM: &[&str] = &["target", "node_modules", ".gitnexus"];

/// How often a waiting lease looks again.
const LOOK_AGAIN: Duration = Duration::from_millis(200);

/// How many looks a lease that is not waiting for anyone to give a slot back makes while the pool
/// reads full only for a moment, and how far apart.
const SETTLE_LOOKS: usize = 10;
const SETTLE_EVERY: Duration = Duration::from_millis(20);

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

/// The branch the record beside the slot at `path` names; `None` where nobody
/// holds it.
pub fn branch_of(path: &Path) -> Option<String> {
    let mut record = path.as_os_str().to_owned();
    record.push(".lease");
    Record::read(Path::new(&record)).map(|record| record.branch)
}

/// The agent session the record beside the slot at `path` says holds the lease, or held it last:
/// written when the lease was taken from an agent's command, or by [`note_session`]. `None` where
/// neither did, which is a lease by hand and one written before either existed. **A holder that is
/// gone still has one**, so a reader that asks who holds the slot asks the holder first.
pub fn session_of(path: &Path) -> Option<String> {
    let mut record = path.as_os_str().to_owned();
    record.push(".lease");
    Record::read(Path::new(&record)).and_then(|record| record.session)
}

/// The branch checked out in the worktree at `path`, read off its `HEAD`; `None` for a detached
/// head and for a path that is no worktree. The lease's own branch is where the slot was cut, not
/// where it stands now.
pub fn checked_out(path: &Path) -> Option<String> {
    let dot_git = path.join(".git");
    let git_dir = match std::fs::read_to_string(&dot_git) {
        Ok(said) => {
            let target = said.trim().strip_prefix("gitdir: ")?;
            path.join(target)
        }
        Err(_) => dot_git,
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head.trim().strip_prefix("ref: refs/heads/").map(str::to_string)
}

/// Write on the record beside the slot at `path` that `session`'s process tree
/// holds it. Only a live process holder is written on: a Job's record already
/// names the Session it was leased for. Nothing, where the record already says
/// so, which is every call but the first.
pub fn note_session(path: &Path, session: &str) -> Result<(), String> {
    let mut record_path = path.as_os_str().to_owned();
    record_path.push(".lease");
    let record_path = PathBuf::from(record_path);
    let wanted = |record: &Record| {
        matches!(record.holder, Holder::Process { .. })
            && record.holder.alive()
            && record.session.as_deref() != Some(session)
    };
    if !Record::read(&record_path).is_some_and(|record| wanted(&record)) {
        return Ok(());
    }
    let lock = OpenOptions::new()
        .append(true)
        .open(&record_path)
        .map_err(|why| format!("{}: {why}", record_path.display()))?;
    lock.lock().map_err(|why| why.to_string())?;
    match Record::read(&record_path) {
        Some(mut record) if wanted(&record) => {
            record.session = Some(session.to_string());
            record.write(&record_path)
        }
        _ => Ok(()),
    }
}

/// One repository's pool.
#[derive(Clone, Debug)]
pub struct Pool {
    root: PathBuf,
    count: usize,
    base: String,
    keep: Vec<String>,
    seeds: Vec<String>,
    trim: Trim,
    /// The agent session each lease taken through this pool is written as taken by.
    session: Option<String>,
}

impl Pool {
    /// `count` slots under `root`, the repository's own checkout, leased from
    /// `base`, unless a person changed the pool on this machine: `shape`. `seeds` is `setup.seed.paths`: cloned into a new slot, and kept
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
            trim: Trim::SHIPPED,
            session: None,
        }
    }

    /// This pool, writing each lease it takes as taken for the agent session `session`. A lease
    /// held by a process says nothing of which agent that is, and this is how Fleet reads it.
    pub fn leased_for(mut self, session: Option<String>) -> Pool {
        self.session = session.filter(|id| !id.trim().is_empty());
        self
    }

    /// This pool, trimming a released slot's `target/` as `trim` says.
    pub fn trimming(mut self, trim: Trim) -> Pool {
        self.trim = trim;
        self
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

    /// How many slots the pool has on this machine.
    pub fn count(&self) -> usize {
        self.bays().len()
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
            match self.take(Onto::New, branch, holder, since, seed)? {
                Leased::Took(lease) => return Ok(lease),
                Leased::Full(full) if full.settled() => waiting(&full),
                Leased::Full(_) => {}
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
        if let Some(own) = self.handed_back(holder) {
            return Ok(Leased::Took(own));
        }
        self.looked_until_settled(|| self.take(Onto::New, branch, holder, since, seed))
    }

    /// `look`, again a few times while the pool reads full only because a slot is locked or not
    /// made for a moment ([`Full::settled`]). **Whoever hears a pool is full acts on it**: Fleet
    /// queues a session and asks the person, so a look that was wrong for a few milliseconds is
    /// not the answer.
    pub(super) fn looked_until_settled(
        &self,
        look: impl Fn() -> Result<Leased, LeaseRefused>,
    ) -> Result<Leased, LeaseRefused> {
        for _ in 1..SETTLE_LOOKS {
            match look()? {
                Leased::Full(full) if !full.settled() => std::thread::sleep(SETTLE_EVERY),
                answer => return Ok(answer),
            }
        }
        look()
    }

    /// A Job asking again is handed its own slot as it stands: resetting it
    /// would throw away the work a Drone left there. **Only Fleet asks this
    /// way** (`try_lease`); `armada worktree lease` takes a slot per call, so
    /// a hosted session, which holds as a Job of its own id, can lease one
    /// for a subagent beside its own.
    fn handed_back(&self, holder: &Holder) -> Option<Lease> {
        let Holder::Hosted(_) = holder else { return None };
        let slot = self.held_by(holder)?;
        Some(Lease {
            slot,
            path: self.path_of(slot),
            reclaimed_from: None,
            seeded_from: None,
            unfetched: None,
            made: false,
            cut: false,
        })
    }

    fn take(
        &self,
        onto: Onto,
        branch: &str,
        holder: &Holder,
        since: u64,
        seed: Seed<'_>,
    ) -> Result<Leased, LeaseRefused> {
        // A branch that exists is leased at its tip whatever was asked. The
        // base is fetched for it too: a branch with no work of its own is
        // brought up to it, so the agent never has to.
        let onto = self.resolved(onto, branch)?;
        let unfetched = self.fetched().err();
        let from = self.base_ref();
        // Git lets one checkout have a branch, so it can only go to the slot
        // that has it, and only if that slot may be taken: one whose holder
        // is gone is, and is already on the branch.
        let mut bays = self.bays();
        let mut only = None;
        if let Onto::Existing = onto {
            if let Some(at) = self.checked_out(branch)? {
                let number = self.slot_at(&at).ok_or_else(|| LeaseRefused::CheckedOutElsewhere {
                    branch: branch.to_string(),
                    at: at.clone(),
                })?;
                bays.retain(|bay| *bay == number);
                only = Some(at);
            }
        }
        let mut slots = Vec::with_capacity(bays.len());
        let mut unmade = None;
        for number in bays {
            let path = self.path_of(number);
            let Some(lock) = self.locked(number).map_err(LeaseRefused::Vcs)? else {
                slots.push(Slot {
                    number,
                    path,
                    state: SlotState::Busy,
                    closed: self.closed(number),
                });
                continue;
            };
            let state = self.state_of(number);
            // Asked under the lock: a person may have closed or removed it
            // since `bays` was read.
            let leasable = self.leasable(number);
            let reclaimed_from = match &state {
                SlotState::Free if leasable => None,
                SlotState::Abandoned { branch, .. } if leasable => Some(branch.clone()),
                SlotState::Unmade if leasable && unmade.is_none() => {
                    unmade = Some(number);
                    slots.push(Slot {
                        number,
                        path,
                        state,
                        closed: false,
                    });
                    continue;
                }
                _ => {
                    slots.push(Slot {
                        number,
                        path,
                        state,
                        closed: !leasable,
                    });
                    continue;
                }
            };
            let lease = self.point(number, onto, branch, &from, holder, since, None)?;
            drop(lock);
            return Ok(Leased::Took(Lease {
                reclaimed_from,
                unfetched,
                ..lease
            }));
        }

        if let Some(at) = only {
            // Held, stranded or not a checkout: not this lease's to take, and
            // not worth waiting on.
            return match slots.iter().all(|slot| slot.state == SlotState::Busy) {
                true => Ok(Leased::Full(Full { slots })),
                false => Err(LeaseRefused::CheckedOutElsewhere {
                    branch: branch.to_string(),
                    at,
                }),
            };
        }
        let Some(number) = unmade else {
            return Ok(Leased::Full(Full { slots }));
        };
        // Looked at again under its lock: another lease may have made it.
        let lock = self.locked(number).map_err(LeaseRefused::Vcs)?;
        if lock.is_none() || !self.leasable(number) || self.state_of(number) != SlotState::Unmade {
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
        let lease = self.point(number, onto, branch, &from, holder, since, seeded_from)?;
        drop(lock);
        Ok(Leased::Took(Lease {
            unfetched,
            made: true,
            ..lease
        }))
    }

    /// Give a slot back: refused while it holds uncommitted files or commits
    /// its branch, the remote and the base all lack, and otherwise detached
    /// so its branch is free to land. A commit on the branch is enough; a
    /// push is not asked for.
    pub fn release(&self, path: &Path) -> Result<Released, ReleaseRefused> {
        let wanted = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let Some(number) = self.bays().into_iter().find(|n| {
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
        self.give_back_locked(number, holder, None)
    }

    /// [`give_back`](Pool::give_back) for a caller already holding slot
    /// `number`'s lock, as a park does between its commit and its release.
    ///
    /// `on` is the branch the checkout is judged against where it is not the
    /// one the lease records: a session's park, which commits to the branch the
    /// session moved to.
    fn give_back_locked(
        &self,
        number: usize,
        holder: Option<&Holder>,
        on: Option<&str>,
    ) -> Result<Released, ReleaseRefused> {
        let slot = self.path_of(number);
        let Some(record) = Record::read(&self.record_path(number)) else {
            return Err(ReleaseRefused::NotLeased(slot));
        };
        if holder.is_some_and(|holder| *holder != record.holder) {
            return Err(ReleaseRefused::HeldByAnother(record.holder.said()));
        }
        let branch = on.map_or_else(|| record.branch.clone(), str::to_string);
        let refused = match git::dirty(&slot) {
            Err(why) => Some(ReleaseRefused::Vcs(why)),
            Ok(files) if !files.is_empty() => Some(ReleaseRefused::Dirty {
                path: slot.clone(),
                files,
            }),
            Ok(_) => match self.off_branch(&slot, &branch) {
                0 => None,
                commits => Some(ReleaseRefused::OffTheBranch {
                    branch: branch.clone(),
                    commits,
                }),
            },
        };
        if let Some(refused) = refused {
            return Err(refused);
        }
        git(&slot, &["switch", "--detach", "--quiet"]).map_err(ReleaseRefused::Vcs)?;
        Record::clear(&self.record_path(number)).map_err(ReleaseRefused::Vcs)?;
        self.noted_release(number, &record);
        trim_target(&slot.join("target"), self.trim, SystemTime::now());
        Ok(Released {
            slot: number,
            branch,
        })
    }

    /// The slot `holder` holds, if it holds one.
    pub fn held_by(&self, holder: &Holder) -> Option<usize> {
        self.bays().into_iter().find(|number| {
            Record::read(&self.record_path(*number)).is_some_and(|record| &record.holder == holder)
        })
    }

    /// Whether a lease for `holder` would take a slot now: one it holds, or
    /// one open and free, abandoned or not yet made. Read without the locks,
    /// as [`status`](Pool::status) is.
    pub fn open_for(&self, holder: &Holder) -> bool {
        self.held_by(holder).is_some()
            || self.bays().into_iter().any(|number| {
                self.leasable(number)
                    && matches!(
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
        let shape = self.shape();
        self.bays()
            .into_iter()
            .map(|number| Slot {
                number,
                path: self.path_of(number),
                state: self.state_of(number),
                closed: shape.closed.contains(&number),
            })
            .collect()
    }

    fn state_of(&self, number: usize) -> SlotState {
        self.probe_of(number).0
    }

    /// [`state_of`](Pool::state_of), and the `git status` it ran where it ran
    /// one, so a reader that also wants what the slot holds does not ask git
    /// for it again.
    fn probe_of(&self, number: usize) -> (SlotState, Option<Result<Vec<String>, String>>) {
        self.probe_trusting(number, None)
    }

    /// [`probe_of`](Pool::probe_of), taking `trusted` as what `git status`
    /// would say of a slot nobody holds, where the caller has a watch on its
    /// work tree that vouches for it. Everything else is read as ever: the
    /// record, HEAD and the refs, which are in-process and cheap.
    fn probe_trusting(
        &self,
        number: usize,
        trusted: Option<&Vec<String>>,
    ) -> (SlotState, Option<Result<Vec<String>, String>>) {
        let path = self.path_of(number);
        if !path.exists() {
            return (SlotState::Unmade, None);
        }
        if !git::is_checkout(&path) {
            return (SlotState::NotACheckout, None);
        }
        let record = Record::read(&self.record_path(number));
        if let Some(Record {
            branch,
            holder,
            since,
            ..
        }) = record.as_ref().filter(|record| record.holder.alive())
        {
            let held = SlotState::Held {
                branch: branch.clone(),
                holder: holder.clone(),
                since: *since,
            };
            return (held, None);
        }
        // **Asked of a slot with no record too.** A record lost or cut short
        // must not make a tree holding work read as free.
        let status = match trusted {
            Some(files) => Ok(files.clone()),
            None => git::dirty(&path),
        };
        let why = match &status {
            Err(why) => Some(why.clone()),
            Ok(files) if !files.is_empty() => {
                Some(format!("{} uncommitted, first {}", files.len(), files[0]))
            }
            // A released slot sits detached on a branch that still names its
            // commits, so a lease loses nothing unless HEAD is all that does.
            Ok(_) if record.is_none() => {
                let named = git::unnamed_commits_in_process(&path)
                    .unwrap_or_else(|| count(&path, &["HEAD", "--not", "--branches", "--remotes"]));
                match named {
                    0 => None,
                    commits => Some(format!("{commits} commits no branch names")),
                }
            }
            Ok(_) => match self.off_branch(&path, record.as_ref().map_or("", |r| &r.branch)) {
                0 => None,
                commits => Some(format!(
                    "{commits} commits on neither its branch, the remote nor {}",
                    self.base
                )),
            },
        };
        let (branch, since) = match record {
            Some(record) => (record.branch, record.since),
            None if why.is_none() => return (SlotState::Free, Some(status)),
            None => (
                git::read(&path, &["branch", "--show-current"]).unwrap_or_default(),
                0,
            ),
        };
        let state = match why {
            None => SlotState::Abandoned { branch, since },
            Some(why) => SlotState::Stranded { branch, since, why },
        };
        (state, Some(status))
    }

    /// Put slot `number` on `branch` (at the base if `onto` is new, else at
    /// its own tip), cleaned of all but the warm
    /// paths, and record who holds it. The caller holds the slot's lock.
    fn point(
        &self,
        number: usize,
        onto: Onto,
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
        match onto {
            // `--no-track`, so the new branch has no upstream and a bare `git
            // push` cannot reach the base.
            Onto::New => git(
                &path,
                &["switch", "--quiet", "--no-track", "-C", branch, from],
            ),
            Onto::Existing => git(&path, &["switch", "--quiet", branch]),
            Onto::Remote => git(
                &path,
                &[
                    "switch",
                    "--quiet",
                    "--track",
                    "-c",
                    branch,
                    &existing::remote(branch),
                ],
            ),
        }
        .map_err(LeaseRefused::Vcs)?;
        // A branch whose tip the base already holds has no commit to lose by
        // moving forward, so it starts at the base rather than where it was
        // cut. One with work of its own stays exactly where it is.
        if !matches!(onto, Onto::New)
            && git_ok(&path, &["merge-base", "--is-ancestor", "HEAD", from])
        {
            git(&path, &["merge", "--ff-only", "--quiet", from]).map_err(LeaseRefused::Vcs)?;
        }
        let mut clean = vec!["clean", "-fdx", "--quiet"];
        for kept in &self.keep {
            clean.extend(["-e", kept.as_str()]);
        }
        git(&path, &clean).map_err(LeaseRefused::Vcs)?;
        // A record still here is a holder that went without giving the slot back, and this lease
        // takes the slot from it.
        if let Some(gone) = Record::read(&self.record_path(number)) {
            self.noted_release(number, &gone);
        }
        let record = Record {
            branch: branch.to_string(),
            holder: holder.clone(),
            since,
            // The new holder's own, where the lease was asked for from an agent's command; the
            // last one's is gone with the old record, and `note_session` fills in the rest.
            session: self.session.clone().filter(|_| matches!(holder, Holder::Process { .. })),
        };
        record
            .write(&self.record_path(number))
            .map_err(LeaseRefused::Vcs)?;
        history::note(
            &path,
            &Turn {
                at_ms: history::now_ms(),
                leased: true,
                holder: record.holder,
                session: record.session,
                branch: Some(record.branch),
            },
        );
        Ok(Lease {
            slot: number,
            path,
            reclaimed_from: None,
            seeded_from,
            unfetched: None,
            made: false,
            cut: matches!(onto, Onto::New),
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
        let head = git::read(&self.root, &["rev-parse", &self.base_ref()]).ok();
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
        let there = git::ref_exists(&self.root, &remote)
            .unwrap_or_else(|| git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &remote]));
        match there {
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

    /// Write on slot `number`'s history that `record`'s holder no longer holds it.
    fn noted_release(&self, number: usize, record: &Record) {
        history::note(
            &self.path_of(number),
            &Turn {
                at_ms: history::now_ms(),
                leased: false,
                holder: record.holder.clone(),
                session: record.session.clone(),
                branch: None,
            },
        );
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

    /// Whether a take or a release has slot `number`'s lock, asked of the file
    /// as it stands: nothing is created, and the shared lock taken to ask is
    /// dropped at once. A reading is made of the slot while this is true only
    /// by what it is in the middle of, so it says Busy and no more.
    pub(super) fn under_way(&self, number: usize) -> bool {
        let Ok(file) = File::open(self.record_path(number)) else {
            return false;
        };
        matches!(file.try_lock_shared(), Err(TryLockError::WouldBlock))
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
