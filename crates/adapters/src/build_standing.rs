//! Where a built commit stands against `origin/main`, and the fetch that keeps
//! `origin/main` current, and whether a session's branch is in a build. Read by Fleet's
//! `get_fleet_build` and its build notices.
//!
//! **Counts, not a verdict.** `origin/main...commit` gives both sides of the
//! symmetric difference at once: what the build holds that `origin/main` lacks
//! and what `origin/main` holds that the build lacks. A preview is ahead by its
//! merged branches and behind by whatever landed after it was cut.

use std::io::Read;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::delivery::read_in;

const POLL: Duration = Duration::from_millis(50);

/// `(ahead, behind)` of `commit` against `origin/main`, or `None` where either
/// is not in the repository at `root` or git would not say.
pub fn position_against_main(root: &str, commit: &str) -> Option<(u32, u32)> {
    // `^{commit}` so a ref that is not a commit, and an empty string, are not counted.
    let named = format!("{commit}^{{commit}}");
    let held = read_in(root, "git", &["rev-parse", "--verify", "--quiet", &named]).ok()?;
    if !held.status.success() {
        return None;
    }
    let range = format!("origin/main...{commit}");
    let counted = read_in(root, "git", &["rev-list", "--left-right", "--count", &range]).ok()?;
    if !counted.status.success() {
        return None;
    }
    let said = String::from_utf8_lossy(&counted.stdout);
    let (behind, ahead) = said.trim().split_once('\t')?;
    Some((ahead.trim().parse().ok()?, behind.trim().parse().ok()?))
}

/// Where a session's branch stands against a build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchInBuild {
    /// Every commit on the branch is in the build, up to its `tip`.
    In { tip: String },
    /// None of the branch's own commits are.
    NotIn,
    /// Some are: `newest` is the newest of the branch's own commits the build holds, and
    /// `since` is how many commits on the branch it lacks.
    Partly { newest: String, since: u32 },
}

/// Whether `branch` is in the build at `build`, counted from the branch's own commits past
/// `origin/main`. **`None` says nothing**: `main`, a branch with no commits of its own, or a
/// branch, build or `origin/main` this repository does not hold.
pub fn branch_in_build(root: &str, build: &str, branch: &str) -> Option<BranchInBuild> {
    if branch == "main" {
        return None;
    }
    let tip = [format!("refs/heads/{branch}"), format!("refs/remotes/origin/{branch}")]
        .iter()
        .find_map(|name| commits(root, &["rev-parse", "--verify", "--quiet", &format!("{name}^{{commit}}")]))?
        .pop()?;
    let own = commits(root, &["rev-list", &format!("origin/main..{tip}")])?;
    if own.is_empty() {
        return None;
    }
    // Past `origin/main` too, so a newer main merged into the branch is not counted as its own.
    let lacking = commits(root, &["rev-list", &tip, &format!("^{build}"), "^origin/main"])?;
    if lacking.is_empty() {
        return Some(BranchInBuild::In { tip });
    }
    Some(match own.iter().find(|one| !lacking.contains(one)) {
        None => BranchInBuild::NotIn,
        Some(newest) => BranchInBuild::Partly { newest: newest.clone(), since: lacking.len() as u32 },
    })
}

/// The commit `refs/heads/<branch>` names in `root`, or `None` where there is no such branch.
pub fn branch_tip(root: &str, branch: &str) -> Option<String> {
    commits(root, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}^{{commit}}")])?.pop()
}

/// The lines git printed for these arguments in `root`, or `None` where it failed.
fn commits(root: &str, args: &[&str]) -> Option<Vec<String>> {
    let run = read_in(root, "git", args).ok().filter(|run| run.status.success())?;
    Some(String::from_utf8_lossy(&run.stdout).lines().map(str::to_string).collect())
}

/// `git fetch origin main` in `root`, stopped if it is still running after
/// `within`. **No prompt**: Fleet has no terminal to answer one on.
pub fn fetch_main(root: &str, within: Duration) -> Result<(), String> {
    fetch_within(root, &["fetch", "--quiet", "origin", "main"], within)
}

/// `git fetch origin <branch>` in `root`, for a commit of the base that is not here yet.
pub fn fetch_branch(root: &str, branch: &str, within: Duration) -> Result<(), String> {
    fetch_within(root, &["fetch", "--quiet", "origin", branch], within)
}

/// `git fetch origin pull/<number>/head` in `root`: the pull request's current head, reachable here
/// afterwards (by `FETCH_HEAD`, and by its id).
pub fn fetch_pull_head(root: &str, number: u64, within: Duration) -> Result<(), String> {
    fetch_within(root, &["fetch", "--quiet", "origin", &format!("pull/{number}/head")], within)
}

/// Run `git` with these arguments (a fetch) in `root`, stopped if it is still
/// running after `within`. Errors with git's own sentence. **No prompt.**
pub(crate) fn fetch_within(root: &str, args: &[&str], within: Duration) -> Result<(), String> {
    let mut child = crate::background::command("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .spawn()
        .map_err(|why| why.to_string())?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    // Read on its own thread so a chatty git cannot fill the pipe and stall.
    let reading = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stderr.read_to_string(&mut said);
        said
    });
    let deadline = Instant::now() + within;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err(reading.join().unwrap_or_default().trim().to_string()),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("git fetch took longer than {}s", within.as_secs()));
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(why) => return Err(why.to_string()),
        }
    }
}
