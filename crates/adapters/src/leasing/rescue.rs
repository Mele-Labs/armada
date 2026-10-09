//! A stranded slot rescued: what it holds read out, then its work scrapped or
//! stashed, and the slot freed. A person asks and Fleet acts; a Scout only
//! ever reads. `docs/concepts/fleet.md`, *Rescuing a stranded slot*.
//!
//! **Under the slot's own lock**, as a take or a release is, and the slot is
//! asked again under it whether it is still stranded.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use adapter_traits::{
    CommitHome, RescueRefused, SlotCommit, SlotRescue, SlotRescued, StrandedWork,
};

use super::git::{count, dirty, git, git_ok};
use super::{Holder, Pool, Record, SlotState};

/// How many of a branch's commits a reading lists. The rest are still counted
/// in `unpushed`.
const COMMITS_LISTED: usize = 50;

impl Pool {
    /// What stranded slot `number` holds.
    pub fn stranded_work(&self, number: usize) -> Result<StrandedWork, RescueRefused> {
        let path = self.stranded(number)?;
        self.work_at(&path)
    }

    /// What slot `number` holds while an agent session holds it. **Read only**:
    /// `stranded` still refuses it, so no rescue act reaches a live holder's
    /// checkout.
    pub fn session_work(&self, number: usize) -> Result<StrandedWork, RescueRefused> {
        if !self.bays().contains(&number) {
            return Err(RescueRefused::NoSuchSlot(number as u32));
        }
        match self.state_of(number) {
            SlotState::Held {
                holder: Holder::Process { .. },
                ..
            } => self.work_at(&self.path_of(number)),
            other => Err(RescueRefused::NotStranded(word(&other).to_string())),
        }
    }

    fn work_at(&self, path: &Path) -> Result<StrandedWork, RescueRefused> {
        let path = path.to_path_buf();
        let commit = git(&path, &["rev-parse", "HEAD"]).map_err(RescueRefused::Vcs)?;
        let uncommitted = dirty(&path).map_err(RescueRefused::Vcs)?;
        let range = format!("{}..HEAD", self.base_ref());
        let listed = COMMITS_LISTED.to_string();
        let log =
            git(&path, &["log", "--format=%H%x1f%s", "-n", &listed, &range]).unwrap_or_default();
        let (not_on_remote, not_on_main) = self.homes_of(&path, &range);
        let commits = log
            .lines()
            .filter_map(|line| line.split_once('\u{1f}'))
            .map(|(sha, subject)| SlotCommit {
                sha: sha.to_string(),
                subject: subject.to_string(),
                home: home_of(sha, &not_on_remote, &not_on_main),
            })
            .collect();
        Ok(StrandedWork {
            branch: on_branch(&path),
            commit,
            uncommitted,
            commits,
            unpushed: self.unlanded(&path, "HEAD") as u32,
        })
    }

    /// The commits of `range` that no remote branch holds, and that the local
    /// base does not, each from one question rather than one per commit. A
    /// question git cannot answer is `None`, which [`home_of`] reads as only
    /// here, because unknown is not landed.
    fn homes_of(&self, at: &Path, range: &str) -> (Option<HashSet<String>>, Option<HashSet<String>>) {
        let shas = |args: &[&str]| {
            git(at, args)
                .ok()
                .map(|said| said.lines().map(str::to_string).collect::<HashSet<_>>())
        };
        let local = format!("refs/heads/{}", self.base);
        (
            shas(&["rev-list", range, "--not", "--remotes"]),
            shas(&["rev-list", range, "--not", &local]),
        )
    }

    /// Stranded slot `number`'s change against where it left the base,
    /// uncommitted changes to tracked files included.
    pub fn stranded_diff(&self, number: usize) -> Result<String, RescueRefused> {
        let path = self.stranded(number)?;
        let since = git(&path, &["merge-base", "HEAD", &self.base_ref()])
            .unwrap_or_else(|_| String::from("HEAD"));
        git(&path, &["diff", &since]).map_err(RescueRefused::Vcs)
    }

    /// Scrap or stash stranded slot `number`'s work, and free it.
    pub fn rescue(&self, number: usize, rescue: SlotRescue) -> Result<SlotRescued, RescueRefused> {
        if !self.bays().contains(&number) {
            return Err(RescueRefused::NoSuchSlot(number as u32));
        }
        let Some(lock) = self.locked(number).map_err(RescueRefused::Vcs)? else {
            return Err(RescueRefused::Busy);
        };
        let path = self.stranded(number)?;
        let branch = on_branch(&path);
        let rescued = match rescue {
            SlotRescue::Scrap => self.scrapped(&path, branch)?,
            SlotRescue::Stash { message } => self.stashed(&path, branch, &message)?,
        };
        Record::clear(&self.record_path(number)).map_err(RescueRefused::Vcs)?;
        drop(lock);
        Ok(rescued)
    }

    /// The slot's path, refused unless it is one of the pool's and stranded, or
    /// held by a Job whose release was refused.
    fn stranded(&self, number: usize) -> Result<PathBuf, RescueRefused> {
        if !self.bays().contains(&number) {
            return Err(RescueRefused::NoSuchSlot(number as u32));
        }
        match self.state_of(number) {
            SlotState::Stranded { .. } => Ok(self.path_of(number)),
            // A Job that ended and could not give its slot back: the work is
            // in it as in a stranded one, and the act ends the Job's claim.
            SlotState::Held { kept: Some(_), .. } => Ok(self.path_of(number)),
            other => Err(RescueRefused::NotStranded(word(&other).to_string())),
        }
    }

    /// Discard everything uncommitted, put the checkout at the base, and
    /// delete the branch only where the base already has every commit on it.
    fn scrapped(&self, path: &Path, branch: Option<String>) -> Result<SlotRescued, RescueRefused> {
        git(path, &["reset", "--hard", "--quiet"]).map_err(RescueRefused::Vcs)?;
        let mut clean = vec!["clean", "-fdx", "--quiet"];
        for kept in &self.keep {
            clean.extend(["-e", kept.as_str()]);
        }
        git(path, &clean).map_err(RescueRefused::Vcs)?;
        git(path, &["switch", "--detach", "--quiet", &self.base_ref()])
            .map_err(RescueRefused::Vcs)?;
        let branch_kept = match branch.as_deref() {
            Some(name) if name != self.base => {
                let local = format!("refs/heads/{name}");
                if self.merged(&local) {
                    git(&self.root, &["branch", "-D", "--quiet", name])
                        .map_err(RescueRefused::Vcs)?;
                    false
                } else {
                    true
                }
            }
            _ => false,
        };
        Ok(SlotRescued {
            branch,
            branch_kept,
            committed: None,
        })
    }

    /// Commit what is uncommitted to the branch, push the branch under its
    /// own name, and put the checkout at the base.
    fn stashed(
        &self,
        path: &Path,
        branch: Option<String>,
        message: &str,
    ) -> Result<SlotRescued, RescueRefused> {
        let Some(name) = branch.clone() else {
            return Err(RescueRefused::OnNoBranch);
        };
        if name == self.base {
            return Err(RescueRefused::OnTheBase(name));
        }
        if !git_ok(&self.root, &["remote", "get-url", "origin"]) {
            return Err(RescueRefused::NoRemote);
        }
        let committed = if dirty(path).map_err(RescueRefused::Vcs)?.is_empty() {
            None
        } else {
            git(path, &["add", "--all"]).map_err(RescueRefused::Vcs)?;
            // `--no-verify`: this is work in progress being kept, not offered,
            // and a hook that refused it would leave it where it was stranded.
            git(path, &["commit", "--quiet", "--no-verify", "-m", message])
                .map_err(RescueRefused::Vcs)?;
            Some(git(path, &["rev-parse", "HEAD"]).map_err(RescueRefused::Vcs)?)
        };
        let to = format!("HEAD:refs/heads/{name}");
        git(path, &["push", "--quiet", "--no-verify", "origin", &to])
            .map_err(RescueRefused::Vcs)?;
        git(path, &["switch", "--detach", "--quiet", &self.base_ref()])
            .map_err(RescueRefused::Vcs)?;
        Ok(SlotRescued {
            branch,
            branch_kept: true,
            committed,
        })
    }

    /// Whether the base, here or on the remote, has every commit on `rev`.
    fn merged(&self, rev: &str) -> bool {
        let base = self.base_ref();
        let local = format!("refs/heads/{}", self.base);
        let mut args = vec![rev, "--not", base.as_str()];
        if git_ok(&self.root, &["rev-parse", "--verify", "--quiet", &local]) {
            args.push(&local);
        }
        count(&self.root, &args) == 0
    }
}

/// Where else `sha` exists: a remote branch first, then the local base.
fn home_of(
    sha: &str,
    not_on_remote: &Option<HashSet<String>>,
    not_on_main: &Option<HashSet<String>>,
) -> CommitHome {
    let held_by = |missing: &Option<HashSet<String>>| missing.as_ref().is_some_and(|m| !m.contains(sha));
    if held_by(not_on_remote) {
        return CommitHome::OnRemote;
    }
    if held_by(not_on_main) {
        return CommitHome::OnMain;
    }
    CommitHome::OnlyHere
}

/// The branch checked out, or `None` for a detached checkout.
fn on_branch(path: &Path) -> Option<String> {
    git(path, &["branch", "--show-current"])
        .ok()
        .filter(|name| !name.is_empty())
}

/// What a slot is, where it is not stranded.
fn word(state: &SlotState) -> &'static str {
    match state {
        SlotState::Unmade => "not made",
        SlotState::NotACheckout => "not a checkout",
        SlotState::Busy => "busy",
        SlotState::Free | SlotState::Abandoned { .. } => "free",
        SlotState::Held { .. } => "held",
        SlotState::Stranded { .. } => "stranded",
    }
}
