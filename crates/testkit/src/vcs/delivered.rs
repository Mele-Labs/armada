//! What the fake's version control was asked to do to a Job's branch.

use adapter_traits::Review;

/// One thing this fake was asked to do to a Job's branch, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivered {
    /// The branch was put on top of `base`.
    BroughtUpToDate { branch: String, base: String },
    /// Auto-merge was turned on for this pull request.
    AutoMerge { pull_request: String },
    /// The branch was pushed.
    Pushed { branch: String },
    /// The branch was pushed `--force-with-lease`, over history this fake was
    /// told had been rewritten.
    PushedForcing { branch: String },
    /// A pull request was opened, carrying this.
    OpenedForReview { base: String, review: Review },
    /// The forge was asked what became of the branch's pull request. **Counted
    /// as well as answered**, because the whole design of the asking is how
    /// rarely it happens — a test that could not see the calls could not tell a
    /// sweep that asks once from one that asks every turn.
    AskedWhatBecameOfIt { pull_request: String },
    /// The forge was asked who has looked at an open pull request, what ran
    /// against it and what anybody wrote on it. **Counted for
    /// [`AskedWhatBecameOfIt`](Delivered::AskedWhatBecameOfIt)'s reason**: this
    /// rides the same rotation, and a test that could not see the calls could
    /// not tell a sweep that asks once from one that asks about a merged pull
    /// request it should have stopped asking about.
    AskedWhatIsUnderReview { pull_request: String },
    /// The forge was asked for inline diff comments. Counted apart from
    /// [`AskedWhatIsUnderReview`]: the sweep pays for it only on a turn that
    /// found the pull request open and read it.
    AskedForInlineRemarks { pull_request: String },
    /// The forge was asked for a pull request's diff, for a Code Review Job's review.
    AskedForTheDiff { pull_request: String },
    /// The forge was asked to start a pull request's failed CI runs again. A write, like a merge.
    RerunFailed { pull_request: String },
    /// An issue was filed on the forge. A write, on a person's confirm. #906.
    FiledIssue { title: String },
    /// The branch was asked to be kept current against a base that moved —
    /// `Delivery::kept_current`, in place of the close-and-reopen this
    /// replaced. **Counted for
    /// [`AskedWhatBecameOfIt`](Delivered::AskedWhatBecameOfIt)'s reason**: a
    /// test that could not see it could not tell once from every sweep.
    KeptCurrent { worktree: String, base: String },
    /// The repository every worktree is cut from was asked to catch up.
    CaughtTheRepositoryUp { base: String },
    /// The forge was asked to merge a pull request. **The one write to a
    /// repository Fleet did not make**, so a test that could not see it could
    /// not tell a press that merged from one that only moved a Job.
    Merged { pull_request: String },
    /// The work was landed by a merge commit pushed onto the base —
    /// `Delivery::merge_by_push`, a Manifest's `merge_by: push`.
    MergedByPush {
        handle: String,
        pull_request: Option<u64>,
    },
    /// The base the remote holds was merged into the branch, to be gated again.
    MergedTheBaseIn { branch: String },
    /// The branch's head was read for its Checks to run on before it lands.
    ReadTheUncheckedHead { branch: String },
    /// The branch was put back from that merge, its gate having gone red.
    PutBack { branch: String },
    /// A merge a killed process left part-way through was cleared.
    SettledWorktree { branch: String },
}
