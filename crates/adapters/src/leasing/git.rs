//! The command-line git a slot is driven with, for the reason
//! `crate::reclaim`'s `porcelain` gives: a guard takes the answer a person
//! would be shown.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex, PoisonError};

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
    // A held repository outlives the `.git` it was opened from, so that is looked at first.
    if !path.join(".git").exists() {
        forget(path);
    }
    if let Some((repo, _)) = held(path) {
        let top = repo.lock().unwrap_or_else(PoisonError::into_inner).workdir().and_then(|top| top.canonicalize().ok());
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

/// A slot's repository, open between readings: a cold open (directory reads, packed refs) cost
/// more than the question asked of it, 1.6 ms of 2.8 ms. libgit2 reads refs from disk each time
/// and refreshes its pack list on an object miss, so a held one still sees a new commit, a moved
/// ref and a `git gc`. One lock per slot, since `Repository` is `Send` and not `Sync`.
static HELD: Mutex<BTreeMap<PathBuf, Arc<Mutex<git2::Repository>>>> = Mutex::new(BTreeMap::new());

/// The repository held for `at`, opened when none is, and whether it was held already.
fn held(at: &Path) -> Option<(Arc<Mutex<git2::Repository>>, bool)> {
    let mut all = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(repo) = all.get(at) {
        return Some((Arc::clone(repo), true));
    }
    let repo = Arc::new(Mutex::new(git2::Repository::open(at).ok()?));
    all.insert(at.to_path_buf(), Arc::clone(&repo));
    Some((repo, false))
}

/// Lets go of `at`'s held repository: its slot is gone, or the one held could not answer.
pub(super) fn forget(at: &Path) {
    HELD.lock().unwrap_or_else(PoisonError::into_inner).remove(at);
}

/// `git rev-list --left-right --count HEAD...<base>`: how far HEAD is ahead of
/// the base and behind it. The outer `None` is libgit2 unable to say; the inner
/// is git's own failure reading as unknown.
pub(super) fn apart_in_process(at: &Path, base: &str) -> Option<Option<(u32, u32)>> {
    let (kept, reused) = held(at)?;
    let answer = apart_of(&kept.lock().unwrap_or_else(PoisonError::into_inner), base);
    if answer.is_some() {
        return answer;
    }
    // A slot deleted and made again under the same path leaves the held one pointing at nothing.
    forget(at);
    if !reused {
        return None;
    }
    apart_of(&git2::Repository::open(at).ok()?, base)
}

fn apart_of(repo: &git2::Repository, base: &str) -> Option<Option<(u32, u32)>> {
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

    /// What `git rev-list --left-right --count HEAD...<base>` says, as `apart_in_process` returns it.
    fn rev_list_says(at: &Path, base: &str) -> Option<Option<(u32, u32)>> {
        let both = read(at, &["rev-list", "--left-right", "--count", &format!("HEAD...{base}")]).ok()?;
        let (ahead, behind) = both.split_once('\t')?;
        Some(Some((ahead.parse().ok()?, behind.parse().ok()?)))
    }

    /// `git init` at `at` with a `base` branch and `ahead` commits past it, as a slot is made.
    fn make_slot(at: &Path, ahead: u32) {
        let git = |args: &[&str]| {
            let run = Command::new("git")
                .arg("-C")
                .arg(at)
                .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
                .args(args)
                .output()
                .expect("git on PATH");
            assert!(run.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&run.stderr));
        };
        std::fs::create_dir_all(at).unwrap();
        git(&["init", "-q", "-b", "main"]);
        git(&["commit", "-q", "--allow-empty", "-m", "the base"]);
        git(&["branch", "base"]);
        for n in 0..ahead {
            git(&["commit", "-q", "--allow-empty", "-m", &format!("ahead {n}")]);
        }
    }

    /// A cold `Repository::open` fails once `.git/refs` is gone and an open one still answers, so
    /// taking it away is how a test sees that the slot's questions went to the repository held.
    /// What is held must still see the slot change, or the pool reads a slot as it was.
    #[test]
    fn a_slots_questions_go_to_the_repository_held_and_it_still_sees_the_slot_change() {
        let repo = TempRepo::with_a_commit();
        repo.git(&["branch", "base"]);
        repo.commit_one("a.txt", "a", "ahead one");
        repo.commit_one("b.txt", "b", "ahead two");
        let at = repo.root();
        let base = "refs/heads/base";

        assert_eq!(apart_in_process(at, base), Some(Some((2, 0))), "read once, opening it");
        assert!(is_checkout(at));

        repo.git(&["pack-refs", "--all"]);
        let refs = at.join(".git/refs");
        let away = at.join(".git/refs.away");
        std::fs::rename(&refs, &away).unwrap();
        assert_eq!(
            apart_in_process(at, base),
            Some(Some((2, 0))),
            "answered by the held repository: opening again fails with .git/refs gone"
        );
        assert!(is_checkout(at), "answered by the held repository: opening again fails with .git/refs gone");
        std::fs::rename(&away, &refs).unwrap();

        repo.commit_one("c.txt", "c", "ahead three");
        assert_eq!(apart_in_process(at, base), Some(Some((3, 0))), "a new commit is seen");
        assert_eq!(apart_in_process(at, base), rev_list_says(at, base));

        repo.git(&["gc", "--aggressive", "--prune=now", "-q"]);
        assert_eq!(apart_in_process(at, base), Some(Some((3, 0))), "the same after a gc");
        repo.commit_one("d.txt", "d", "ahead four");
        assert_eq!(apart_in_process(at, base), Some(Some((4, 0))), "a commit after a gc is seen");
        assert_eq!(apart_in_process(at, base), rev_list_says(at, base));

        // A base that has moved to a commit HEAD does not hold, so the slot is behind it as well.
        let side = repo.git(&["commit-tree", "HEAD~3^{tree}", "-p", "HEAD~3", "-m", "the base moved"]);
        repo.git(&["branch", "-f", "base", &side]);
        assert_eq!(apart_in_process(at, base), Some(Some((3, 1))), "a moved base ref is seen");
        assert_eq!(apart_in_process(at, base), rev_list_says(at, base));

        std::fs::remove_dir_all(at.join(".git")).unwrap();
        assert!(!is_checkout(at), "a directory whose .git was removed is no checkout, held repository or not");

        make_slot(at, 1);
        assert_eq!(apart_in_process(at, base), Some(Some((1, 0))), "a slot made again is read as itself");
        std::fs::remove_dir_all(at).unwrap();
        make_slot(at, 3);
        assert_eq!(
            apart_in_process(at, base),
            Some(Some((3, 0))),
            "a slot removed and made again under the same path is not read from the one that was held"
        );
    }
}
