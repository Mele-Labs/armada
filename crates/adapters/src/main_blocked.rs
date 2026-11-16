//! Untracked files that stop a checkout's `main` fast-forwarding to `origin/main`, and the clearing
//! that lets it. Read by Fleet's `get_fleet_build`; cleared by `change_fleet_build`'s `clear`.
//!
//! **Local refs only.** `origin/main` is where the last fetch left it, so a read never waits on the
//! network. A file is in the way when the checkout has it untracked and `origin/main` tracks it: git
//! refuses to overwrite it, so `git pull --ff-only` in `scripts/preview` and `scripts/restart` fails.

use std::collections::HashSet;
use std::time::Duration;

use crate::build_standing::fetch_within;
use crate::delivery::read_in;

/// A checkout on `main` that is behind `origin/main` and holds untracked files `origin/main` tracks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainBlocked {
    /// Commits `origin/main` holds that local `main` lacks.
    pub behind: u32,
    /// The untracked files `origin/main` tracks, relative to the checkout, sorted.
    pub files: Vec<String>,
}

/// What blocks `main` at `root` from fast-forwarding. **`None` unless the checkout is on `main`, is
/// behind, and holds at least one such file**, and where git would not say. Cheapest test first, so
/// a checkout level with `origin/main` never walks its tree.
pub fn main_blocked(root: &str) -> Option<MainBlocked> {
    if lines(root, &["branch", "--show-current"])?.first().map(String::as_str) != Some("main") {
        return None;
    }
    let behind: u32 = lines(root, &["rev-list", "--count", "refs/heads/main..refs/remotes/origin/main"])?
        .first()?
        .trim()
        .parse()
        .ok()?;
    if behind == 0 {
        return None;
    }
    let untracked = zeroed(root, &["ls-files", "-z", "--others", "--exclude-standard"])?;
    if untracked.is_empty() {
        return None;
    }
    let tracked: HashSet<String> = zeroed(root, &["ls-tree", "-r", "-z", "--name-only", "refs/remotes/origin/main"])?.into_iter().collect();
    let mut files: Vec<String> = untracked.into_iter().filter(|file| tracked.contains(file)).collect();
    files.sort();
    (!files.is_empty()).then_some(MainBlocked { behind, files })
}

/// `git stash push --include-untracked -m <message> -- <files>` at `root`. Errors with git's line.
pub fn stash_files(root: &str, files: &[String], message: &str) -> Result<(), String> {
    let mut args = vec!["stash", "push", "--include-untracked", "-m", message, "--"];
    args.extend(files.iter().map(String::as_str));
    let run = read_in(root, "git", &args).map_err(|why| why.to_string())?;
    if run.status.success() {
        Ok(())
    } else {
        Err(first_line(&run.stderr).unwrap_or_else(|| format!("git stash exited {}", run.status.code().unwrap_or(-1))))
    }
}

/// Remove each of `files`, relative to `root`. **Stops at the first that will not go**, with its
/// name and why.
pub fn delete_files(root: &str, files: &[String]) -> Result<(), String> {
    let root = std::path::Path::new(root);
    for file in files {
        std::fs::remove_file(root.join(file)).map_err(|why| format!("{file}: {why}"))?;
    }
    Ok(())
}

/// `git pull --ff-only --quiet origin main` at `root`, stopped after `within`. Errors with git's
/// line. **No prompt.**
pub fn pull_main(root: &str, within: Duration) -> Result<(), String> {
    fetch_within(root, &["pull", "--ff-only", "--quiet", "origin", "main"], within)
        .map_err(|said| said.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or(&said).to_string())
}

fn lines(root: &str, args: &[&str]) -> Option<Vec<String>> {
    let run = read_in(root, "git", args).ok().filter(|run| run.status.success())?;
    Some(String::from_utf8_lossy(&run.stdout).lines().map(str::to_string).collect())
}

/// NUL-separated paths, so a name with a newline or a quote is read as git holds it.
fn zeroed(root: &str, args: &[&str]) -> Option<Vec<String>> {
    let run = read_in(root, "git", args).ok().filter(|run| run.status.success())?;
    Some(
        run.stdout
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect(),
    )
}

fn first_line(said: &[u8]) -> Option<String> {
    String::from_utf8_lossy(said).lines().map(str::trim).find(|line| !line.is_empty()).map(str::to_string)
}
