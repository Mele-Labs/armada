//! Leasing a branch that already exists, at its tip, so work parked on it
//! continues where it stopped. A sibling of `leasing.rs`, which is past 500
//! lines; the slot-taking itself stays there and is shared, this adds only
//! what an existing branch is checked for.
//!
//! **A lease never moves a branch.** Whatever the caller asked for, a branch
//! that exists here or on `origin` is checked out as it stands, and only a
//! name nothing has is cut from the base.

use std::path::PathBuf;
use std::time::Duration;

use super::git::{git, git_ok};
use super::{Full, Holder, Lease, LeaseRefused, Leased, Pool, Seed};

/// Where a lease puts the branch.
#[derive(Clone, Copy)]
pub(super) enum Onto {
    /// What the caller named, if nothing has it: cut from the base. A name
    /// something has is resolved to one of the other two.
    New,
    /// A branch that exists here, at its tip.
    Existing,
    /// A branch only `origin` has, at its tip, tracking it.
    Remote,
}

impl Pool {
    /// Lease a slot onto the existing `branch` at its tip, waiting as long as
    /// it takes, as [`lease`](Pool::lease) does.
    pub fn lease_existing(
        &self,
        branch: &str,
        holder: &Holder,
        since: u64,
        seed: Seed<'_>,
        mut waiting: impl FnMut(&Full),
    ) -> Result<Lease, LeaseRefused> {
        loop {
            match self.take(Onto::Existing, branch, holder, since, seed)? {
                Leased::Took(lease) => return Ok(lease),
                Leased::Full(full) => waiting(&full),
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// Take a slot onto the existing `branch` now, or say why every one is
    /// unavailable.
    pub fn try_lease_existing(
        &self,
        branch: &str,
        holder: &Holder,
        since: u64,
        seed: Seed<'_>,
    ) -> Result<Leased, LeaseRefused> {
        if let Some(own) = self.handed_back(holder) {
            return Ok(Leased::Took(own));
        }
        self.take(Onto::Existing, branch, holder, since, seed)
    }

    /// Where a lease of `branch` puts it: on the branch as it stands if this
    /// repository has it, else on `origin`'s if that has it, else (for a
    /// caller who named a new branch) cut from the base. A caller who asked for
    /// an existing branch and finds none is refused.
    pub(super) fn resolved(&self, asked: Onto, branch: &str) -> Result<Onto, LeaseRefused> {
        if git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &local(branch)]) {
            return Ok(Onto::Existing);
        }
        // A branch pushed from another machine, or deleted here after a
        // merge, is still the branch's own work; a best-effort fetch finds it.
        if git_ok(&self.root, &["remote", "get-url", "origin"]) {
            let refspec = format!("+refs/heads/{branch}:{}", remote(branch));
            let _ = git(
                &self.root,
                &["fetch", "--quiet", "--no-tags", "origin", &refspec],
            );
        }
        if git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &remote(branch)]) {
            return Ok(Onto::Remote);
        }
        match asked {
            Onto::New => Ok(Onto::New),
            _ => Err(LeaseRefused::NoSuchBranch(branch.to_string())),
        }
    }

    /// The checkout that has `branch` checked out, if any.
    pub(super) fn checked_out(&self, branch: &str) -> Result<Option<PathBuf>, LeaseRefused> {
        let named = local(branch);
        let listed =
            git(&self.root, &["worktree", "list", "--porcelain"]).map_err(LeaseRefused::Vcs)?;
        let mut at = "";
        for line in listed.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                at = path;
            } else if line.strip_prefix("branch ") == Some(named.as_str()) {
                return Ok(Some(at.into()));
            }
        }
        Ok(None)
    }

    /// The number of the pool's slot at `at`, if it is one.
    pub(super) fn slot_at(&self, at: &std::path::Path) -> Option<usize> {
        let at = at.canonicalize().unwrap_or_else(|_| at.to_path_buf());
        self.bays().into_iter().find(|number| {
            let path = self.path_of(*number);
            path.canonicalize().unwrap_or(path) == at
        })
    }
}

pub(super) fn local(branch: &str) -> String {
    format!("refs/heads/{branch}")
}

pub(super) fn remote(branch: &str) -> String {
    format!("refs/remotes/origin/{branch}")
}
