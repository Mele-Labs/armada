//! Getting a finished Job's work back to the branch it was cut from: the base
//! it merges into, catching up to it, the push, and the pull request.
//!
//! # A separate trait, so [`Vcs`](crate::Vcs) keeps having no push
//!
//! `Vcs` is what creates a Job's worktree and commits into it, and its whole
//! shape is that a holder of it cannot publish anything. Adding a push there
//! would put the capability one method away from every caller that only wanted
//! a worktree. Delivery is held by the one caller that is allowed to reach a
//! remote, and it is a different trait so that the two cannot be confused.
//!
//! # The ordinary outcomes are variants, not errors
//!
//! A repository with no remote, and a base branch that moved and now
//! conflicts, are both ordinary. Neither is a failed Job — the Checks passed
//! either way — so each is a variant of what the call *returned* rather than
//! something a caller has to catch. What is left is a tool that would not run,
//! which is [`NotDelivered`] and has one shape.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{Landable, Remark, UncheckedHead, UnderReview, Worktree, WorktreeSpec};

mod outcomes;
mod standing;

pub use outcomes::*;
pub use standing::*;

/// Publishing a Job's work: what it merges into, catching up to it, the push,
/// and the pull request.
///
/// # Every method is the operator's credentials, and none of them is a Drone's
///
/// Fleet runs in the operator's own environment, so a push authenticates the
/// way the operator's own shell would. A Drone's five variables carry none of
/// that and a Drone holds no type that implements this.
pub trait Delivery {
    /// The branch this Job's work merges into.
    ///
    /// `declared` is what the Manifest said, or `None`. **Inference is the
    /// fallback, never the override** — a declared base that the repository
    /// does not have is refused rather than quietly replaced with a guess.
    ///
    /// `Ok(None)` is a repository that names no base and no candidate is there,
    /// which is ordinary in a repository that has never had a default branch.
    fn base(
        &self,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered>;

    /// Where the branch stands against the base. **Asked before anything is
    /// moved**, so that a branch which is not behind costs one comparison.
    fn standing(&self, worktree: &Worktree, base: &Base) -> Result<Standing, NotDelivered>;

    /// How many commits the branch holds that the base has not got.
    ///
    /// **Not a variant of [`Standing`]**, because a branch can be ahead and
    /// behind at once. Asked where Fleet's own commit found nothing new: a
    /// branch somebody else committed on still has work to send out.
    fn ahead(&self, worktree: &Worktree, base: &Base) -> Result<usize, NotDelivered>;

    /// Where the base branch stands against the one the forge would merge into.
    ///
    /// **Asked at the pull request and nowhere else.** It changes nothing about
    /// what is merged in — that is the remote's copy of the base, see
    /// [`fetch_base`](Delivery::fetch_base) — and it is read so that the pull
    /// request can say what it is carrying.
    fn base_on_the_remote(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BaseOnTheRemote, NotDelivered>;

    /// Fetch the base from the remote, so that [`standing`](Delivery::standing),
    /// [`ahead`](Delivery::ahead), [`bring_up_to_date`](Delivery::bring_up_to_date)
    /// and [`kept_current`](Delivery::kept_current) read what the remote holds.
    ///
    /// **Asked first by every caller of those four.** They read the remote's
    /// copy of the base where this machine has one and the local branch where it
    /// has none, and none of them fetches: this is the one call that touches the
    /// network, so the one place a failure is seen and a caller decides what it
    /// costs. `in_repo` is the repository or any worktree of it.
    fn fetch_base(&self, in_repo: &str, base: &str) -> BaseFetched;

    /// Put the branch on top of the base, carrying uncommitted work across.
    ///
    /// **Uncommitted work is never destroyed.** Whatever the worktree is
    /// holding comes across with it, and where it cannot, the branch is put
    /// back where it was.
    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered>;

    /// Merge the local branch `branch` into the one this worktree has checked
    /// out. **A conflict is put back and never left in the files**, because
    /// the caller is Fleet bringing a repair's fix onto a Job's branch and no
    /// Drone follows to clear markers.
    fn merge_branch(&self, worktree: &Worktree, branch: &str)
        -> Result<BranchMerged, NotDelivered>;

    /// Put the branch on the remote, under its own name.
    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered>;

    /// The same push, `--force-with-lease`. **The one call this trait makes
    /// that may overwrite what is on the remote**, and the only one that may:
    /// every other write here either creates something that was not there —
    /// the ordinary [`push`](Delivery::push), a pull request — or is refused
    /// where history would move under it. This exists for the one case where
    /// moving it is the point — [`bring_up_to_date`](Delivery::bring_up_to_date)
    /// rewrote the branch, and the remote's own copy is what it is rewriting.
    /// `#663`.
    fn push_forcing(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered>;

    /// Open a pull request from the branch into the base.
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered>;

    /// What became of a pull request that was opened.
    ///
    /// **No `Result`, unlike every method above it.** There is nothing a caller
    /// could do differently about a forge that would not answer than about a
    /// pull request it cannot find: both mean ask again later, so both are
    /// [`Landing::Unknown`] and there is no error to handle. The same reasoning
    /// `already_open` has kept since delivery shipped, made into a type.
    ///
    /// **Asked by address and not by branch.** A branch is what
    /// [`open_for_review`](Delivery::open_for_review) had, and a merged branch
    /// is usually deleted — so the address the record kept is the only handle
    /// that still resolves once the answer is the interesting one.
    ///
    /// `in_repo` is an absolute path to run from, and **the one method here
    /// that does not take a [`Worktree`]**: a Job's own worktree is reclaimed
    /// long before anybody merges its work, so the caller passes the repository
    /// every worktree was cut from, which is not one.
    fn landed(&self, in_repo: &str, pull_request: &str) -> WhatBecameOfIt;

    /// What the people and the machines on the forge say about a pull request
    /// nobody has merged yet.
    ///
    /// **Asked only where [`landed`](Delivery::landed) answered
    /// [`Landing::Open`].** A merged pull request has left the sweep's rotation
    /// and a closed one is terminal, so neither has an answer here anybody
    /// would act on — the rule [`Rendering`] keeps by riding on that variant.
    ///
    /// **A second call and not four more fields on `landed`**, which is the
    /// reverse of `#427`'s argument for folding [`Rendering`] in, because the
    /// shape is the reverse: those were four scalars on one line beside the
    /// state, and these are three lists, one carrying whatever anybody typed
    /// into a comment box. The callers differ too — a merge pressed from a
    /// screen reads `landed` and has no use for any of this.
    ///
    /// **No `Result`**, for [`landed`](Delivery::landed)'s reason exactly: no
    /// tool, not signed in and a forge that would not answer are one silence,
    /// [`UnderReview::unreadable`].
    ///
    /// **Nothing here is a verdict and nothing here passes a Check.** See
    /// [`UnderReview`], whose module carries that argument in full.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview;

    /// The comments left on individual lines of the diff, which
    /// [`under_review`](Delivery::under_review) does not fetch. Asked by a
    /// person opening a Job's comments, by a press taking some up, and by the
    /// sweep's turn on an open pull request, to count them on the card — the
    /// owner's 2 Oct 2026 call. A Job read asks the forge nothing.
    ///
    /// **`None` where the forge did not answer**, one silence for
    /// [`landed`](Delivery::landed)'s reason. Not an empty list, because the
    /// sweep keeps its last count on a silence and would count an empty list
    /// as zero. A person's read takes `None` as nothing more to add.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn inline_remarks(&self, in_repo: &str, pull_request: &str) -> Option<Vec<Remark>>;

    /// The commit the base branch stands at on the forge now. **One cheap
    /// ref lookup**, asked to learn whether main moved before anything dearer
    /// is. `None` is the forge's silence, for [`landed`](Delivery::landed)'s
    /// reason.
    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String>;

    /// The CI jobs the forge ran on one commit. **`None` is silence; an empty
    /// list is a commit nothing ran on**, which a caller must not read as a
    /// pass. **One job a name, its newest run**: a commit the queue and then the
    /// base branch both ran has each job twice, and a caller comparing the failed
    /// names of two commits would see a job that one run failed and the next passed.
    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<crate::CiRuns>;

    /// The text of one CI job's log, **its tail and bounded**: a failure is
    /// printed last, and a log runs to megabytes. Timestamps the forge puts on
    /// every line are taken off. `None` where it would not give one.
    fn ci_log(&self, in_repo: &str, run: &crate::CiRun) -> Option<crate::FromOutside>;

    /// The pull request that put `commit` on the base, as the forge maps a
    /// commit to its pull request, falling back to what a merge commit's own
    /// message says. `None` where neither names one, as a direct push does not.
    fn merged_by(&self, in_repo: &str, commit: &str) -> Option<crate::MergedPull>;

    /// The repository's open pull requests, each with its `ci`. **One forge
    /// call for all of them**, so the cost does not grow with the number open.
    /// `None` is the forge's silence; an empty list is a repository with none
    /// open.
    fn open_pull_requests(&self, in_repo: &str, gate: &str) -> Option<crate::OpenPulls>;

    /// One pull request of that list, **by its number**, for a pull request someone holds that the
    /// list's cap left out. `None` is the forge's silence, and also a pull request that is no
    /// longer open: either way it is not listed.
    fn open_pull_request(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<crate::OpenPull>;

    /// The merge queue of `base`, one forge call. `None` is the forge's silence.
    fn merge_queue(&self, in_repo: &str, base: &str) -> Option<crate::MergeQueue>;

    /// The same pull requests as a watcher reads them: every check on the newest commit with its
    /// log address, whether it conflicts with the base, and where the merge queue holds it. **One
    /// forge call for all of them.** `None` is the forge's silence.
    fn pull_watch(&self, in_repo: &str, gate: &str) -> Option<crate::WatchedPulls>;

    /// One pull request as [`pull_watch`](Delivery::pull_watch) reads it, **by its number**. `None`
    /// is the forge's silence, and also a pull request that is no longer open.
    fn watched_pull(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<crate::WatchedPull>;

    /// The login the forge's own command line is signed in as on this machine, the person whose
    /// pull requests are "mine" beside the account Armada is signed in as. `None` is a forge
    /// that would not say, or no login. It does not change under a run, so a caller keeps it.
    fn forge_login(&self, in_repo: &str) -> Option<String>;

    /// The newest `limit` pull requests merged into `base`, newest first, in
    /// one forge call. `None` is the forge's silence.
    fn recently_merged_pull_requests(
        &self,
        in_repo: &str,
        base: &str,
        limit: usize,
    ) -> Option<crate::RecentlyMergedPulls>;

    /// The diff of a pull request somebody else opened, for a Code Review Job whose review is
    /// checked against it. #903. **`None` is the forge's silence**, for
    /// [`landed`](Delivery::landed)'s reason.
    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestDiff>;

    /// Start a pull request's failed CI runs again. #905.
    ///
    /// **A write to the forge, taken only from a person's press**, like [`merge`](Delivery::merge).
    /// Nothing is posted on the pull request.
    fn rerun_failed(&self, in_repo: &str, pull_request: &str) -> Result<Rerun, NotRerun>;

    /// File an issue on the forge from a review finding. #906.
    ///
    /// **A write to the forge, taken only from a person's confirm** of a draft they could edit,
    /// because the issue is written as them.
    fn file_issue(&self, in_repo: &str, title: &str, body: &str) -> Result<FiledIssue, NotFiled>;

    /// Merge a pull request Armada opened.
    ///
    /// **The one method on this trait that writes into a repository nobody on
    /// this machine holds.** Every other act Fleet takes is confined to a
    /// worktree Fleet made; a push puts a branch somewhere nobody merges from,
    /// and this changes what everybody else builds on. A caller logs it before
    /// it happens.
    ///
    /// **Nothing here decides whether it should be merged.** That is a
    /// person's, at a human gate, and this is the act they pressed for.
    ///
    /// **What it merged into is not returned**, for [`Merged`]'s reason: the
    /// caller reads it with [`landed`](Delivery::landed), which is the same
    /// question a sweep asks about a pull request nobody pressed anything for.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason — a Job's own worktree is
    /// reclaimed long before anybody merges its work.
    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged>;

    /// Turn on the forge's auto-merge for a pull request Fleet just opened, so
    /// the forge merges it once its required checks pass. 23.24.
    ///
    /// **Merges nothing itself**, so it is not [`merge`](Delivery::merge)'s
    /// press. The refusal is the forge's own sentence, because the usual one
    /// is a repository that does not allow auto-merge, and the person has to
    /// read that rather than a code.
    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// Take back the forge's auto-merge on a pull request, so it waits for a person to merge it.
    ///
    /// **Merges nothing**, as [`enable_auto_merge`](Delivery::enable_auto_merge) does not. The
    /// refusal is the forge's own sentence, as that one's is.
    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// What the forge says about one pull request, whoever opened it: draft or
    /// open or merged or closed, its branch, whether auto-merge is asked for.
    /// **`None` is the forge's silence**, for [`landed`](Delivery::landed)'s
    /// reason. `in_repo` is the repository to run from.
    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestFacts>;

    /// Take a draft pull request out of draft.
    ///
    /// **A write to the forge, taken only from a person's ask**, like
    /// [`merge`](Delivery::merge): it tells everyone watching the repository
    /// that the work is ready to be read. The refusal is the forge's own
    /// sentence, as [`enable_auto_merge`](Delivery::enable_auto_merge)'s is.
    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// What a person reads on one pull request's page: the conversation, the
    /// forge's checks, the files and commits. **`None` is the forge's
    /// silence**, for [`landed`](Delivery::landed)'s reason. A list the reader
    /// capped says by how much, in the facts.
    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestPageFacts>;

    /// One file's patch on a pull request, read when a person opens the file.
    /// **`None` is the forge's silence**, for [`landed`](Delivery::landed)'s
    /// reason; a file the forge gave no patch for is
    /// [`FilePatch::NoPatch`], which is an answer.
    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<crate::FilePatch>;

    /// Every file's patch on a pull request, in the forge's order, from the
    /// same one read as [`pull_request_file_patch`](Delivery::pull_request_file_patch).
    /// **`None` is the forge's silence**, for [`landed`](Delivery::landed)'s reason.
    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<crate::PatchedFile>>;

    /// Write on a pull request: a comment, an approval, a request for changes.
    ///
    /// **A write to the forge, taken only from a person's ask**, like
    /// [`mark_ready`](Delivery::mark_ready). The body travels on the tool's
    /// standard input and never in its arguments. The refusal is the forge's
    /// own sentence.
    fn write_pull_request(
        &self,
        in_repo: &str,
        pull_request: &str,
        write: &crate::PullRequestWrite,
    ) -> Result<(), String>;

    /// One of the four lists of pull requests waiting on a person, across every repository named
    /// in `ask`, newest-updated first and at most `ask.most` rows. **One forge call**, whoever
    /// opened the pull requests and whether or not a Session holds them. `None` is the forge's
    /// silence, for [`landed`](Delivery::landed)'s reason; an empty list is an answer.
    fn pull_review_list(
        &self,
        in_repo: &str,
        ask: &crate::ReviewListAsk<'_>,
    ) -> Option<Vec<crate::ListedPull>>;

    /// Resolve a conversation on a pull request's diff, or open it again. **A write to the forge,
    /// taken at once and seen by everyone watching**, so only from a person's press. Answers
    /// whether the thread is now resolved, as the forge holds it. `Err` is the forge's own
    /// sentence.
    fn resolve_review_thread(
        &self,
        in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String>;

    /// Land a Job's branch by making the `--no-ff` merge commit here and
    /// pushing the base, never forced — `merge_by: push`.
    ///
    /// **Only a branch that already holds the base lands.** One that does not,
    /// or a base that moves before the push, is [`NotMerged::BaseMoved`]:
    /// the merged tree is the branch's own.
    ///
    /// **And only a head `landable` names**, or it is
    /// [`NotMerged::Unchecked`]. Read in the same call as the push, so nothing
    /// that moves the branch between a Check and the push is landed unread.
    ///
    /// `handle` derives the branch, `declared` is `base:` in `armada.yml`, and
    /// `pull_request` is named in the merge's subject; the forge reads it
    /// merged once its head is in the base.
    fn merge_by_push(
        &self,
        in_repo: &str,
        handle: &str,
        declared: Option<&str>,
        pull_request: Option<u64>,
        landable: Landable<'_>,
    ) -> Result<PushedOntoBase, NotMerged>;

    /// The tree a worktree holds as it stands, untracked files included: the
    /// tree Fleet's commit would make of it. What a gate's Checks read.
    fn tree_as_it_stands(&self, worktree: &Worktree) -> Result<String, NotDelivered>;

    /// The branch's head in its own worktree, and what changed since `checked`
    /// and over the base the remote holds, for its Checks to read before
    /// [`merge_by_push`](Delivery::merge_by_push) is asked again. `None` reads
    /// every path as changed.
    ///
    /// **Refused where the worktree holds changes nothing committed**, for
    /// [`merge_the_moved_base_in`](Delivery::merge_the_moved_base_in)'s reason.
    fn the_unchecked_head(
        &self,
        in_repo: &str,
        worktree: &Worktree,
        declared: Option<&str>,
        checked: Option<&str>,
    ) -> Result<UncheckedHead, NotMerged>;

    /// Merge the base, as the remote holds it now, into a Job's branch in its
    /// own worktree — never a rebase — so the branch can be gated again before
    /// [`merge_by_push`](Delivery::merge_by_push) is asked once more.
    ///
    /// **A conflict leaves the branch exactly as it was**, as
    /// [`NotMerged::Conflicted`] naming the files. So does a worktree holding
    /// changes nothing committed: the Checks would read a tree that would not
    /// land.
    fn merge_the_moved_base_in(
        &self,
        in_repo: &str,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<BaseMergedIn, NotMerged>;

    /// Take the branch back to where it was before
    /// [`merge_the_moved_base_in`](Delivery::merge_the_moved_base_in), where
    /// its gate went red. **Only while its head is still that merge**, so
    /// nothing committed since is thrown away.
    fn put_back(&self, worktree: &Worktree, merged: &BaseMergedIn) -> Result<(), NotDelivered>;

    /// Clear a merge a killed process left part-way through a Job's worktree,
    /// so the next gate reads the branch as it was committed. **A no-op where
    /// no merge is in progress**, and it never touches a commit.
    fn settle_worktree(&self, worktree: &Worktree) -> Result<(), NotDelivered>;

    /// Rebase a Job's branch onto a base that has moved, and push the result —
    /// in place of closing and reopening the pull request. `#663`.
    ///
    /// **Not `#427`'s close-and-reopen.** That measured the alternatives for
    /// *rendering* a pull request afresh without changing what it carries; this
    /// changes what it carries, which is the fix for the finding that motivated
    /// it in the first place — a branch behind its base with conflicts, which
    /// only a rebase resolves.
    ///
    /// **Works whether or not a Job's own worktree is still there.** A finished
    /// Job's worktree is very often reclaimed before its pull request is
    /// merged, so this works in `spec`'s worktree where it is on disk, and
    /// otherwise in a scratch checkout onto the same branch, attached and
    /// detached inside this one call.
    ///
    /// **Called only for [`Rendering::FromASupersededBase`]**, and at most once
    /// per base — see `fleet::currency`, which holds that guard durably rather
    /// than in memory, because losing it on a restart is `#663`'s own finding.
    ///
    /// `spec` is the Job's worktree as Fleet looks it up: the pool slot it
    /// still holds, where its branch is checked out, or the derived path a
    /// scratch checkout is attached at. `base` is the branch the forge named
    /// as what this merges into.
    fn kept_current(&self, spec: &WorktreeSpec, base: &str) -> KeptCurrent;

    /// The commit a base branch is on, read without a worktree: the remote's
    /// copy as [`fetch_base`](Delivery::fetch_base) last left it, and the local
    /// branch only where the repository has no remote copy.
    ///
    /// **The cheap half of [`kept_current`](Delivery::kept_current)**, asked
    /// first so that a base which has not moved past what was already tried
    /// costs one process rather than a scratch checkout and a rebase. `None`
    /// is a base this could not read — no such branch, an unreadable
    /// repository — and a caller answers that the same as never having seen
    /// it, never as agreement.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](Delivery::landed)'s reason.
    fn base_tip(&self, in_repo: &str, base: &str) -> Option<String>;

    /// Bring the repository every worktree is cut from up to the branch that
    /// just merged.
    ///
    /// **What merged is now what everything else builds on** — `#337` — so the
    /// tree the next Job is cut from should have it. Nothing here is a gate and
    /// nothing fails a Job: every refusal is
    /// [`LeftAlone`](RepositoryStanding::LeftAlone).
    ///
    /// **Never anything but a fast-forward, and never over a person's work.**
    /// This is the one method on this trait that writes into the repository a
    /// person is standing in rather than a Job's worktree, so it declines on
    /// anything it cannot do without merging, rebasing or discarding.
    fn caught_the_repository_up(&self, in_repo: &str, base: &str) -> RepositoryStanding;

    /// Whether `in_repo` already has `commit` on `base`, read in process: the
    /// commit checked out where `base` is, and the local `base` branch where
    /// another is. `false` where the commit is not here at all.
    fn checkout_has(&self, in_repo: &str, base: &str, commit: &str) -> bool;

    /// Fast-forward the local `base` branch to `origin`'s while another branch
    /// is checked out, touching nothing checked out. **False where git refuses**:
    /// a history that will not fast-forward, or `base` checked out in some
    /// worktree.
    fn base_fast_forwarded(&self, in_repo: &str, base: &str) -> bool;

    /// Why `in_repo` cannot be fast-forwarded to `base` on the remote, read for
    /// a person to act on. Fetches the base first so the counts are current.
    /// `None` where the repository or its remote base cannot be read.
    fn held_back(&self, in_repo: &str, base: &str) -> Option<HeldBack>;

    /// Pull the checkout in `in_repo` up to its upstream with a fast-forward
    /// only, setting its uncommitted work aside and putting it back.
    ///
    /// **The press behind a held checkout's one safe command.** Whether the
    /// hold is one a pull can clear is the caller's to have read just before;
    /// this runs the pull and says what git said. **A stash git could not put
    /// back is left where git left it** and reported as a non-zero code.
    fn pulled_the_checkout(&self, in_repo: &str) -> CheckoutPulled;

    /// The unified diff of one tracked file's uncommitted change against `HEAD`
    /// in `in_repo`, as git prints it. **Reads only.** Empty where the file has
    /// no change; `Err` carries git's own words where it would not run.
    ///
    /// The path is taken literally, never as a pathspec, but **what may be asked
    /// for is the caller's to have limited**: this reads any path it is given.
    fn uncommitted_change_of(&self, in_repo: &str, path: &str) -> Result<String, String>;

    /// Whether an edit to the Manifest at `file` can go out as a pull request:
    /// `in_repo` is a repository with an `origin`, and the file is tracked and
    /// not ignored there. **False sends the edit to the file on disk.**
    fn manifest_by_pull_request(&self, in_repo: &str, file: &str) -> bool;

    /// Fetch `base` and read the Manifest at `file` as `origin` holds it.
    /// **Writes nothing but the remote-tracking ref.**
    fn manifest_on_the_base(
        &self,
        in_repo: &str,
        base: &str,
        file: &str,
    ) -> Result<crate::ManifestOnTheBase, NotDelivered>;

    /// Commit the Manifest on top of `sending.parent`, push the commit to
    /// `sending.branch`, and where it opens a pull request open one and turn
    /// auto-merge on.
    ///
    /// **Never touches `in_repo`'s tree, index or HEAD**, which a person may be
    /// working in: the commit is made from objects, not from a checkout.
    fn send_manifest_edit(
        &self,
        in_repo: &str,
        sending: &crate::ManifestEditToSend,
    ) -> Result<crate::ManifestEditSent, NotDelivered>;

    /// Send the uncommitted files of the checkout in `in_repo` up as a pull
    /// request into `sending.base`, auto-merge on: a commit on `origin`'s tip of
    /// the base whose tree is that tip's with each path's working-tree content,
    /// pushed to `sending.branch`.
    ///
    /// **Never touches `in_repo`'s working tree, index or HEAD**, which the
    /// person is standing in: the commit is made from objects. **Refuses a path
    /// the base also changed to something else**, so the commit can never undo
    /// what landed on the base.
    fn send_checkout_as_pull_request(
        &self,
        in_repo: &str,
        sending: &crate::CheckoutToSend,
    ) -> Result<crate::CheckoutSent, NotDelivered>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_base_says_so_and_an_inferred_one_does_not() {
        assert!(Base::Declared("release".into()).was_declared());
        assert!(!Base::Inferred("main".into()).was_declared());
        assert_eq!(Base::Inferred("main".into()).name(), "main");
    }

    #[test]
    fn a_refusal_names_what_was_being_attempted() {
        let refused = NotDelivered::of("the push", "remote rejected");
        assert!(refused.said().starts_with("the push did not happen"));
        assert!(refused.said().ends_with("remote rejected"));
    }

    #[test]
    fn a_review_carries_both_halves_and_offers_no_way_to_change_them() {
        let review = Review::assembled("fix the reader", "## What was checked");
        assert_eq!(review.title(), "fix the reader");
        assert_eq!(review.body(), "## What was checked");
    }
}
