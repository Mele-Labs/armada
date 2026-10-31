//! The command-line git a slot is driven with, for the reason
//! `crate::reclaim`'s `porcelain` gives: a guard takes the answer a person
//! would be shown.

use std::path::Path;
use std::process::{Command, Output};

fn run(at: &Path, args: &[&str]) -> std::io::Result<Output> {
    command(Command::new("git"), at, args).output()
}

/// `git` for arguments, at `at`, with no prompt. Takes the command so a
/// caller picks its priority: an act is at normal and a read is at utility,
/// which is what `crate::background` is for.
fn command(mut command: Command, at: &Path, args: &[&str]) -> Command {
    command
        .arg("-C")
        .arg(at)
        .args(args)
        // Nobody is at a terminal to answer a credential prompt.
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

/// [`run`] for a command that only reads. **`GIT_OPTIONAL_LOCKS=0`**: a
/// `git status` otherwise takes the index lock to refresh it, and an agent's
/// `git commit` in the same slot is then refused for it.
fn run_reading(at: &Path, args: &[&str]) -> std::io::Result<Output> {
    command(crate::background::command("git"), at, args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
}

/// What git printed, trimmed, or what it said when it refused.
pub(super) fn git(at: &Path, args: &[&str]) -> Result<String, String> {
    said(at, args, run(at, args))
}

/// [`git`] for a command that only reads, which leaves the index lock alone.
pub(super) fn read(at: &Path, args: &[&str]) -> Result<String, String> {
    said(at, args, run_reading(at, args))
}

fn said(at: &Path, args: &[&str], ran: std::io::Result<Output>) -> Result<String, String> {
    let ran = ran.map_err(|why| format!("git would not run: {why}"))?;
    if !ran.status.success() {
        return Err(format!(
            "`git {}` in {}: {}",
            args.join(" "),
            at.display(),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string())
}

/// Whether git answered at all.
pub(super) fn git_ok(at: &Path, args: &[&str]) -> bool {
    run_reading(at, args).is_ok_and(|ran| ran.status.success())
}

/// Whether `path` is the top of a checkout of its own, rather than a directory
/// inside the repository's main one.
pub(super) fn is_checkout(path: &Path) -> bool {
    // In process first: this runs for every slot on every read of the pool, and a
    // process is what costs the machine, not the question.
    if let Ok(repo) = git2::Repository::open(path) {
        let top = repo.workdir().and_then(|top| top.canonicalize().ok());
        return top.is_some() && top == path.canonicalize().ok();
    }
    let Ok(top) = read(path, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    Path::new(&top).canonicalize().is_ok_and(|top| top == path)
}

/// Whether `reference` exists in the repository at `at`. `None` where libgit2
/// could not say, so the caller asks git instead.
pub(super) fn ref_exists(at: &Path, reference: &str) -> Option<bool> {
    let repo = git2::Repository::open(at).ok()?;
    let found = repo.find_reference(reference).map(|_| ());
    match found {
        Ok(()) => Some(true),
        Err(why) if why.code() == git2::ErrorCode::NotFound => Some(false),
        Err(_) => None,
    }
}

/// `git rev-list --left-right --count HEAD...<base>`: how far HEAD is ahead of
/// the base and behind it. The outer `None` is libgit2 unable to say; the inner
/// is git's own failure reading as unknown.
pub(super) fn apart_in_process(at: &Path, base: &str) -> Option<Option<(u32, u32)>> {
    let repo = git2::Repository::open(at).ok()?;
    let head = match repo.head().and_then(|head| head.peel_to_commit()) {
        Ok(head) => head.id(),
        Err(_) => return None,
    };
    let base = match repo.revparse_single(base).and_then(|base| base.peel_to_commit()) {
        Ok(base) => base.id(),
        Err(why) if why.code() == git2::ErrorCode::NotFound => return Some(None),
        Err(_) => return None,
    };
    let (ahead, behind) = repo.graph_ahead_behind(head, base).ok()?;
    Some(u32::try_from(ahead).ok().zip(u32::try_from(behind).ok()))
}

/// `git rev-list --count HEAD --not --branches --remotes`: commits no branch
/// and no remote names. `None` where libgit2 could not say.
pub(super) fn unnamed_commits_in_process(at: &Path) -> Option<usize> {
    let repo = git2::Repository::open(at).ok()?;
    let mut walk = repo.revwalk().ok()?;
    walk.push_head().ok()?;
    walk.hide_glob("refs/heads/*").ok()?;
    walk.hide_glob("refs/remotes/*").ok()?;
    let mut counted = 0;
    for next in walk {
        next.ok()?;
        counted += 1;
    }
    Some(counted)
}

/// How many commits `rev-list` names for `revs`. A count git cannot give reads
/// as one: unknown is not landed.
pub(super) fn count(at: &Path, revs: &[&str]) -> usize {
    let mut args = vec!["rev-list", "--count"];
    args.extend_from_slice(revs);
    read(at, &args)
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(1)
}

/// Every path `git status` reports, untracked included and ignored not.
pub(super) fn dirty(at: &Path) -> Result<Vec<String>, String> {
    let said =
        run_reading(at, &["status", "--porcelain"]).map_err(|why| format!("git would not run: {why}"))?;
    if !said.status.success() {
        return Err(String::from_utf8_lossy(&said.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&said.stdout)
        .lines()
        .filter_map(|line| line.get(3..))
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::repo::TempRepo;

    /// The in-process answers must be the ones the command line gives, since
    /// they decide what a slot reads as holding.
    #[test]
    fn the_in_process_readings_match_what_git_says() {
        let repo = TempRepo::with_a_commit();
        repo.with_a_bare_remote();
        repo.git(&["push", "-q", "origin", "main"]);
        repo.git(&["branch", "feature/nested/one"]);
        repo.git(&["switch", "-q", "--detach"]);
        repo.commit_one("a.txt", "a", "on no branch");
        repo.commit_one("b.txt", "b", "also on no branch");
        let at = repo.root();

        assert_eq!(
            unnamed_commits_in_process(at),
            Some(count(at, &["HEAD", "--not", "--branches", "--remotes"]))
        );
        assert_eq!(unnamed_commits_in_process(at), Some(2));

        repo.git(&["branch", "feature/nested/two"]);
        assert_eq!(unnamed_commits_in_process(at), Some(0), "a nested branch names them");

        let base = "refs/remotes/origin/main";
        let said = read(at, &["rev-list", "--left-right", "--count", &format!("HEAD...{base}")]).ok();
        let said = said.and_then(|both| {
            let (ahead, behind) = both.split_once('\t')?;
            Some((ahead.parse().ok()?, behind.parse().ok()?))
        });
        assert_eq!(apart_in_process(at, base), Some(said));
        assert_eq!(apart_in_process(at, "refs/remotes/origin/nope"), Some(None));

        assert_eq!(ref_exists(at, base), Some(true));
        assert_eq!(ref_exists(at, "refs/remotes/origin/nope"), Some(false));

        assert!(is_checkout(at));
        std::fs::create_dir_all(at.join("inside")).unwrap();
        assert!(!is_checkout(&at.join("inside")), "a directory inside one is not its own");
    }
}
