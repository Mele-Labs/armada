//! Leasing a branch that already exists, at its tip, so work parked on it
//! continues where it stopped. A sibling of `leasing.rs`, which is past 500
//! lines; the slot-taking itself stays there and is shared, this adds only
//! what an existing branch is checked for.

use std::time::Duration;

use super::git::{git, git_ok};
use super::{Full, Holder, Lease, LeaseRefused, Leased, Pool, Seed};

/// Where a lease puts the branch.
#[derive(Clone, Copy)]
pub(super) enum Onto {
    /// A new branch cut from the base.
    New,
    /// A branch that exists, at its tip.
    Existing,
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
            match self.try_lease_existing(branch, holder, since, seed)? {
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
        self.take(Onto::Existing, branch, holder, since, seed)
    }

    /// Refuse a branch that does not exist or that a checkout already has.
    /// Git refuses the second too, but only with a path in its sentence.
    pub(super) fn existing_free(&self, branch: &str) -> Result<(), LeaseRefused> {
        let named = format!("refs/heads/{branch}");
        if !git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &named]) {
            return Err(LeaseRefused::NoSuchBranch(branch.to_string()));
        }
        let listed =
            git(&self.root, &["worktree", "list", "--porcelain"]).map_err(LeaseRefused::Vcs)?;
        let mut at = "";
        for line in listed.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                at = path;
            } else if line.strip_prefix("branch ") == Some(named.as_str()) {
                return Err(LeaseRefused::CheckedOutElsewhere {
                    branch: branch.to_string(),
                    at: at.into(),
                });
            }
        }
        Ok(())
    }
}
