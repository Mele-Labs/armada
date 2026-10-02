//! The step onto a base: one `--no-ff` merge commit over a gated candidate,
//! pushed by whoever gated it and never forced.
//!
//! **One copy for both lines.** `armada land` lands this repository through
//! it, and `Delivery::merge_by_push` lands a Job's work through it where a
//! Manifest says `merge_by: push`. `docs/capabilities/merge-line.md`, *The
//! merge*.

use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

/// How many times a base may move under a gate before the landing gives up:
/// `armada land`'s turn, and Fleet's `merge_by: push`.
pub const ROUNDS: u32 = 5;

/// The message of the merge commit a branch lands as.
pub fn message(branch: &str, pull_request: Option<u64>) -> String {
    let subject = match pull_request {
        Some(number) => format!("Merge pull request #{number} from {branch}"),
        None => format!("Merge branch '{branch}'"),
    };
    format!("{subject}\n\nLanded-from: {branch}\n")
}

/// Why a merge commit was not made or not pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotOntoBase {
    /// The candidate does not hold the base, so a merge of it would land a
    /// combination nothing gated.
    DoesNotHoldBase { base: String, candidate: String },
    /// `git` would not run, or refused, and said this.
    Git(String),
}

impl fmt::Display for NotOntoBase {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotOntoBase::DoesNotHoldBase { base, candidate } => write!(
                out,
                "the candidate {} does not hold {}, so a merge of it would not be what was gated",
                short(candidate),
                short(base)
            ),
            NotOntoBase::Git(said) => out.write_str(said),
        }
    }
}

/// The commit `git merge --no-ff <candidate>` makes on `base`. The candidate
/// already holds `base`, so its tree is the merge's tree, and what is pushed
/// is exactly what was gated.
pub fn merge_commit(
    repo: &Path,
    base: &str,
    candidate: &str,
    message: &str,
) -> Result<String, NotOntoBase> {
    if !is_ancestor(repo, base, candidate) {
        return Err(NotOntoBase::DoesNotHoldBase {
            base: base.to_string(),
            candidate: candidate.to_string(),
        });
    }
    let tree = format!("{candidate}^{{tree}}");
    let args = [
        "commit-tree",
        &tree,
        "-p",
        base,
        "-p",
        candidate,
        "-m",
        message,
    ];
    let made = git(repo, &args)?;
    if !made.status.success() {
        return Err(refused(repo, &args, &made));
    }
    Ok(String::from_utf8_lossy(&made.stdout).trim().to_string())
}

/// What pushing a merge commit onto the base found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onto {
    Landed,
    /// The push was not a fast-forward: the base moved since it was gated.
    Moved,
}

/// Push `merge` onto `remote`'s `onto`, never forced: a base that moved since
/// `gated` refuses it, and that refusal is [`Onto::Moved`]. `ran` is handed
/// the push's own output, for a caller that keeps a log.
pub fn push(
    repo: &Path,
    remote: &str,
    onto: &str,
    merge: &str,
    gated: &str,
    ran: impl FnOnce(&[&str], &Output),
) -> Result<Onto, NotOntoBase> {
    let refspec = format!("{merge}:refs/heads/{onto}");
    let args = ["push", "--quiet", remote, &refspec];
    let pushed = git(repo, &args)?;
    let mut argv = vec!["git", "-C"];
    let shown = repo.to_string_lossy();
    argv.push(&shown);
    argv.extend(args);
    ran(&argv, &pushed);
    if pushed.status.success() {
        return Ok(Onto::Landed);
    }
    match remote_head(repo, remote, onto) {
        Some(now) if now == merge => Ok(Onto::Landed),
        Some(now) if now != gated => Ok(Onto::Moved),
        _ => Err(NotOntoBase::Git(format!(
            "the push of {onto} was refused, and {onto} has not moved: {}",
            String::from_utf8_lossy(&pushed.stderr).trim()
        ))),
    }
}

/// What `remote` holds for `branch`, or `None` where it has no such branch
/// or would not answer.
pub fn remote_head(repo: &Path, remote: &str, branch: &str) -> Option<String> {
    let listed = git(
        repo,
        &["ls-remote", remote, &format!("refs/heads/{branch}")],
    )
    .ok()?;
    if !listed.status.success() {
        return None;
    }
    String::from_utf8_lossy(&listed.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
}

/// Whether `ancestor` is reachable from `descendant`; `false` on any failure.
pub fn is_ancestor(repo: &Path, ancestor: &str, descendant: &str) -> bool {
    git(repo, &["merge-base", "--is-ancestor", ancestor, descendant])
        .map(|run| run.status.success())
        .unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> Result<Output, NotOntoBase> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|cause| {
            NotOntoBase::Git(format!(
                "`git {}` in {}: could not be run: {cause}",
                args.join(" "),
                repo.display()
            ))
        })
}

fn refused(repo: &Path, args: &[&str], run: &Output) -> NotOntoBase {
    let said = match run.stderr.is_empty() {
        true => &run.stdout,
        false => &run.stderr,
    };
    NotOntoBase::Git(format!(
        "`git {}` in {}: {}",
        args.join(" "),
        repo.display(),
        String::from_utf8_lossy(said).trim()
    ))
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}

#[cfg(test)]
mod tests {
    use super::message;

    #[test]
    fn a_landing_names_its_branch_in_a_trailer_and_its_pull_request_where_there_is_one() {
        assert_eq!(
            message("fix/one", Some(12)),
            "Merge pull request #12 from fix/one\n\nLanded-from: fix/one\n"
        );
        assert_eq!(
            message("fix/one", None),
            "Merge branch 'fix/one'\n\nLanded-from: fix/one\n"
        );
    }
}
