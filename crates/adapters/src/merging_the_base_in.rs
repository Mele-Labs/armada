//! Bringing a Job's branch up to the base the remote holds, in its own
//! worktree, so `merge_by: push` can gate it again before it lands. The merge
//! is `crate::merging_in`'s and the base is fetched as `crate::landing` fetches
//! it for the push. A head something else moved is read here too, for its
//! Checks to run on before it lands.

use std::collections::BTreeSet;

use adapter_traits::{BaseMergedIn, NotDelivered, NotMerged, UncheckedHead, Worktree};

use crate::delivery::{git, said};
use crate::landing::{fetched_base, RemoteBase};
use crate::merging_in::{merge_in_progress, merged_in, rev_parsed, MergedIn, OnConflict};

/// See [`adapter_traits::Delivery::merge_the_moved_base_in`].
pub(crate) fn merge_the_moved_base_in(
    in_repo: &str,
    worktree: &Worktree,
    declared: Option<&str>,
) -> Result<BaseMergedIn, NotMerged> {
    let refused = |said: String| NotMerged::Refused { said };
    let RemoteBase {
        base,
        tracking,
        gated,
    } = fetched_base(in_repo, declared)?;
    only_what_is_committed(worktree)?;
    let was = rev_parsed(worktree, "HEAD")
        .ok_or_else(|| refused(String::from("the Job's branch points at no commit")))?;
    match merged_in(worktree, &tracking, OnConflict::PutItBack)
        .map_err(|why| refused(why.said()))?
    {
        MergedIn::Clean { .. } => {}
        MergedIn::Conflicted { files } | MergedIn::PutBack { files } => {
            return Err(NotMerged::Conflicted {
                said: format!(
                    "{base} as the remote holds it conflicts with the branch in {}, and the \
                     branch is as it was",
                    files.join(", ")
                ),
            })
        }
    }
    let head = rev_parsed(worktree, "HEAD")
        .ok_or_else(|| refused(String::from("the Job's branch points at no commit")))?;
    let tree = tree_of(worktree, &head)?;
    // Either side, `armada land`'s rule: what the base brought, and what the
    // branch carries over it.
    let mut touched = BTreeSet::new();
    for from in [&was, &gated] {
        touched.extend(changed(worktree, from, &head).map_err(|why| refused(why.said()))?);
    }
    Ok(BaseMergedIn {
        base,
        onto: gated,
        head,
        tree,
        was,
        touched: touched.into_iter().collect(),
    })
}

/// See [`adapter_traits::Delivery::the_unchecked_head`].
pub(crate) fn the_unchecked_head(
    in_repo: &str,
    worktree: &Worktree,
    declared: Option<&str>,
    checked: Option<&str>,
) -> Result<UncheckedHead, NotMerged> {
    let refused = |said: String| NotMerged::Refused { said };
    let RemoteBase { gated, .. } = fetched_base(in_repo, declared)?;
    only_what_is_committed(worktree)?;
    let head = rev_parsed(worktree, "HEAD")
        .ok_or_else(|| refused(String::from("the Job's branch points at no commit")))?;
    let tree = tree_of(worktree, &head)?;
    let mut touched = BTreeSet::new();
    let since_checked = match checked {
        Some(checked) => changed(worktree, checked, &head),
        None => every_path(worktree, &head),
    };
    touched.extend(since_checked.map_err(|why| refused(why.said()))?);
    touched.extend(changed(worktree, &gated, &head).map_err(|why| refused(why.said()))?);
    Ok(UncheckedHead {
        head,
        tree,
        touched: touched.into_iter().collect(),
    })
}

/// Refuse a worktree the Checks would read differently from what lands.
fn only_what_is_committed(worktree: &Worktree) -> Result<(), NotMerged> {
    let refused = |said: String| NotMerged::Refused { said };
    if merge_in_progress(worktree) {
        return Err(NotMerged::Conflicted {
            said: String::from("a merge is part-way through in the Job's worktree"),
        });
    }
    // Untracked files are a build's leftovers far more often than work, and
    // they do not land either way; a tracked change would be read and not land.
    let status = git(worktree, &["status", "--porcelain", "--untracked-files=no"])
        .map_err(|why| refused(why.said()))?;
    if !status.status.success() {
        return Err(refused(said(&status)));
    }
    if !status.stdout.is_empty() {
        return Err(refused(String::from(
            "the Job's worktree holds changes nothing committed, so its Checks would read a \
             tree that would not land",
        )));
    }
    Ok(())
}

fn tree_of(worktree: &Worktree, commit: &str) -> Result<String, NotMerged> {
    rev_parsed(worktree, &format!("{commit}^{{tree}}")).ok_or_else(|| NotMerged::Refused {
        said: format!("{commit} has no tree"),
    })
}

/// Every path `commit` holds: what changed since a run of Checks that never
/// happened.
fn every_path(worktree: &Worktree, commit: &str) -> Result<Vec<String>, NotDelivered> {
    let run = git(worktree, &["ls-tree", "-r", "--name-only", commit])?;
    if !run.status.success() {
        return Err(NotDelivered::of(
            "reading what the branch holds",
            said(&run),
        ));
    }
    Ok(String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// See [`adapter_traits::Delivery::settle_worktree`].
pub(crate) fn settle_worktree(worktree: &Worktree) -> Result<(), NotDelivered> {
    if !merge_in_progress(worktree) {
        return Ok(());
    }
    // `--abort` refuses where it cannot rebuild the tree it began with, and a
    // killed merge may leave exactly that; the committed branch is what counts.
    let aborted = git(worktree, &["merge", "--abort"])?;
    if aborted.status.success() {
        return Ok(());
    }
    let reset = git(worktree, &["reset", "--quiet", "--hard", "HEAD"])?;
    match reset.status.success() {
        true => Ok(()),
        false => Err(NotDelivered::of(
            "clearing a merge left part-way through",
            said(&reset),
        )),
    }
}

/// See [`adapter_traits::Delivery::put_back`].
pub(crate) fn put_back(worktree: &Worktree, merged: &BaseMergedIn) -> Result<(), NotDelivered> {
    if rev_parsed(worktree, "HEAD").as_deref() != Some(merged.head.as_str()) {
        return Err(NotDelivered::of(
            "putting the branch back",
            format!(
                "the branch has moved past the merge of {}, so it is left where it is",
                merged.base
            ),
        ));
    }
    let run = git(worktree, &["reset", "--quiet", "--hard", &merged.was])?;
    match run.status.success() {
        true => Ok(()),
        false => Err(NotDelivered::of("putting the branch back", said(&run))),
    }
}

fn changed(worktree: &Worktree, from: &str, to: &str) -> Result<Vec<String>, NotDelivered> {
    let run = git(worktree, &["diff", "--name-only", "--no-renames", from, to])?;
    if !run.status.success() {
        return Err(NotDelivered::of(
            "reading what the merge changed",
            said(&run),
        ));
    }
    Ok(String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}
