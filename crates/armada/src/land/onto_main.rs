//! The step onto `main`: one `--no-ff` merge commit of the gated candidate,
//! pushed by the runner itself, then the branch's remote copy and pull
//! request tidied after it. `docs/capabilities/merge-line.md`, *The merge*.

use std::path::Path;
use std::time::{Duration, Instant};

use super::dir::StateDir;
use super::env::Env;
use super::git::{best_effort, checked};
use super::outcome::{OutcomePatch, OutcomeState};
use super::queue::QueueEntry;
use super::repo::{is_ancestor, remote_head, rev_parse};
use super::shell::{gh_view, run};
use super::stop::Stopped;

/// The message of the merge commit a branch lands as.
pub fn message(branch: &str, pull_request: Option<u64>) -> String {
    let subject = match pull_request {
        Some(number) => format!("Merge pull request #{number} from {branch}"),
        None => format!("Merge branch '{branch}'"),
    };
    format!("{subject}\n\nLanded-from: {branch}\n")
}

/// The commit `git merge --no-ff <candidate>` makes on `base`. The candidate
/// already holds `base`, so its tree is the merge's tree, and what is pushed
/// is exactly what was gated.
pub fn merge_commit(
    repo: &Path,
    base: &str,
    candidate: &str,
    message: &str,
) -> Result<String, Stopped> {
    if !is_ancestor(repo, base, candidate) {
        return Err(Stopped::stopped(format!(
            "the candidate {} does not hold {}, so a merge of it would not be what was gated",
            short(candidate),
            short(base)
        )));
    }
    let tree = format!("{candidate}^{{tree}}");
    let made = checked(
        repo,
        &[
            "commit-tree",
            &tree,
            "-p",
            base,
            "-p",
            candidate,
            "-m",
            message,
        ],
    )?;
    Ok(String::from_utf8_lossy(&made.stdout).trim().to_string())
}

/// What pushing a merge commit onto the base found.
pub enum Pushed {
    Landed,
    /// The push was not a fast-forward: the base moved, so the turn gates again.
    Moved,
}

/// Push `merge` onto the base, never forced: a base that moved since `base`
/// refuses it, and that refusal is what makes the turn gate again.
pub fn push(
    repo: &Path,
    env: &Env,
    merge: &str,
    base: &str,
    log: &Path,
) -> Result<Pushed, Stopped> {
    let refspec = format!("{merge}:refs/heads/{}", env.base);
    let pushed = run(
        &[
            "git",
            "-C",
            &repo.to_string_lossy(),
            "push",
            "--quiet",
            &env.remote,
            &refspec,
        ],
        repo,
        None,
        Some(log),
    )?;
    if pushed.success() {
        return Ok(Pushed::Landed);
    }
    match remote_head(repo, &env.remote, &env.base) {
        Ok(Some(now)) if now == merge => Ok(Pushed::Landed),
        Ok(Some(now)) if now != base => Ok(Pushed::Moved),
        _ => Err(Stopped::stopped(format!(
            "the push of {} was refused, and {} has not moved: {}",
            env.base,
            env.base,
            pushed.stderr().trim()
        ))),
    }
}

/// The turn's last act once `merge` is on the base: drop the stamp, settle
/// the pull request, and delete the branch's remote copy where everything on
/// it landed.
pub fn landed(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    entry: &QueueEntry,
    base: &str,
    merge: &str,
) -> Stopped {
    let _ = std::fs::remove_file(state.stamp_path(&entry.branch));
    let branch = entry.branch.as_str();
    let mut detail = format!(
        "merged as {} onto {}, the base it was gated against",
        short(merge),
        short(base)
    );
    let remote = remote_head(repo, &env.remote, branch).ok().flatten();
    match remote {
        Some(at) if !is_ancestor(repo, &at, merge) => {
            detail.push_str(&format!(
                ". {}/{branch} is at {}, which did not land, so it stays{}",
                env.remote,
                short(&at),
                entry
                    .pr
                    .map_or(String::new(), |pr| format!(", and so does #{pr}"))
            ));
        }
        _ => {
            if let Some(pr) = entry.pr {
                if let Some(said) = settle(repo, env, pr, merge) {
                    detail.push_str(&format!(". {said}"));
                }
            }
            if remote.is_some() {
                let _ = best_effort(repo, &["push", "--quiet", &env.remote, "--delete", branch]);
            }
        }
    }

    let cleanup = if entry.worktree.is_empty() {
        Vec::new()
    } else {
        vec![
            "once `git status --porcelain` there prints nothing (agent-worktrees):".to_string(),
            format!(
                "git worktree remove --force {}",
                shell_quote(&entry.worktree)
            ),
            format!("git branch -D {}", shell_quote(branch)),
        ]
    };
    Stopped::of(
        OutcomeState::Landed,
        detail,
        OutcomePatch {
            merge_commit: Some(merge.to_string()),
            cleanup: Some(cleanup),
            ..OutcomePatch::default()
        },
    )
}

/// Wait for the forge to read the push as the pull request's merge, which it
/// does once the head is in the base, before the branch is deleted — deleting
/// it first would close the pull request unmerged. Past [`Env::pr_wait`] it
/// is closed with a comment naming the merge. `Some` says what was done.
fn settle(repo: &Path, env: &Env, pr: u64, merge: &str) -> Option<String> {
    let number = pr.to_string();
    let deadline = Instant::now() + env.pr_wait;
    loop {
        let state = gh_view(&env.gh, repo, &number, "state").and_then(|view| view.state);
        match state.as_deref() {
            Some("OPEN") if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Some("OPEN") => break,
            _ => return None,
        }
    }
    let comment = format!("Landed on {} as {merge} by the merge line.", env.base);
    let closed = run(
        &[&env.gh, "pr", "close", &number, "--comment", &comment],
        repo,
        None,
        None,
    );
    Some(match closed {
        Ok(ran) if ran.success() => format!(
            "#{pr} did not read as merged, so it was closed naming {}",
            short(merge)
        ),
        _ => format!("#{pr} did not read as merged and could not be closed; close it by hand"),
    })
}

/// Where a killed runner's push already landed the branch: the merge commit
/// its last turn recorded, if that is on the base, or the base itself.
pub fn already_landed(repo: &Path, recorded: Option<&str>, base: &str) -> String {
    recorded
        .filter(|merge| is_ancestor(repo, merge, base))
        .map(str::to_string)
        .unwrap_or_else(|| base.to_string())
}

/// The commit a local branch points at, or `None` where it is gone.
pub fn local_head(repo: &Path, branch: &str) -> Option<String> {
    rev_parse(repo, &format!("refs/heads/{branch}")).ok()
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}

/// `shlex.quote`, near enough: a token made only of the characters a shell
/// never treats specially is printed bare, and anything else is
/// single-quoted with its own single quotes escaped.
fn shell_quote(value: &str) -> String {
    let plain = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
    if plain {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
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
