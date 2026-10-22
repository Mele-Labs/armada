//! The forge: where a pull request is opened, read, reviewed and merged, and
//! where CI runs are asked about.
//!
//! # Its own trait, so a forge is one adapter
//!
//! [`Delivery`](crate::Delivery) is git's half of publishing and holds a
//! `Forge` as its supertrait, so a caller holding `V: Delivery` reaches both.
//! Everything here talks to the forge alone and to no repository on disk, which
//! is what lets a second forge adapter replace the first without touching git's.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{
    Base, FiledIssue, Merged, NotDelivered, NotFiled, NotMerged, NotRerun, Opened, Remark, Rerun,
    Review, UnderReview, WhatBecameOfIt, Worktree,
};

/// Asking the forge about pull requests and CI, and the few writes a person
/// presses for.
///
/// **Every method is the operator's credentials**, for
/// [`Delivery`](crate::Delivery)'s reason: a Drone holds no type that
/// implements this.
pub trait Forge {
    /// Open a pull request from the branch into the base.
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered>;

    /// What the people and the machines on the forge say about a pull request
    /// nobody has merged yet.
    ///
    /// **Asked only where [`landed`](crate::Delivery::landed) answered
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
    /// **No `Result`**, for [`landed`](crate::Delivery::landed)'s reason exactly: no
    /// tool, not signed in and a forge that would not answer are one silence,
    /// [`UnderReview::unreadable`].
    ///
    /// **Nothing here is a verdict and nothing here passes a Check.** See
    /// [`UnderReview`], whose module carries that argument in full.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](crate::Delivery::landed)'s reason.
    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview;

    /// The comments left on individual lines of the diff, which
    /// [`under_review`](Forge::under_review) does not fetch. Asked by a
    /// person opening a Job's comments, by a press taking some up, and by the
    /// sweep's turn on an open pull request, to count them on the card — the
    /// owner's 2 Oct 2026 call. A Job read asks the forge nothing.
    ///
    /// **`None` where the forge did not answer**, one silence for
    /// [`landed`](crate::Delivery::landed)'s reason. Not an empty list, because the
    /// sweep keeps its last count on a silence and would count an empty list
    /// as zero. A person's read takes `None` as nothing more to add.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](crate::Delivery::landed)'s reason.
    fn inline_remarks(&self, in_repo: &str, pull_request: &str) -> Option<Vec<Remark>>;

    /// The commit the base branch stands at on the forge now. **One cheap
    /// ref lookup**, asked to learn whether main moved before anything dearer
    /// is. `None` is the forge's silence, for [`landed`](crate::Delivery::landed)'s
    /// reason.
    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String>;

    /// The CI jobs the forge ran on one commit. **`None` is silence; an empty
    /// list is a commit nothing ran on**, which a caller must not read as a
    /// pass. **One job a name, its newest run**: a commit the queue and then the
    /// base branch both ran has each job twice, and a caller comparing the failed
    /// names of two commits would see a job that one run failed and the next passed.
    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<crate::CiRuns>;

    /// The commit the merge queue last tested pull request `number` on, against `base`: its
    /// newest run, for what the queue failed once the entry is gone. `None` where the queue never
    /// ran it, or the forge would not say.
    fn queue_commit_of(&self, in_repo: &str, base: &str, number: u64) -> Option<String>;

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

    /// One pull request as [`pull_watch`](Forge::pull_watch) reads it, **by its number**. `None`
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
    /// [`landed`](crate::Delivery::landed)'s reason.
    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestDiff>;

    /// Start a pull request's failed CI runs again. #905.
    ///
    /// **A write to the forge, taken only from a person's press**, like [`merge`](Forge::merge).
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
    /// caller reads it with [`landed`](crate::Delivery::landed), which is the same
    /// question a sweep asks about a pull request nobody pressed anything for.
    ///
    /// `in_repo` is the repository every worktree was cut from, for
    /// [`landed`](crate::Delivery::landed)'s reason — a Job's own worktree is
    /// reclaimed long before anybody merges its work.
    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged>;

    /// Turn on the forge's auto-merge for a pull request Fleet just opened, so
    /// the forge merges it once its required checks pass. 23.24.
    ///
    /// **Merges nothing itself**, so it is not [`merge`](Forge::merge)'s
    /// press. The refusal is the forge's own sentence, because the usual one
    /// is a repository that does not allow auto-merge, and the person has to
    /// read that rather than a code.
    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// Take back the forge's auto-merge on a pull request, so it waits for a person to merge it.
    ///
    /// **Merges nothing**, as [`enable_auto_merge`](Forge::enable_auto_merge) does not. The
    /// refusal is the forge's own sentence, as that one's is.
    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// What the forge says about one pull request, whoever opened it: draft or
    /// open or merged or closed, its branch, whether auto-merge is asked for.
    /// **`None` is the forge's silence**, for [`landed`](crate::Delivery::landed)'s
    /// reason. `in_repo` is the repository to run from.
    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestFacts>;

    /// Take a draft pull request out of draft.
    ///
    /// **A write to the forge, taken only from a person's ask**, like
    /// [`merge`](Forge::merge): it tells everyone watching the repository
    /// that the work is ready to be read. The refusal is the forge's own
    /// sentence, as [`enable_auto_merge`](Forge::enable_auto_merge)'s is.
    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String>;

    /// What a person reads on one pull request's page: the conversation, the
    /// forge's checks, the files and commits. **`None` is the forge's
    /// silence**, for [`landed`](crate::Delivery::landed)'s reason. A list the reader
    /// capped says by how much, in the facts.
    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<crate::PullRequestPageFacts>;

    /// One file's patch on a pull request, read when a person opens the file.
    /// **`None` is the forge's silence**, for [`landed`](crate::Delivery::landed)'s
    /// reason; a file the forge gave no patch for is
    /// [`FilePatch::NoPatch`], which is an answer.
    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<crate::FilePatch>;

    /// Every file's patch on a pull request, in the forge's order, from the
    /// same one read as [`pull_request_file_patch`](Forge::pull_request_file_patch).
    /// **`None` is the forge's silence**, for [`landed`](crate::Delivery::landed)'s reason.
    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<crate::PatchedFile>>;

    /// Write on a pull request: a comment, an approval, a request for changes.
    ///
    /// **A write to the forge, taken only from a person's ask**, like
    /// [`mark_ready`](Forge::mark_ready). The body travels on the tool's
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
    /// silence, for [`landed`](crate::Delivery::landed)'s reason; an empty list is an answer.
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

    /// What the forge says became of a pull request, before this machine reads
    /// what an open one is compared against. The forge's half of
    /// [`landed`](crate::Delivery::landed), and silent for its reason.
    fn landing(&self, in_repo: &str, pull_request: &str) -> LandingOnTheForge;

    /// Open a pull request from `branch`, already pushed, into `base`, and
    /// answer its address. **Not [`open_for_review`](Forge::open_for_review)**:
    /// no worktree, no draft, and no look for one already open, because the
    /// caller has just pushed a branch nobody opened anything from.
    fn open_branch_for_review(
        &self,
        in_repo: &str,
        base: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<String, NotDelivered>;
}

/// [`Forge::landing`]'s answer: what became of the pull request, and for an
/// open one the two commits git compares to say how it is rendered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LandingOnTheForge {
    /// What became of it. **An open one's rendering is
    /// [`Rendering::Unreadable`](crate::Rendering::Unreadable) until
    /// [`landed`](crate::Delivery::landed) reads it from `compared`.**
    pub became: WhatBecameOfIt,
    /// The commit the forge pinned as the base, and the head's. `None` on
    /// anything but an open pull request.
    pub compared: Option<Compared>,
}

/// The two commits an open pull request is rendered between.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compared {
    /// The base commit the forge is comparing against.
    pub pinned: String,
    /// The head of the branch.
    pub head: String,
}
