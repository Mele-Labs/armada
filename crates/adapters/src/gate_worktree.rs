//! The one detached worktree per repository that Fleet runs a plan gate's Checks in, and the git
//! readings that choose what to run there. `docs/concepts/session/the-plan.md`, *Gates*.
//!
//! **Kept between runs**, so its `target/` stays warm: a run checks out the commit it is for and
//! nothing is cleaned, ignored files included.

use std::path::{Path, PathBuf};

use crate::territory::{git, NotRead};

/// Where the worktree stands, below the repository root.
const AT: &str = ".armada/gate-check";

/// The commit `sha` and the base branch last met at. The remote's copy of `base` is read first, since
/// the local branch may be behind it.
pub fn met_at(dir: &Path, base: &str, sha: &str) -> Result<String, NotRead> {
    git(dir, &["merge-base", &format!("origin/{base}"), sha])
        .or_else(|_| git(dir, &["merge-base", base, sha]))
        .map(|met| met.trim().to_string())
}

/// The paths changed between `met` and `sha`, repository-relative and sorted. A rename is both its paths.
pub fn changed_between(dir: &Path, met: &str, sha: &str) -> Result<Vec<String>, NotRead> {
    let listed = git(dir, &["--no-optional-locks", "diff", "--name-only", "--no-renames", "-z", &format!("{met}...{sha}")])?;
    let mut paths: Vec<String> = listed.split('\0').filter(|name| !name.is_empty()).map(str::to_string).collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// The repository's gate-check worktree, made on first use and left at `sha`. `root` is the
/// repository's own checkout.
pub fn checked_out(root: &Path, sha: &str) -> Result<PathBuf, NotRead> {
    let at = root.join(AT);
    if !at.join(".git").exists() {
        // A folder left from a worktree git no longer lists would be refused.
        git(root, &["worktree", "prune"])?;
        git(root, &["worktree", "add", "--detach", "--force", &at.to_string_lossy(), sha])?;
    }
    git(&at, &["checkout", "--detach", "--force", sha])?;
    Ok(at)
}
