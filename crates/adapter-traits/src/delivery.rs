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

use crate::{Forge, Landable, UncheckedHead, Worktree, WorktreeSpec};

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
///
/// **The forge is [`Forge`], a supertrait**, so this trait is git's half and a
/// holder of it still asks the forge through the same value.
pub trait Delivery: Forge {
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

    /// What became of a pull request that was opened.
    ///
    /// **No `Result`, unlike every method above it.** There is nothing a caller
    /// could do differently about a forge that would not answer than about a
    /// pull request it cannot find: both mean ask again later, so both are
    /// [`Landing::Unknown`] and there is no error to handle. The same reasoning
    /// `already_open` has kept since delivery shipped, made into a type.
    ///
    /// **Asked by address and not by branch.** A branch is what
    /// [`open_for_review`](Forge::open_for_review) had, and a merged branch
    /// is usually deleted — so the address the record kept is the only handle
    /// that still resolves once the answer is the interesting one.
    ///
    /// `in_repo` is an absolute path to run from, and **the one method here
    /// that does not take a [`Worktree`]**: a Job's own worktree is reclaimed
    /// long before anybody merges its work, so the caller passes the repository
    /// every worktree was cut from, which is not one.
    ///
    /// **[`Forge::landing`] and a git read**: the forge says what became of
    /// it, and this machine says what an open one is rendered against.
    fn landed(&self, in_repo: &str, pull_request: &str) -> WhatBecameOfIt;

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
