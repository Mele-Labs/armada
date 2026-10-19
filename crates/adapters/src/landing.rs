//! What became of a pull request after Armada opened it, and what the
//! repository it merges into should do about it.
//!
//! # A different moment from `crate::delivery`, which is why it is a different
//! file
//!
//! That module runs inside a Job's last turn, in the Job's own worktree, with
//! the work in hand. Everything here runs minutes or days later, from the
//! repository every worktree was cut from, about a pull request nobody on this
//! machine is holding. `#337` asked whether anybody merged it and `#427`
//! whether the forge is still comparing it against the right commit; both are
//! one `gh pr view`, because building them apart would be two processes where
//! one does.
//!
//! # Reading is free and writing is not
//!
//! [`read`] and [`caught_up`] are cheap and repeatable. `crate::keeping_current`
//! rebases and pushes a person's pull request where its base is provably
//! superseded, and never on a reading this could not make. [`merge`] is louder
//! still and is the one act here nothing decides: a person
//! presses for it, and a refusal is classified from `mergeStateStatus` rather
//! than from the prose `gh` printed. A sentence [`why_not`] has no word for is
//! [`NotMerged::Refused`] carrying it verbatim, never a guess — the kinds exist
//! to send a person somewhere, and the wrong place is worse than the sentence.

use std::path::Path;

use adapter_traits::{
    CheckoutPulled, HeldBack, Landable, Landing, Mergeable, Merged, NotMerged, PushedOntoBase, Rendering, RepositoryStanding,
    WhatBecameOfIt, WorktreeSpec,
};
use git2::Repository;

use crate::onto_base::{self, NotOntoBase, Onto};

use crate::delivery::{asked, last_line, read_in, run_in, said, FORGE};

/// Ask the forge what became of one pull request, what it is showing beside
/// it, and what a person would read about it before either of those answers
/// mattered.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
    // **Seven fields in one call**, `#427`'s argument carried one field
    // further: the number, the title and whether the forge can merge it are
    // three more scalars on the one `pr view` this already runs, and a second
    // call for any of them would be a process spent on an answer this one
    // already has. The address is not among them — it is the argument, so
    // asking for it back would be asking the forge to confirm what was just
    // handed to it.
    //
    // `mergedAt` is null until it merges, and a blank field refuses the whole
    // line, so it crosses as `-` until then.
    let Some(said) = asked(
        in_repo,
        pull_request,
        "number,title,state,mergeable,baseRefName,baseRefOid,headRefOid,mergedAt",
        "[.number, .title, .state, .mergeable, .baseRefName, .baseRefOid, .headRefOid, \
         (.mergedAt // \"-\")] | @tsv",
    ) else {
        return WhatBecameOfIt::unknown();
    };
    let Some([number, title, state, mergeable, base, pinned, head, merged]) = fields::<8>(&said)
    else {
        return WhatBecameOfIt::unknown();
    };
    let url = pull_request.to_string();
    let landing = match state {
        "MERGED" => Landing::Merged { url },
        "CLOSED" => Landing::ClosedUnmerged { url },
        "OPEN" => Landing::Open {
            url,
            rendering: rendering(in_repo, base, pinned, head),
        },
        // A word this forge has and Armada has not. Saying nothing is the
        // honest answer, and it costs one more ask later.
        _ => return WhatBecameOfIt::unknown(),
    };
    WhatBecameOfIt {
        landing,
        base: Some(base.to_string()),
        // A number the forge printed that this cannot parse is a forge
        // sending back something other than a number, which this reads the
        // same as not having asked — never a guessed id.
        number: number.parse().ok(),
        title: Some(title.to_string()),
        mergeable: match mergeable {
            "MERGEABLE" => Mergeable::Yes,
            "CONFLICTING" => Mergeable::No,
            // `UNKNOWN` is the forge still computing it, and a word this
            // build has no other name for is the same silence.
            _ => Mergeable::Unreadable,
        },
        merged_at: merged_at(state, merged),
    }
}

/// When a pull request merged, off the forge's own field: only a merged one
/// has an instant, and `-` is the null [`read`] asked to be spelled.
pub(crate) fn merged_at(state: &str, said: &str) -> Option<String> {
    (state == "MERGED" && said != "-").then(|| said.to_string())
}

/// Merge a pull request Armada opened.
///
/// **Read, then write, and the read is what makes the refusal nameable.** `gh
/// pr merge` prints prose; `gh pr view` answers with the forge's own words for
/// the two states and one flag that decide most of it. So the state is asked
/// first — a pull request somebody already merged is not a failed press, and a
/// closed one is a different sentence from a conflicted one.
///
/// **`--merge`, and no other strategy.** A squash or a rebase rewrites what the
/// Job's Checks passed against, and `crate::proving` runs the repository's
/// after-merge Checks against the commit the base is left on: proving a tree
/// nobody produced is worse than not proving one.
pub(crate) fn merge(in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
    let Some(standing) = asked(
        in_repo,
        pull_request,
        "state,mergeable,mergeStateStatus",
        "[.state, .mergeable, .mergeStateStatus] | @tsv",
    ) else {
        return Err(NotMerged::NoTool {
            said: format!("`{FORGE} pr view {pull_request}` would not answer"),
        });
    };
    let Some([state, mergeable, status]) = fields::<3>(&standing) else {
        return Err(NotMerged::Refused {
            said: format!("`{FORGE}` answered `{standing}`, which has no reading here"),
        });
    };
    match state {
        // **Not a refusal.** The work is where the press was trying to put it.
        "MERGED" => return Ok(Merged::AlreadyMerged),
        "OPEN" => {}
        other => {
            return Err(NotMerged::NotOpen {
                said: format!("the forge says it is {other}"),
            })
        }
    }
    // Asked before the write, because a conflict is the one refusal the forge
    // will not phrase as one: `gh pr merge` on a conflicting branch fails with
    // a sentence about the merge method.
    if mergeable == "CONFLICTING" {
        return Err(NotMerged::Conflicted {
            said: String::from("the forge cannot merge this branch into its base as it stands"),
        });
    }
    let run = match run_in(in_repo, FORGE, &["pr", "merge", pull_request, "--merge"]) {
        Ok(run) => run,
        Err(why) => {
            return Err(NotMerged::NoTool {
                said: format!("`{FORGE}` would not run: {why}"),
            })
        }
    };
    match run.status.success() {
        true => Ok(Merged::Taken),
        false => Err(why_not(status, said(&run))),
    }
}

/// Ask the forge to merge a pull request itself once its checks pass. `--merge`,
/// for [`merge`]'s reason. The forge's own words come back on a refusal.
pub(crate) fn enable_auto_merge(in_repo: &str, pull_request: &str) -> Result<(), String> {
    let run = run_in(
        in_repo,
        FORGE,
        &["pr", "merge", pull_request, "--auto", "--merge"],
    )
    .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(said(&run)),
    }
}

/// Take back a request for the forge to merge a pull request itself, so it merges when a person
/// presses it and not before. The forge's own words come back on a refusal, as
/// [`enable_auto_merge`]'s do.
pub(crate) fn disable_auto_merge(in_repo: &str, pull_request: &str) -> Result<(), String> {
    let run = run_in(in_repo, FORGE, &["pr", "merge", pull_request, "--disable-auto"])
        .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(said(&run)),
    }
}

/// The remote a `merge_by: push` lands on, and the one `crate::base` reads a
/// default branch from.
pub(crate) const REMOTE: &str = "origin";

/// Land a Job's branch with the merge commit made here and the base pushed,
/// never forced. `merge_by: push`, through [`onto_base`].
///
/// **The base is fetched, not read locally**, unlike every catch-up in
/// `crate::delivery`: the push is checked against the remote's base, so the
/// merge has to be made on that one.
pub(crate) fn merge_by_push(
    in_repo: &str,
    handle: &str,
    declared: Option<&str>,
    pull_request: Option<u64>,
    landable: Landable<'_>,
) -> Result<PushedOntoBase, NotMerged> {
    let refused = |said: String| NotMerged::Refused { said };
    let branch = WorktreeSpec::for_job(in_repo, handle)
        .map_err(|why| refused(why.said()))?
        .branch();
    let root = Path::new(in_repo);
    let candidate = rev_parse(in_repo, &format!("refs/heads/{branch}"))
        .ok_or_else(|| refused(format!("the branch {branch} is gone")))?;
    let RemoteBase { base, gated, .. } = fetched_base(in_repo, declared)?;
    if onto_base::is_ancestor(root, &candidate, &gated) {
        return Ok(PushedOntoBase {
            merge: onto_base::merge_naming(root, &gated, &candidate),
            base,
            merged: Merged::AlreadyMerged,
        });
    }
    // The pull request reads its head from the remote branch, so a commit
    // there that this branch lacks would be left behind with it open.
    if let Some(at) = onto_base::remote_head(root, REMOTE, &branch) {
        if !onto_base::is_ancestor(root, &at, &candidate) {
            return Err(refused(format!(
                "{REMOTE}/{branch} is at {at}, which the branch here does not hold, so \
                 landing it would leave that commit behind"
            )));
        }
    }
    let message = onto_base::message(&branch, pull_request);
    let moved = |why: NotOntoBase| match why {
        NotOntoBase::DoesNotHoldBase { .. } => NotMerged::BaseMoved {
            said: why.to_string(),
        },
        NotOntoBase::Git(said) => NotMerged::Refused { said },
    };
    let merge = onto_base::merge_commit(root, &gated, &candidate, &message).map_err(moved)?;
    // After the merge commit, which refuses a branch short of the base first:
    // bringing it up answers both, and gates it once.
    let tree = rev_parse(in_repo, &format!("{candidate}^{{tree}}"))
        .ok_or_else(|| refused(format!("the branch {branch} has no tree")))?;
    let read = match landable {
        Landable::Checked(checked) => checked == tree,
        Landable::Unchecked => false,
        Landable::NothingToCheck => true,
    };
    if !read {
        return Err(NotMerged::Unchecked {
            said: format!("{branch} is at {candidate}, whose tree no run of its Checks passed on"),
        });
    }
    match onto_base::push(root, REMOTE, &base, &merge, &gated, |_, _| {}).map_err(moved)? {
        Onto::Landed => Ok(PushedOntoBase {
            base,
            merged: Merged::Taken,
            merge: Some(merge),
        }),
        Onto::Moved => Err(NotMerged::BaseMoved {
            said: format!("{base} moved on {REMOTE} between the fetch and the push"),
        }),
    }
}

/// The base a `merge_by: push` lands on, as the remote holds it this moment.
pub(crate) struct RemoteBase {
    pub(crate) base: String,
    /// The ref the fetch wrote, which a merge names.
    pub(crate) tracking: String,
    /// The commit it is on.
    pub(crate) gated: String,
}

/// Resolve the base and fetch the remote's copy of it: the push is checked
/// against that one, so every merge toward it is made on it.
pub(crate) fn fetched_base(in_repo: &str, declared: Option<&str>) -> Result<RemoteBase, NotMerged> {
    let refused = |said: String| NotMerged::Refused { said };
    let repo = Repository::open(in_repo).map_err(|cause| refused(cause.message().to_string()))?;
    let base = match crate::base::resolve(&repo, declared) {
        Ok(Some(base)) => base.name().to_string(),
        Ok(None) => {
            return Err(refused(String::from(
                "this repository names no base branch",
            )))
        }
        Err(why) => return Err(refused(why.said())),
    };
    let tracking = format!("refs/remotes/{REMOTE}/{base}");
    let fetched = run_in(
        in_repo,
        "git",
        &[
            "fetch",
            "--quiet",
            REMOTE,
            &format!("+refs/heads/{base}:{tracking}"),
        ],
    )
    .map_err(|why| NotMerged::NoTool {
        said: format!("`git` would not run: {why}"),
    })?;
    if !fetched.status.success() {
        return Err(refused(said(&fetched)));
    }
    let gated = rev_parse(in_repo, &tracking)
        .ok_or_else(|| refused(format!("{REMOTE} has no branch {base}")))?;
    Ok(RemoteBase {
        base,
        tracking,
        gated,
    })
}

fn rev_parse(in_repo: &str, rev: &str) -> Option<String> {
    let run = run_in(in_repo, "git", &["rev-parse", "--verify", "--quiet", rev]).ok()?;
    run.status.success().then(|| last_line(&run))
}

/// Which kind of refusal a failed merge was, from the forge's own state word.
///
/// **Three words have a reading and everything else is the sentence.** The
/// forge computes `mergeStateStatus` and names it, so a caller acting on one of
/// these three is acting on what the forge decided rather than on how `gh`
/// worded it that release.
fn why_not(status: &str, printed: String) -> NotMerged {
    match status {
        "DIRTY" => NotMerged::Conflicted { said: printed },
        "BLOCKED" => NotMerged::Protected { said: printed },
        "UNSTABLE" => NotMerged::ChecksNotPassed { said: printed },
        _ => NotMerged::Refused { said: printed },
    }
}

/// Bring the repository every worktree was cut from up to what just merged.
pub(crate) fn caught_up(in_repo: &str, base: &str) -> RepositoryStanding {
    let left_alone = |why: String| RepositoryStanding::LeftAlone { why };
    let repo = match Repository::open(in_repo) {
        Ok(repo) => repo,
        Err(cause) => return left_alone(cause.message().to_string()),
    };
    if let Some(why) = why_not_to_touch_it(&repo, base) {
        return left_alone(why);
    }
    let before = repo.head().ok().and_then(|head| head.target());
    // `--ff-only`, so a repository that has diverged is refused by git
    // rather than merged by Armada. `pull` and not `fetch` then `merge`:
    // one process, and the refusal comes back as the sentence git printed.
    let run = match run_in(in_repo, "git", &["pull", "--ff-only"]) {
        Ok(run) => run,
        Err(why) => return left_alone(why.to_string()),
    };
    if !run.status.success() {
        return left_alone(said(&run));
    }
    // **Read after the pull and reported, not inferred.** What `#474` runs its
    // Checks against is the tree that is here now, so the commit is the one
    // this repository's HEAD resolves to — never the merge commit the forge
    // named, which may be three commits back by the time anybody looks.
    let Some(after) = repo.head().ok().and_then(|head| head.target()) else {
        // A pull that succeeded over a HEAD nothing can read. Unreachable in
        // practice and reported as a refusal rather than as a standing with no
        // commit, because the alternative is a variant carrying an optional
        // one — and every caller would then have to ask a question that has no
        // answer it could act on.
        return left_alone(format!("git would not say what commit `{base}` is on"));
    };
    let moved = match before {
        Some(before) if before != after => repo
            .graph_ahead_behind(after, before)
            .map(|(ahead, _)| ahead)
            .unwrap_or(0),
        _ => 0,
    };
    match moved {
        0 => RepositoryStanding::AlreadyHadIt {
            base: base.to_string(),
            head: after.to_string(),
        },
        commits => RepositoryStanding::MovedOn {
            base: base.to_string(),
            commits,
            head: after.to_string(),
        },
    }
}

/// Why this repository is not Armada's to move, or `None` where it is.
///
/// **Three refusals and every one of them is a person's, not a fault.** A
/// checkout on some other branch is somebody mid-thought; uncommitted work is
/// somebody's unfinished change, and a fast-forward over it is exactly the
/// thing `bring_up_to_date` uses `--autostash` to avoid doing without asking.
fn why_not_to_touch_it(repo: &Repository, base: &str) -> Option<String> {
    let head = repo.head().ok()?;
    if !head.is_branch() {
        return Some(String::from("the repository is not on a branch"));
    }
    let on = head.shorthand().unwrap_or_default().to_string();
    if on != base {
        return Some(format!("the repository is on `{on}` and not `{base}`"));
    }
    let mut asking = git2::StatusOptions::new();
    asking.include_untracked(false).include_ignored(false);
    match repo.statuses(Some(&mut asking)) {
        Ok(statuses) if statuses.is_empty() => None,
        Ok(statuses) => Some(format!(
            "`{on}` is carrying {} uncommitted change(s)",
            statuses.len()
        )),
        Err(cause) => Some(cause.message().to_string()),
    }
}

/// Whether `base` here already has `commit`: the checked-out commit where the
/// base is checked out, the local base branch where it is not. In process, so
/// the reading of main can ask it every minute for nothing.
pub(crate) fn checkout_has(in_repo: &str, base: &str, commit: &str) -> bool {
    let Ok(repo) = Repository::open(in_repo) else {
        return false;
    };
    let (Some(ours), Ok(commit)) = (base_here(&repo, base), git2::Oid::from_str(commit)) else {
        return false;
    };
    ours == commit || repo.graph_descendant_of(ours, commit).unwrap_or(false)
}

/// The commit that stands for `base` in this checkout: `HEAD` where the base is
/// checked out, the local base branch where another is.
pub(crate) fn base_here(repo: &Repository, base: &str) -> Option<git2::Oid> {
    let head = repo.head().ok()?;
    match head.is_branch() && head.shorthand() == Some(base) {
        true => head.target(),
        false => repo.refname_to_id(&format!("refs/heads/{base}")).ok(),
    }
}

/// Fast-forward the local base branch while another is checked out. A refspec
/// without `+`, so git itself refuses a history that is not a fast-forward,
/// and a branch checked out in any worktree.
pub(crate) fn base_fast_forwarded(in_repo: &str, base: &str) -> bool {
    let refspec = format!("{base}:{base}");
    run_in(in_repo, "git", &["fetch", "--quiet", "origin", &refspec])
        .is_ok_and(|run| run.status.success())
}

/// What stops `in_repo` fast-forwarding to `base` on `origin`. One fetch of the
/// base, which writes only the remote-tracking ref, then read in process.
pub(crate) fn held_back(in_repo: &str, base: &str) -> Option<HeldBack> {
    let fetched = run_in(in_repo, "git", &["fetch", "--quiet", "origin", base]).ok()?;
    if !fetched.status.success() {
        return None;
    }
    let repo = Repository::open(in_repo).ok()?;
    let head = repo.head().ok()?;
    let on = match head.is_branch() {
        true => head.shorthand().map(str::to_string).filter(|on| on != base),
        false => Some(String::from("HEAD")),
    };
    let ours = base_here(&repo, base)?;
    let theirs = repo
        .refname_to_id(&format!("refs/remotes/origin/{base}"))
        .ok()?;
    let (ahead, behind) = repo.graph_ahead_behind(ours, theirs).ok()?;
    // Another branch's uncommitted files are not on the base, and hold nothing back.
    let mut asking = git2::StatusOptions::new();
    asking.include_untracked(false).include_ignored(false);
    let uncommitted: Vec<String> = match on {
        Some(_) => Vec::new(),
        None => repo
            .statuses(Some(&mut asking))
            .ok()?
            .iter()
            .filter_map(|entry| entry.path().map(str::to_string))
            .collect(),
    };
    // **A file main changed that already holds the owner's change is not a clash**: the change
    // went up another road, and the pull over it takes nothing from anyone.
    let (clashing, already_on_base): (Vec<String>, Vec<String>) = match uncommitted.is_empty() {
        true => (Vec::new(), Vec::new()),
        false => {
            let tree = |oid| repo.find_commit(oid).and_then(|commit| commit.tree()).ok();
            let (here, tip) = (tree(ours)?, tree(theirs)?);
            changed_between(&repo, ours, theirs)
                .into_iter()
                .filter(|path| uncommitted.contains(path))
                .partition(|path| {
                    !same_as_the_tree(&repo, &tip, path) && !held_by_the_tree(&repo, &here, &tip, path)
                })
        }
    };
    Some(HeldBack {
        base: base.to_string(),
        on,
        uncommitted,
        clashing,
        already_on_base,
        behind,
        ahead,
    })
}

/// `git pull --ff-only --autostash` in `in_repo`: the command a held checkout
/// offers its owner, run for them.
///
/// **A stash that would not go back on is not a success**, though git exits
/// clean for it: the pull moved the branch, the owner's files are in conflict
/// and their work is in the stash git left. That is a `1`, with git's own words,
/// and nothing here drops or pops the stash.
pub(crate) fn pulled_the_checkout(in_repo: &str) -> CheckoutPulled {
    let run = match run_in(in_repo, "git", &["pull", "--ff-only", "--autostash"]) {
        Ok(run) => run,
        Err(why) => {
            return CheckoutPulled {
                code: -1,
                output: why.to_string(),
            }
        }
    };
    let output = said(&run);
    let code = run.status.code().unwrap_or(-1);
    let conflicted = Repository::open(in_repo)
        .and_then(|repo| repo.index())
        .is_ok_and(|index| index.has_conflicts());
    CheckoutPulled {
        code: if code == 0 && conflicted { 1 } else { code },
        output,
    }
}

/// One tracked file's uncommitted change against `HEAD`, as `git diff` prints it.
/// `--literal-pathspecs`, so a path with pathspec magic in its name is a file name.
pub(crate) fn uncommitted_change_of(in_repo: &str, path: &str) -> Result<String, String> {
    let run = run_in(
        in_repo,
        "git",
        &["--literal-pathspecs", "diff", "--no-color", "--no-ext-diff", "HEAD", "--", path],
    )
    .map_err(|why| why.to_string())?;
    if !run.status.success() {
        return Err(said(&run));
    }
    Ok(String::from_utf8_lossy(&run.stdout).into_owned())
}

/// What a path holds in the working tree of the checkout.
pub(crate) enum Working {
    /// Nothing is there: a deletion, where the tree has the path.
    Gone,
    /// A file, or a link whose target is the content.
    File(Vec<u8>, git2::FileMode),
    /// Something that is neither, or that would not be read: not ours to send or to call the same.
    Unreadable,
}

pub(crate) fn working(repo: &Repository, path: &str) -> Working {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;
    let Some(at) = repo.workdir().map(|root| root.join(path)) else {
        return Working::Unreadable;
    };
    let meta = match std::fs::symlink_metadata(&at) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Working::Gone,
        Err(_) => return Working::Unreadable,
    };
    if meta.file_type().is_symlink() {
        return match std::fs::read_link(&at) {
            Ok(target) => Working::File(target.as_os_str().as_bytes().to_vec(), git2::FileMode::Link),
            Err(_) => Working::Unreadable,
        };
    }
    if !meta.is_file() {
        return Working::Unreadable;
    }
    let mode = match meta.permissions().mode() & 0o111 {
        0 => git2::FileMode::Blob,
        _ => git2::FileMode::BlobExecutable,
    };
    match std::fs::read(&at) {
        Ok(content) => Working::File(content, mode),
        Err(_) => Working::Unreadable,
    }
}

/// Whether the working-tree file at `path` is, byte for byte and in mode, what
/// `tree` holds there. A path gone from both is the same.
pub(crate) fn same_as_the_tree(repo: &Repository, tree: &git2::Tree<'_>, path: &str) -> bool {
    let held = tree.get_path(Path::new(path)).ok();
    match (working(repo, path), held) {
        (Working::Gone, None) => true,
        (Working::File(content, mode), Some(entry)) => {
            entry.filemode() == i32::from(mode)
                && repo.find_blob(entry.id()).is_ok_and(|blob| blob.content() == content)
        }
        _ => false,
    }
}

/// Whether `tip` already holds the owner's uncommitted change to `path`: their
/// change, merged with main's over what `here` has, gives back main's file
/// exactly. Main took the change and moved on (a comment reworded beside it), so
/// the pull keeps everything of theirs.
fn held_by_the_tree(repo: &Repository, here: &git2::Tree<'_>, tip: &git2::Tree<'_>, path: &str) -> bool {
    let Working::File(content, mode) = working(repo, path) else {
        return false;
    };
    let (Ok(was), Ok(main)) = (here.get_path(Path::new(path)), tip.get_path(Path::new(path))) else {
        return false;
    };
    if main.filemode() != i32::from(mode) {
        return false;
    }
    let (Some(root), Ok(owners)) = (repo.workdir(), repo.blob(&content)) else {
        return false;
    };
    // `--object-id`, so nothing is written beside the owner's files.
    let Ok(merged) = run_in(
        &root.to_string_lossy(),
        "git",
        &["merge-file", "-p", "--object-id", &owners.to_string(), &was.id().to_string(), &main.id().to_string()],
    ) else {
        return false;
    };
    merged.status.success()
        && repo
            .find_blob(main.id())
            .is_ok_and(|blob| blob.content() == merged.stdout.as_slice())
}

/// The paths `theirs` changed since it and `ours` parted.
pub(crate) fn changed_between(repo: &Repository, ours: git2::Oid, theirs: git2::Oid) -> Vec<String> {
    let tree = |oid| repo.find_commit(oid).and_then(|commit| commit.tree()).ok();
    let Some(parted) = repo.merge_base(ours, theirs).ok() else {
        return Vec::new();
    };
    let (Some(from), Some(to)) = (tree(parted), tree(theirs)) else {
        return Vec::new();
    };
    let Ok(diff) = repo.diff_tree_to_tree(Some(&from), Some(&to), None) else {
        return Vec::new();
    };
    diff.deltas()
        .filter_map(|delta| delta.new_file().path().or(delta.old_file().path()))
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// Whether the forge is comparing this pull request against the commit its
/// branch was written on top of.
///
/// **Read locally, and that is the whole reason it costs nothing.** Every
/// worktree Armada cuts shares one object database with the repository they
/// were cut from, so the branch's own commits are here — and where the branch
/// and the base part company is a question git answers without a network.
///
/// **Silence rather than a guess.** A merge base this cannot compute is
/// [`Rendering::Unreadable`], never `AsWritten`, because the caller's answer to
/// a superseded base is to close and reopen a person's pull request.
pub(crate) fn rendering(in_repo: &str, base: &str, pinned: &str, head: &str) -> Rendering {
    let Ok(run) = read_in(in_repo, "git", &["merge-base", head, base]) else {
        return Rendering::Unreadable;
    };
    let written_on = last_line(&run);
    if !run.status.success() || written_on.is_empty() {
        return Rendering::Unreadable;
    }
    match written_on == pinned {
        true => Rendering::AsWritten,
        false => Rendering::FromASupersededBase {
            pinned: pinned.to_string(),
            written_on,
        },
    }
}

/// The `N` fields of one tab-separated line, or `None` where it is not `N`.
///
/// **A split and not a parse.** `--jq` did the reading on the forge's side, so
/// what arrives is one line of text and `store` and `ipc` stay the only two
/// crates that deserialise anything. A field the forge left blank makes the
/// whole reading `None`, because a pull request with no base branch is not a
/// thing this could act on.
pub(crate) fn fields<const N: usize>(said: &str) -> Option<[&str; N]> {
    let read: [&str; N] = said
        .split('\t')
        .map(str::trim)
        .collect::<Vec<&str>>()
        .try_into()
        .ok()?;
    read.iter().all(|field| !field.is_empty()).then_some(read)
}
