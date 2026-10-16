//! Parking a slot's work on its branch: commit everything, free the slot,
//! push nothing. A sibling of `leasing.rs`, which is past 500 lines and whose
//! take, release and status stay whole; this holds only what is new to a
//! release that no longer needs a push.

use std::path::Path;

use super::git::{count, dirty, git_ok, read};
use super::saving::{commit_all, cut_kept_branch};
use super::strays;
use super::{Holder, ParkRefused, Parked, Pool, Record};

/// Whose rules a park follows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tenant {
    /// A Job, or a person's process.
    Job,
    /// A session that has ended: what it left running in the slot is ended
    /// first.
    Session,
}

impl Pool {
    /// Commits reachable from `HEAD` in `at` that `branch`, the remotes and
    /// the local base all lack. Those are the ones a detach would leave on no
    /// branch. A count git cannot give reads as one: unknown is not saved.
    pub(super) fn off_branch(&self, at: &Path, branch: &str) -> usize {
        let local = format!("refs/heads/{}", self.base);
        let own = format!("refs/heads/{branch}");
        let mut args = vec!["HEAD", "--not", "--remotes"];
        for named in [&local, &own] {
            if git_ok(at, &["rev-parse", "--verify", "--quiet", named]) {
                args.push(named);
            }
        }
        count(at, &args)
    }

    /// Commits on slot `number`'s checkout that are on no remote and not on the
    /// base, whatever its own branch holds. A caller about to delete that
    /// branch asks this, because a release no longer does.
    pub fn unlanded_in(&self, number: usize) -> usize {
        self.unlanded(&self.path_of(number), "HEAD")
    }

    /// Commit all of slot `number`'s uncommitted work, untracked files
    /// included and ignored ones not, to the branch it is on, and give the
    /// slot back. `holder` must hold it. A clean slot is only released. A
    /// checkout on no branch or on the base, holding files or commits no
    /// branch has, gets a `kept/` branch cut at `HEAD` for them. Nothing is
    /// pushed.
    pub fn park(&self, number: usize, holder: &Holder) -> Result<Parked, ParkRefused> {
        self.park_named(number, holder, &holder.said())
    }

    /// [`park_named`](Pool::park_named) for a slot whose holder is a process
    /// that has ended, the one it was leased by, so nothing can still be
    /// writing into it. **A holder still running is refused**, never taken:
    /// it is somebody else's, or the same process not yet gone.
    pub fn park_left(&self, number: usize, whose: &str) -> Result<Parked, ParkRefused> {
        let Some(record) = Record::read(&self.record_path(number)) else {
            return Err(ParkRefused::NotLeased(self.path_of(number)));
        };
        match &record.holder {
            holder @ Holder::Process { .. } if !holder.alive() => {
                self.parked(number, holder, whose, Tenant::Session)
            }
            holder => Err(ParkRefused::HeldByAnother(holder.said())),
        }
    }

    /// [`park`](SlotPool::park), with the WIP commit naming `whose` work it
    /// is where the holder's own name would say the wrong thing: a Session
    /// holds its slot as a `Holder::Job` of its own id.
    pub fn park_named(
        &self,
        number: usize,
        holder: &Holder,
        whose: &str,
    ) -> Result<Parked, ParkRefused> {
        self.parked(number, holder, whose, Tenant::Job)
    }

    /// [`park_named`](Pool::park_named) for a hosted session that has ended:
    /// committed to whatever branch the slot is on, and with anything still
    /// running in the slot ended first.
    pub fn park_session(
        &self,
        number: usize,
        holder: &Holder,
        whose: &str,
    ) -> Result<Parked, ParkRefused> {
        self.parked(number, holder, whose, Tenant::Session)
    }

    fn parked(
        &self,
        number: usize,
        holder: &Holder,
        whose: &str,
        tenant: Tenant,
    ) -> Result<Parked, ParkRefused> {
        let slot = self.path_of(number);
        if !self.bays().contains(&number) || !slot.exists() {
            return Err(ParkRefused::NotASlot(slot));
        }
        let Some(lock) = self.locked(number).map_err(ParkRefused::Vcs)? else {
            return Err(ParkRefused::Busy);
        };
        let Some(record) = Record::read(&self.record_path(number)) else {
            return Err(ParkRefused::NotLeased(slot));
        };
        if record.holder != *holder {
            return Err(ParkRefused::HeldByAnother(record.holder.said()));
        }
        let on = read(&slot, &["branch", "--show-current"]).map_err(ParkRefused::Vcs)?;
        if tenant == Tenant::Session {
            strays::end_in(&slot);
        }
        // Where the work goes. A branch of its own, whichever it is, takes it.
        // None, or the base, which a WIP commit must not land on, gets a new
        // branch only if there is something a detach would lose.
        let target = if on.is_empty() || on == self.base {
            let files = dirty(&slot).map_err(ParkRefused::Vcs)?;
            if files.is_empty() && self.off_branch(&slot, &record.branch) == 0 {
                None
            } else {
                let stem = format!("slot-{number}-{}", holder.short());
                Some(cut_kept_branch(&slot, &stem).map_err(ParkRefused::Vcs)?)
            }
        } else {
            Some(on)
        };
        let message = format!("WIP: uncommitted files of {whose}, saved to free slot-{number}");
        let committed = commit_all(&slot, &message).map_err(ParkRefused::Vcs)?;
        let released = self
            .give_back_locked(number, Some(holder), target.as_deref())
            .map_err(ParkRefused::Release)?;
        drop(lock);
        Ok(Parked {
            slot: number,
            branch: released.branch,
            committed,
        })
    }
}
