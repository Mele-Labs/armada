//! A served checkout's uncommitted files, committed from objects, pushed, and
//! opened as a pull request with auto-merge on.
//!
//! **Nothing here reads or writes `in_repo`'s index or `HEAD`, and the working
//! tree is only read.** The commit's parent is `origin`'s tip of the base and
//! its tree is that tip's with each path replaced by what the working tree holds
//! (a file gone from it is removed), so it is the owner's change on top of
//! main, however far behind the checkout is. For `manifest_edit`'s reason the
//! commit is reachable from nothing until the push names it.

use std::path::Path;

use adapter_traits::{CheckoutSent, CheckoutToSend, Forge, NotDelivered};
use git2::build::TreeUpdateBuilder;
use git2::{Oid, Repository, Signature};

use crate::landing::{base_here, changed_between, same_as_the_tree, working, Working, REMOTE};
use crate::manifest_edit::pushed;

/// What a pull request opened from a checkout's files says beside its title.
const BODY: &str = "Sent from the uncommitted files of the served checkout.";

/// Commit, push, open the pull request and turn auto-merge on.
/// The pull request and its auto-merge go through `forge`.
pub(crate) fn send(
    in_repo: &str,
    sending: &CheckoutToSend,
    forge: &impl Forge,
) -> Result<CheckoutSent, NotDelivered> {
    let commit = committed(in_repo, sending)?;
    pushed(in_repo, &commit, &sending.branch)?;
    let pull_request =
        forge.open_branch_for_review(in_repo, &sending.base, &sending.branch, &sending.message, BODY)?;
    let auto_merge = forge.enable_auto_merge(in_repo, &pull_request);
    Ok(CheckoutSent {
        commit,
        pull_request,
        auto_merge,
    })
}

/// The commit, written with no ref to move. **Refuses a path the base also
/// changed to something else**, so a commit on the tip cannot undo what landed
/// there since the hold was read; a path that already equals the tip is left out.
pub(crate) fn committed(in_repo: &str, sending: &CheckoutToSend) -> Result<String, NotDelivered> {
    const DOING: &str = "committing the checkout's files";
    let git = |cause: git2::Error| NotDelivered::of(DOING, cause.message().to_string());
    let repo = Repository::open(in_repo).map_err(git)?;
    let tip = repo
        .refname_to_id(&format!("refs/remotes/{REMOTE}/{}", sending.base))
        .and_then(|id| repo.find_commit(id))
        .map_err(git)?;
    let tree = tip.tree().map_err(git)?;
    let ours = base_here(&repo, &sending.base)
        .ok_or_else(|| NotDelivered::of(DOING, format!("git could not find `{}` here", sending.base)))?;
    let changed_on_the_base = changed_between(&repo, ours, tip.id());

    let mut update = TreeUpdateBuilder::new();
    let mut changes = 0;
    for path in &sending.paths {
        if same_as_the_tree(&repo, &tree, path) {
            continue;
        }
        if changed_on_the_base.contains(path) {
            return Err(NotDelivered::of(
                DOING,
                format!("`{path}` was also changed on `{}`", sending.base),
            ));
        }
        match working(&repo, path) {
            Working::Gone => {
                update.remove(Path::new(path));
            }
            Working::File(content, mode) => {
                let blob = repo.blob(&content).map_err(git)?;
                update.upsert(Path::new(path), blob, mode);
            }
            Working::Unreadable => {
                return Err(NotDelivered::of(DOING, format!("`{path}` is not a file git can send")));
            }
        }
        changes += 1;
    }
    if changes == 0 {
        return Err(NotDelivered::of(
            DOING,
            format!("every file is what `{}` already holds", sending.base),
        ));
    }
    let tree = update
        .create_updated(&repo, &tree)
        .and_then(|id| repo.find_tree(id))
        .map_err(git)?;
    // The repository's own identity where it has one, as `manifest_edit` does.
    let who = repo
        .signature()
        .or_else(|_| Signature::now("armada", "armada@localhost"))
        .map_err(git)?;
    let commit: Oid = repo
        .commit(None, &who, &who, &sending.message, &tree, &[&tip])
        .map_err(git)?;
    Ok(commit.to_string())
}
