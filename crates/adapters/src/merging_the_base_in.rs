//! Bringing a Job's branch up to the base the remote holds, in its own
//! worktree, so `merge_by: push` can gate it again before it lands. The merge
//! is `crate::merging_in`'s and the base is fetched as `crate::landing` fetches
//! it for the push.

use std::collections::BTreeSet;

use adapter_traits::{BaseMergedIn, NotDelivered, NotMerged, Worktree};

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
        was,
        touched: touched.into_iter().collect(),
    })
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
