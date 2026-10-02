//! A turn's batch: up to [`super::env::Env::batch`] waiting branches, merged
//! one after another onto the base into one candidate, each as its own
//! `--no-ff` merge commit, so history still reads per branch.
//! `docs/capabilities/merge-line.md`, *Batching*.

use std::path::{Path, PathBuf};

use super::dir::StateDir;
use super::env::Env;
use super::git::best_effort;
use super::merge_in::{merge_in, regenerate, MergeInFailed};
use super::onto_main::{merge_commit, message};
use super::outcome::{read_outcome, OutcomePatch, OutcomeState};
use super::prepare::{nothing_left, seed};
use super::queue::QueueEntry;
use super::repo::{changed_paths, is_ancestor, merge_base, rev_parse};
use super::say::say;
use super::stop::Stopped;
use super::worktree::{reused_keeping, LandWorktree};

/// Say the same thing to every member, naming the others it gates with, so
/// `--status` shows which entries are gating together.
pub fn tell(
    state: &StateDir,
    group: &[QueueEntry],
    status: OutcomeState,
    detail: impl Into<String>,
    patch: OutcomePatch,
) -> Result<(), Stopped> {
    let detail = detail.into();
    for entry in group {
        let others: Vec<&str> = group
            .iter()
            .filter(|other| other.branch != entry.branch)
            .map(|other| other.branch.as_str())
            .collect();
        let said = if others.is_empty() {
            detail.clone()
        } else {
            format!("{detail} — together with {}", others.join(", "))
        };
        say(state, &entry.branch, status, said, patch.clone())?;
    }
    Ok(())
}

/// A batch split in two, first half first: a red or a clash between members
/// is retried on each half, down to a single entry.
pub fn halves<T>(mut group: Vec<T>) -> (Vec<T>, Vec<T>) {
    let second = group.split_off(group.len().div_ceil(2));
    (group, second)
}

/// The candidate a group built: one merge commit per member, in order, then
/// whatever regeneration committed on top.
pub struct Built {
    pub worktree: PathBuf,
    pub merges: Vec<String>,
    pub top: String,
    pub regenerated: bool,
    /// Whether anything was merged in beyond a lone branch already on the base.
    pub moved: bool,
    /// Every path the Checks are asked about: each branch's side, and what
    /// landed on the base since it was cut.
    pub hit: Vec<String>,
    /// Each member's own side alone, in member order.
    pub own: Vec<Vec<String>>,
}

pub enum NotBuilt {
    /// The member at this index does not merge with the base itself: it goes
    /// back with the conflict, and the rest go on without it.
    Conflict(usize, Stopped),
    /// Two members do not merge with each other.
    Between,
    Stopped(Stopped),
}

impl From<Stopped> for NotBuilt {
    fn from(stopped: Stopped) -> NotBuilt {
        NotBuilt::Stopped(stopped)
    }
}

impl From<super::git::GitFailed> for NotBuilt {
    fn from(cause: super::git::GitFailed) -> NotBuilt {
        NotBuilt::Stopped(cause.into())
    }
}

/// Merge each member onto the stack in turn — the stack starting at `base` —
/// then seed and regenerate the top.
pub fn build(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    group: &[QueueEntry],
    base: &str,
    logs: &Path,
) -> Result<Built, NotBuilt> {
    let keep = env.keep_refs();
    let reuse = |at: &str| {
        reused_keeping(repo, LandWorktree::Candidate, at, logs, &keep)
            .map_err(|why| Stopped::stopped(why.to_string()))
    };
    let mut stack = base.to_string();
    let mut merges = Vec::new();
    let mut hit = Vec::new();
    let mut own = Vec::new();
    let mut moved = group.len() > 1;
    for (index, entry) in group.iter().enumerate() {
        let (branch, head) = (entry.branch.as_str(), entry.head.as_str());
        let since = merge_base(repo, head, base)?;
        let mine = changed_paths(repo, &since, head)?;
        hit.extend(mine.iter().cloned());
        own.push(mine);
        if !is_ancestor(repo, base, head) {
            moved = true;
            hit.extend(changed_paths(repo, &since, base)?);
        }
        let where_ = reuse(head)?;
        if !is_ancestor(repo, &stack, head) {
            tell(
                state,
                group,
                OutcomeState::Gating,
                format!("merging {} ({}) into {branch}", env.base, short(&stack)),
                OutcomePatch::default(),
            )?;
            if let Err(failed) = merge_in(&where_, branch, &stack, logs) {
                let _ = best_effort(&where_, &["merge", "--abort"]);
                let files = match failed {
                    MergeInFailed::Conflict { files } => files,
                    MergeInFailed::Stopped(detail) => return Err(Stopped::stopped(detail).into()),
                };
                if index > 0 && !conflicts_with(repo, env, entry, base, logs)? {
                    return Err(NotBuilt::Between);
                }
                return Err(NotBuilt::Conflict(
                    index,
                    conflict(state, env, entry, base, files),
                ));
            }
            nothing_left(&where_, "the merge")?;
        }
        let candidate = rev_parse(&where_, "HEAD")?;
        stack = merge_commit(repo, &stack, &candidate, &message(branch, entry.pr))?;
        merges.push(stack.clone());
    }

    let where_ = reuse(&stack)?;
    seed(repo, &where_, env, logs)?;
    let regenerated = regenerate(&where_, &env.regenerate, logs).map_err(|why| {
        Stopped::red(
            format!("red: {why}. Nothing was pushed or merged."),
            OutcomePatch {
                logs: Some(vec![logs.join("regenerate.log").display().to_string()]),
                ..OutcomePatch::default()
            },
        )
    })?;
    let top = rev_parse(&where_, "HEAD")?;
    hit.extend(changed_paths(repo, base, &top)?);
    hit.sort();
    hit.dedup();
    Ok(Built {
        worktree: where_,
        merges,
        top,
        regenerated,
        moved,
        hit,
        own,
    })
}

/// Whether `entry` conflicts with `base` alone, as opposed to with a member
/// merged before it.
fn conflicts_with(
    repo: &Path,
    env: &Env,
    entry: &QueueEntry,
    base: &str,
    logs: &Path,
) -> Result<bool, NotBuilt> {
    if is_ancestor(repo, base, &entry.head) {
        return Ok(false);
    }
    let keep = env.keep_refs();
    let where_ = reused_keeping(repo, LandWorktree::Candidate, &entry.head, logs, &keep)
        .map_err(|why| Stopped::stopped(why.to_string()))?;
    match merge_in(&where_, &entry.branch, base, logs) {
        Ok(()) => Ok(false),
        Err(MergeInFailed::Conflict { .. }) => {
            let _ = best_effort(&where_, &["merge", "--abort"]);
            Ok(true)
        }
        Err(MergeInFailed::Stopped(detail)) => Err(Stopped::stopped(detail).into()),
    }
}

fn conflict(
    state: &StateDir,
    env: &Env,
    entry: &QueueEntry,
    base: &str,
    files: Vec<String>,
) -> Stopped {
    let branch = entry.branch.as_str();
    let place = read_outcome(state, branch)
        .ok()
        .flatten()
        .and_then(|outcome| outcome.place)
        .or(Some(entry.place));
    Stopped::conflict(
        format!(
            "{base} does not merge into {branch} cleanly. Merge {}/{} in, \
             commit, preflight and land again — it keeps its place in line.",
            env.remote, env.base
        ),
        OutcomePatch {
            conflicts: Some(if files.is_empty() {
                vec!["the merge failed without naming a file; see merge-in.log".to_string()]
            } else {
                files
            }),
            place,
            ..OutcomePatch::default()
        },
    )
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}

#[cfg(test)]
mod tests {
    use super::halves;

    #[test]
    fn a_batch_splits_in_place_order_down_to_one() {
        assert_eq!(halves(vec![1, 2, 3, 4]), (vec![1, 2], vec![3, 4]));
        assert_eq!(halves(vec![1, 2, 3]), (vec![1, 2], vec![3]));
        assert_eq!(halves(vec![1, 2]), (vec![1], vec![2]));
    }
}
