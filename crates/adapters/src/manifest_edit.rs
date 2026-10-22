//! A form's edit to `armada.yml`, committed from objects and pushed, so it
//! never lands in the checkout a person is standing in.
//!
//! **The commit is made without a checkout.** The tree of the commit it sits on
//! gets one blob replaced, and the commit is written with no ref to move. Nothing
//! here reads or writes `in_repo`'s working tree, index or `HEAD`, which is the
//! whole reason this is not a `git commit` in the served directory: Fleet only
//! fast-forwards a clean checkout, and an edit left uncommitted there blocked
//! every catch-up (14 Oct 2026).

use std::path::Path;

use adapter_traits::{
    Forge,
    BaseFetched, ManifestEditSent, ManifestEditToSend, ManifestOnTheBase, NotDelivered,
    PullRequestFor,
};
use git2::build::TreeUpdateBuilder;
use git2::{FileMode, Oid, Repository, Signature, Status};

use crate::delivery::{last_line, run_in, said, FORGE};
use crate::fetching_the_base;
use crate::landing::REMOTE;

/// What a pull request opened from a form's edit says beside its title.
const BODY: &str = "Saved from the Manifest form.";

/// Whether the file is tracked and not ignored in a repository with an `origin`.
/// **An ignored file is the case this exists to leave alone**: it is not in the
/// repository, so a pull request has nothing to change.
pub(crate) fn goes_by_pull_request(in_repo: &str, file: &str) -> bool {
    let Ok(repo) = Repository::open(in_repo) else {
        return false;
    };
    if repo.find_remote(REMOTE).is_err() {
        return false;
    }
    let Some(relative) = relative(&repo, file) else {
        return false;
    };
    match repo.status_file(Path::new(&relative)) {
        Ok(status) => !status.intersects(Status::IGNORED | Status::WT_NEW | Status::INDEX_NEW),
        Err(_) => false,
    }
}

/// The Manifest as `origin/<base>` holds it, after fetching it.
pub(crate) fn on_the_base(
    in_repo: &str,
    base: &str,
    file: &str,
) -> Result<ManifestOnTheBase, NotDelivered> {
    const DOING: &str = "reading the Manifest on the base";
    match fetching_the_base::fetched(in_repo, base) {
        BaseFetched::Fetched { .. } => {}
        BaseFetched::Local => {
            return Err(NotDelivered::of(
                DOING,
                format!("`{REMOTE}` has no branch `{base}`"),
            ))
        }
        BaseFetched::Failed { said, .. } => return Err(NotDelivered::of(DOING, said)),
    }
    let git = |cause: git2::Error| NotDelivered::of(DOING, cause.message().to_string());
    let repo = Repository::open(in_repo).map_err(git)?;
    let tip = repo
        .revparse_single(&format!("refs/remotes/{REMOTE}/{base}"))
        .and_then(|found| found.peel_to_commit())
        .map_err(git)?;
    let relative = relative(&repo, file).ok_or_else(|| outside(DOING, file))?;
    let entry = tip
        .tree()
        .and_then(|tree| tree.get_path(Path::new(&relative)))
        .map_err(git)?;
    let blob = repo.find_blob(entry.id()).map_err(git)?;
    let text = String::from_utf8(blob.content().to_vec())
        .map_err(|_| NotDelivered::of(DOING, format!("`{relative}` is not text")))?;
    Ok(ManifestOnTheBase {
        commit: tip.id().to_string(),
        text,
    })
}

/// Commit, push and, where it is a new branch, open the pull request.
/// The pull request and its auto-merge go through `forge`.
pub(crate) fn send(
    in_repo: &str,
    sending: &ManifestEditToSend,
    forge: &impl Forge,
) -> Result<ManifestEditSent, NotDelivered> {
    let commit = committed_and_pushed(in_repo, sending)?;
    match &sending.to {
        PullRequestFor::Update { pull_request, .. } => Ok(ManifestEditSent {
            commit,
            pull_request: pull_request.clone(),
            auto_merge: Ok(()),
        }),
        PullRequestFor::Open { base } => {
            // **A branch with no pull request on it is left on the remote by
            // nobody's choice**, so a pull request that will not open takes
            // the branch the push just made with it.
            let pull_request = forge
                .open_branch_for_review(in_repo, base, &sending.branch, &sending.message, BODY)
                .inspect_err(|_| unpushed(in_repo, &sending.branch))?;
            let auto_merge = forge.enable_auto_merge(in_repo, &pull_request);
            Ok(ManifestEditSent {
                commit,
                pull_request,
                auto_merge,
            })
        }
    }
}

/// Open a pull request from `branch` into `base`, and answer its address.
pub(crate) fn opened(
    in_repo: &str,
    base: &str,
    branch: &str,
    title: &str,
    body: &str,
) -> Result<String, NotDelivered> {
    let opened = run_in(
        in_repo,
        FORGE,
        &[
            "pr", "create", "--base", base, "--head", branch, "--title", title, "--body", body,
        ],
    )
    .map_err(|why| {
        NotDelivered::of("the pull request", format!("`{FORGE}` would not run: {why}"))
    })?;
    if !opened.status.success() {
        return Err(NotDelivered::of("the pull request", said(&opened)));
    }
    Ok(last_line(&opened))
}

/// Delete `branch` from the remote. **Best effort**: the refusal this follows is
/// what the caller reports, and a branch that will not go is no worse than it
/// was.
fn unpushed(in_repo: &str, branch: &str) {
    let _ = run_in(
        in_repo,
        "git",
        &["push", "--quiet", REMOTE, "--delete", branch],
    );
}

/// The commit, made on `parent`'s tree with the one file replaced, and pushed
/// to the branch. Answers the commit.
fn committed_and_pushed(
    in_repo: &str,
    sending: &ManifestEditToSend,
) -> Result<String, NotDelivered> {
    const DOING: &str = "committing the Manifest";
    let git = |cause: git2::Error| NotDelivered::of(DOING, cause.message().to_string());
    let repo = Repository::open(in_repo).map_err(git)?;
    let relative = relative(&repo, &sending.file).ok_or_else(|| outside(DOING, &sending.file))?;
    let parent = Oid::from_str(&sending.parent)
        .and_then(|id| repo.find_commit(id))
        .map_err(git)?;
    let tree = parent.tree().map_err(git)?;
    let mode = match tree.get_path(Path::new(&relative)).map_err(git)?.filemode() {
        0o100755 => FileMode::BlobExecutable,
        _ => FileMode::Blob,
    };
    let blob = repo.blob(sending.text.as_bytes()).map_err(git)?;
    let mut update = TreeUpdateBuilder::new();
    update.upsert(&relative, blob, mode);
    let tree = update
        .create_updated(&repo, &tree)
        .and_then(|id| repo.find_tree(id))
        .map_err(git)?;
    // The repository's own identity where it has one, so the commit reads as
    // the person's; a machine with none is `armada`.
    let who = repo
        .signature()
        .or_else(|_| Signature::now("armada", "armada@localhost"))
        .map_err(git)?;
    // **No ref to update**: the commit is reachable from nothing until the
    // push names it, and `HEAD` stays where it was.
    let commit = repo
        .commit(None, &who, &who, &sending.message, &tree, &[&parent])
        .map_err(git)?
        .to_string();
    pushed(in_repo, &commit, &sending.branch)?;
    Ok(commit)
}

/// Push `commit` to a branch of that name on the remote. Never forced.
pub(crate) fn pushed(in_repo: &str, commit: &str, branch: &str) -> Result<(), NotDelivered> {
    let pushed = run_in(
        in_repo,
        "git",
        &["push", "--quiet", REMOTE, &format!("{commit}:refs/heads/{branch}")],
    )
    .map_err(|why| NotDelivered::of("the push", format!("`git` would not run: {why}")))?;
    match pushed.status.success() {
        true => Ok(()),
        false => Err(NotDelivered::of("the push", said(&pushed))),
    }
}

/// `file` as a path from the repository's root, which is how a tree names it.
/// A Manifest in a workspace is a path with a directory in it.
fn relative(repo: &Repository, file: &str) -> Option<String> {
    let root = repo.workdir()?.canonicalize().ok()?;
    let file = Path::new(file).canonicalize().ok()?;
    Some(file.strip_prefix(&root).ok()?.to_string_lossy().into_owned())
}

fn outside(doing: &'static str, file: &str) -> NotDelivered {
    NotDelivered::of(doing, format!("`{file}` is not inside the repository"))
}
