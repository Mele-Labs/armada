//! Parking a slot's work on its branch: commit everything, free the slot,
//! push nothing. A sibling of `leasing.rs`, which is past 500 lines and whose
//! take, release and status stay whole; this holds only what is new to a
//! release that no longer needs a push.

use std::path::Path;

use super::git::{count, dirty, git, git_ok};
use super::{Committed, Holder, ParkRefused, Parked, Pool, Record};

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
    /// slot back. `holder` must hold it. A clean slot is only released. Nothing
    /// is pushed.
    pub fn park(&self, number: usize, holder: &Holder) -> Result<Parked, ParkRefused> {
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
        let on = git(&slot, &["branch", "--show-current"]).map_err(ParkRefused::Vcs)?;
        if on.is_empty() {
            return Err(ParkRefused::OnNoBranch);
        }
        if on == self.base {
            return Err(ParkRefused::OnTheBase(on));
        }
        if on != record.branch {
            return Err(ParkRefused::OnAnotherBranch {
                leased: record.branch,
                on,
            });
        }
        let files = dirty(&slot).map_err(ParkRefused::Vcs)?;
        let committed = if files.is_empty() {
            None
        } else {
            git(&slot, &["add", "--all"]).map_err(ParkRefused::Vcs)?;
            // `--no-verify`: work in progress being kept, not offered, and a
            // hook that refused it would leave it in a slot the person wants
            // freed.
            let message = format!(
                "WIP: uncommitted files of {}, saved to free slot-{number}",
                holder.said()
            );
            git(&slot, &["commit", "--quiet", "--no-verify", "-m", &message])
                .map_err(ParkRefused::Vcs)?;
            let commit = git(&slot, &["rev-parse", "HEAD"]).map_err(ParkRefused::Vcs)?;
            Some(Committed { commit, files })
        };
        let released = self
            .give_back_locked(number, Some(holder))
            .map_err(ParkRefused::Release)?;
        drop(lock);
        Ok(Parked {
            slot: number,
            branch: released.branch,
            committed,
        })
    }
}
