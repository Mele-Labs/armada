//! The values a delivery step answers with: the base, each outcome of the git and forge calls, and why a merge did not happen.

use super::*;

/// The branch a Job's work merges into, and where the name came from.
///
/// **Both halves matter to a person reading a line about it.** A base that was
/// declared is the repository stating its own answer; an inferred one is
/// Armada's best reading of a repository that did not say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// Named by `base:` in the Manifest.
    Declared(String),
    /// Nothing named one, so this is what the repository looks like it means:
    /// what a clone recorded as the remote's default, or the conventional name.
    Inferred(String),
}

impl Base {
    /// The branch name, which is all git needs.
    pub fn name(&self) -> &str {
        match self {
            Base::Declared(name) | Base::Inferred(name) => name,
        }
    }

    /// Whether the repository said so itself.
    pub fn was_declared(&self) -> bool {
        matches!(self, Base::Declared(_))
    }
}

/// Where a Job's branch stands against its base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// The base holds nothing the branch does not already have. **Nothing
    /// follows from this**, and a caller that announced it would be announcing
    /// a no-op.
    UpToDate,
    /// `commits` landed on the base that the branch has not got.
    Behind { commits: usize },
}

/// Where the local base branch stands against the one the forge has.
///
/// **A different question from [`Standing`], and the one nothing was asking.**
/// `Standing` compares the Job's branch with the base *on this machine*, which
/// is what a rebase needs. A pull request is opened against the base *on the
/// remote*, and nothing compared those two — so a base carrying commits the
/// remote has not got produced a pull request that carried them as well, under
/// a Job that never touched the files in them.
///
/// **Behind is carried alongside ahead** even though only ahead makes a pull
/// request wrong. They are one `graph_ahead_behind` call, and a base behind its
/// remote is a rebase that used a stale reading — worth saying in the same
/// breath rather than discovered separately later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseOnTheRemote {
    /// Nothing to say: they hold the same commits, or the base tracks no
    /// remote branch at all and there is no second reading to compare with.
    Agreed,
    /// They hold different commits. `remote` is the tracking branch's own name
    /// — `origin/main` — because that is what a person types to look at it.
    Apart {
        remote: String,
        /// Commits the local base has that the remote's has not got. **These
        /// are the ones a pull request would carry.**
        ahead: usize,
        /// Commits the remote's base has that the local one has not got.
        behind: usize,
    },
}

/// What fetching the base from the remote came to.
///
/// **A catch-up merges what the remote holds, never the local branch of the
/// same name.** The branch a person has checked out is rarely fast-forwarded,
/// so merging it reports a branch up to date while the forge's pull request
/// conflicts (Job 14, 12 Oct 2026). This is the one answer that says whether
/// the remote's copy is the one now on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseFetched {
    /// `remote`'s copy of the base was fetched just now, and is what a catch-up
    /// reads.
    Fetched { remote: String },
    /// There is no remote copy to prefer: the repository has no remote, or the
    /// remote has never had this branch. **Ordinary**, and the local branch is
    /// the only reading there is.
    Local,
    /// The remote has a copy and it could not be fetched. **Never a licence to
    /// read the local branch or an older fetch**: either is a stale base
    /// presented as a current one. `said` is git's own sentence.
    Failed { remote: String, said: String },
}

/// What catching up to the base came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BroughtUpToDate {
    /// The branch is on the base and there is nothing to resolve.
    Clean { base: String, commits: usize },
    /// The branch is on the base, and the work it carried across landed with
    /// conflicts still in the files. **Work rather than a failure** — resolving
    /// one needs judgement about the code.
    Conflicted { base: String, files: Vec<String> },
    /// Replaying the branch's own commits conflicted, so it was **put back
    /// exactly as it was**. Nothing moved and nothing is half-done.
    PutBack { base: String, files: Vec<String> },
}

/// What merging one local branch into the worktree's own came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchMerged {
    /// The worktree's branch holds the other, by fast-forward or a merge commit.
    Merged,
    /// It would conflict in these files, so the worktree is **put back exactly
    /// as it was** and nothing is left half-merged.
    PutBack { files: Vec<String> },
}

/// What became of the push.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// On the remote, under the branch's own name.
    ToTheRemote { remote: String, branch: String },
    /// **Ordinary, not a failure.** The repository has no remote, so the branch
    /// is the work and a person merges it where it is.
    NoRemote,
}

/// What became of the pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    /// It is open, and the address is what a person clicks.
    PullRequest { url: String },
    /// One is already open for this branch. A redispatched Job pushing again
    /// finds its own.
    AlreadyOpen { url: String },
    /// Nothing reached a remote, so there is nothing to open one from.
    NothingPushed,
    /// Nothing on this machine can open one. **Not a failure of the Job** — the
    /// branch is pushed and a person opens it by hand.
    NoTool { why: String },
}

/// What became of the pull request after it was opened. **The question a
/// person actually has about finished work**, and the one the record could not
/// answer: a merge Fleet did not perform is only ever knowable by asking.
///
/// **Fleet may merge, and what stays a person's is the decision** —
/// [`Delivery::merge`] is the act a person pressed for. This is the other way a
/// pull request settles, and there is no third: either Armada did it and knows,
/// or somebody did it on the forge and Armada finds out on a sweep.
///
/// **Every failure is [`Unknown`](Landing::Unknown)**, which is the rule
/// [`Opened`] already keeps one call earlier: no tool, not signed in, no such
/// pull request and a forge that would not answer are one absence here, because
/// nothing follows differently from any of them. A caller records nothing and
/// asks again later.
///
/// **`Unknown` is not `Open`.** A merge that Armada could not read about must
/// not render as a pull request still waiting for somebody — that is the same
/// sentence as the one the record already got wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Somebody merged it. **The end of the asking** — this never changes back,
    /// so a caller that records it never asks again.
    Merged { url: String },
    /// It is open and nobody has merged it yet — and, because that is the only
    /// state in which the question means anything, what the forge is comparing
    /// it against. A merged pull request's render is nobody's problem, and this
    /// is where that is said in the type rather than in a comment.
    Open { url: String, rendering: Rendering },
    /// It was closed and never merged. Also terminal, and a different sentence:
    /// the work was published and turned down.
    ClosedUnmerged { url: String },
    /// Nothing on this machine could say.
    Unknown,
}

impl Landing {
    /// Whether this is an answer worth writing down. **`Open` is not** — it is
    /// the state a pull request is in from the moment it exists, so recording
    /// it would store the absence of news.
    pub fn is_settled(&self) -> bool {
        matches!(
            self,
            Landing::Merged { .. } | Landing::ClosedUnmerged { .. }
        )
    }
}

/// Everything one ask of the forge answers about a pull request that was
/// opened. **One call and six answers.**
///
/// `#427` asked for the base question to ride the merge question rather than
/// sweep on its own, because `gh pr view` answers both in the same breath and
/// building them apart would be two processes where one would do. So this is
/// what [`Delivery::landed`] returns, and what it carries has grown from the
/// two things a person wants to know about work they cannot see — did it land,
/// and what does it land into — to what a person reads about it before it has:
/// its number, its title, and whether the forge can merge it as it stands.
/// Riding the same call is what `#427`'s argument already was; asking a second
/// time for a value this one already has would be the process this call exists
/// to spend once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhatBecameOfIt {
    /// Whether anybody merged it.
    pub landing: Landing,
    /// The branch it merges into, as the forge names it — not as this machine
    /// infers it, because the forge is the one rendering the comparison.
    ///
    /// **`None` is a forge that did not answer**, which is exactly
    /// [`Landing::Unknown`]'s case and never any other.
    pub base: Option<String>,
    /// The forge's own number for it. **`None` is `Unknown`'s case again** —
    /// a forge that answered anything at all named a number, so this is never
    /// absent beside a `landing` other than [`Landing::Unknown`].
    pub number: Option<u64>,
    /// Its title, as the forge holds it right now.
    ///
    /// **Not the title Armada opened it with.** `Review::title` is what
    /// [`Delivery::open_for_review`] sent; a person may have edited the pull
    /// request's own title since, and this is a later read of the same forge
    /// answering about the same address. `None` on [`Landing::Unknown`], for
    /// [`number`](WhatBecameOfIt::number)'s reason.
    pub title: Option<String>,
    /// Whether the forge can merge it into its base as it stands.
    ///
    /// **A third answer, and not a guess at one of the other two.** A forge
    /// still computing the answer says so rather than picking one, and
    /// [`Mergeable::Unreadable`] is that word carried rather than collapsed
    /// into a conflict nobody has found yet.
    pub mergeable: Mergeable,
    /// When it merged, as the forge says, RFC 3339. **`None` on anything but
    /// [`Landing::Merged`]**, and on a forge that named no instant: a parent
    /// Job's members are read as landed off the merge, and when it happened is
    /// what a person reads beside it (spike 022, slice 6).
    pub merged_at: Option<String>,
}

impl WhatBecameOfIt {
    /// Nothing on this machine could say anything at all. **The one absence**,
    /// for [`Landing::Unknown`]'s reason: no tool, not signed in, no such pull
    /// request and a forge that would not answer are one silence here, and a
    /// caller records nothing and asks again later.
    pub fn unknown() -> WhatBecameOfIt {
        WhatBecameOfIt {
            landing: Landing::Unknown,
            base: None,
            number: None,
            title: None,
            mergeable: Mergeable::Unreadable,
            merged_at: None,
        }
    }
}

/// Whether the forge can merge a pull request into its base as it stands.
///
/// **Not a `bool`.** A forge that has not finished computing the answer says
/// so, and folding that third word into either `Yes` or `No` would be a guess
/// this machine has no grounds for — the same argument [`crate::WhatPeopleSaid`]
/// and [`crate::WhatTheForgeRan`] already make for their own `Unreadable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mergeable {
    /// The forge would merge it, unassisted, right now.
    Yes,
    /// The branch and its base disagree, in the forge's own reading —
    /// [`NotMerged::Conflicted`]'s reading, taken before anybody pressed.
    No,
    /// The forge has not said, or did not answer at all.
    Unreadable,
}

/// Whether the forge is comparing a pull request against the commit its branch
/// actually sits on.
///
/// A forge pins the base at the commit the pull request was opened from and
/// does not move it when the base branch advances underneath. So a pull request
/// that was right when it was opened renders other people's commits as this
/// Job's work once anything else merges — `#427` read 65 files beside the 28 its
/// Job wrote, and only a close and reopen moved it.
///
/// **This is not the defect a pull request's body already warns about.** A base
/// that was ahead of its remote *at open time* is a Job carrying commits it did
/// not write, and the remedy there is a push — [`BaseOnTheRemote`]. This is a
/// base that moved *after*, and no sentence in the body changes what the forge
/// renders beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rendering {
    /// The commit the forge pinned is the one the branch was written on top of,
    /// so the file list beside it is this Job's work and nothing else.
    AsWritten,
    /// The pinned commit has been superseded, so what is rendered beside the
    /// pull request is somebody else's work as well as this Job's.
    FromASupersededBase {
        /// What the forge is comparing against.
        pinned: String,
        /// What it should be comparing against — where the branch and the base
        /// actually part company.
        written_on: String,
    },
    /// Nothing on this machine could say which of the two it is.
    ///
    /// **Not stale**, and the distinction is the whole reason this is three
    /// variants rather than a `bool`: a pull request nobody can check is left
    /// alone rather than nudged, because the nudge closes and reopens it.
    Unreadable,
}

/// What came of asking the forge to merge a pull request.
///
/// **Two ways it is merged and only one of them is this call's doing.** A
/// person who pressed the button on a pull request somebody else had already
/// merged is not looking at a failure — the work is where the press was trying
/// to put it — and every caller does the same thing afterwards either way.
///
/// **It carries nothing.** What it merged into, what the base is on now and
/// whether the forge agrees are read with [`Delivery::landed`], which is the
/// same question a sweep asks about a pull request nobody pressed anything
/// for. A merge that answered them here would be a second reading of one fact,
/// and the two would come to disagree about a Job that was both noticed and
/// pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merged {
    /// The forge took it.
    Taken,
    /// Somebody had already merged it. **Not a refusal.**
    AlreadyMerged,
}

/// What [`Delivery::merge_by_push`] came to.
///
/// **It carries the base, where [`Merged`] carries nothing**: there may be no
/// forge reading to ask, and the push is the reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushedOntoBase {
    /// The branch the work landed on.
    pub base: String,
    pub merged: Merged,
    /// The merge commit that holds the branch on the base: the one just
    /// pushed, or for [`Merged::AlreadyMerged`] the one found there. `None`
    /// where the base holds the branch by some other road.
    pub merge: Option<String>,
}

/// What [`Delivery::merge_the_moved_base_in`] left on a Job's branch: the base
/// the remote holds, merged in as one commit, waiting to be gated again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseMergedIn {
    /// The base branch, by name.
    pub base: String,
    /// The base's commit that was merged in.
    pub onto: String,
    /// The branch's head with the base in it, which is what the Checks read.
    pub head: String,
    /// Its tree, which is what is landable once they pass.
    pub tree: String,
    /// The branch's head before, which [`Delivery::put_back`] returns it to.
    pub was: String,
    /// Every path either side changed since the branch last held the base, for
    /// each Check's `when`.
    pub touched: Vec<String>,
}

/// Why the forge would not merge, in the kinds a person does something
/// different about.
///
/// **Not one shape with a sentence in it**, which is the arrangement
/// [`NotDelivered`] has and the reason it has it: there, no caller matches on
/// what went wrong. Here every kind sends a person somewhere else — an
/// administrator, the branch, the failing check — and a button that answered
/// all of them with one line would be a button that failed silently in five
/// different ways.
///
/// Every variant carries what the forge said, verbatim, because the kind is
/// what to do next and the sentence is why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotMerged {
    /// The base branch is protected, and merging it needs something this
    /// operator cannot supply from a command line — a review, an
    /// administrator, a rule.
    Protected { said: String },
    /// The branch and its base disagree. Nothing merges until somebody
    /// resolves it, and resolving one needs judgement about the code.
    Conflicted { said: String },
    /// A check the forge requires has not passed. **Not one of Armada's** —
    /// those already passed, or the Job would not be at a gate.
    ChecksNotPassed { said: String },
    /// It is closed, already gone, or there is no such pull request. The
    /// record names an address the forge will not act on.
    NotOpen { said: String },
    /// Nothing on this machine could ask: no tool, or nobody signed in.
    NoTool { said: String },
    /// The base moved past what the branch holds, so a merge of it would land
    /// a combination nothing gated. Only `merge_by: push` says this; the
    /// answer is to bring the branch up and gate it again.
    BaseMoved { said: String },
    /// The branch, with the moved base merged in, did not pass its Checks, so
    /// nothing was pushed. Only `merge_by: push` says this.
    GateFailed { said: String },
    /// The branch's head carries a tree its Checks never passed on, so nothing
    /// was pushed. Only `merge_by: push` says this; the answer is to run them.
    Unchecked { said: String },
    /// The forge refused and said something this vocabulary has no name for.
    ///
    /// **Never folded into the eight above.** A guess about which kind a
    /// sentence is would send a person to fix the wrong thing, and the honest
    /// answer is the sentence itself.
    Refused { said: String },
}

impl NotMerged {
    /// A sentence for a person, built beside the value rather than by whichever
    /// crate caught it — the shape [`NotDelivered::said`] takes.
    pub fn said(&self) -> String {
        let (why, said) = match self {
            NotMerged::Protected { said } => ("the base branch is protected", said),
            NotMerged::Conflicted { said } => ("the branch conflicts with its base", said),
            NotMerged::ChecksNotPassed { said } => {
                ("a check the forge requires has not passed", said)
            }
            NotMerged::NotOpen { said } => ("the pull request is not open", said),
            NotMerged::NoTool { said } => ("nothing on this machine could ask the forge", said),
            NotMerged::BaseMoved { said } => ("the base moved past what was gated", said),
            NotMerged::GateFailed { said } => (
                "the branch with the base merged in did not pass its Checks",
                said,
            ),
            NotMerged::Unchecked { said } => {
                ("the branch's head is not what its Checks passed on", said)
            }
            NotMerged::Refused { said } => ("the forge refused", said),
        };
        let mut out = String::from("the merge did not happen — ");
        out.push_str(why);
        out.push_str(": ");
        out.push_str(said);
        out
    }

    /// What the refusal carries, without the words [`said`](NotMerged::said)
    /// puts around it: the part worth keeping to rebuild it later.
    pub fn detail(&self) -> &str {
        match self {
            NotMerged::Protected { said }
            | NotMerged::Conflicted { said }
            | NotMerged::ChecksNotPassed { said }
            | NotMerged::NotOpen { said }
            | NotMerged::NoTool { said }
            | NotMerged::BaseMoved { said }
            | NotMerged::GateFailed { said }
            | NotMerged::Unchecked { said }
            | NotMerged::Refused { said } => said,
        }
    }

    /// The refusal a stored `kind` and sentence came from, for a caller that
    /// kept one and answers with it later. An unknown kind is
    /// [`NotMerged::Refused`], the kind that guesses nothing.
    pub fn from_kind(kind: &str, said: String) -> NotMerged {
        match kind {
            "branch_protected" => NotMerged::Protected { said },
            "conflicted" => NotMerged::Conflicted { said },
            "checks_not_passed" => NotMerged::ChecksNotPassed { said },
            "not_open" => NotMerged::NotOpen { said },
            "no_tool" => NotMerged::NoTool { said },
            "base_moved" => NotMerged::BaseMoved { said },
            "gate_failed" => NotMerged::GateFailed { said },
            "unchecked" => NotMerged::Unchecked { said },
            _ => NotMerged::Refused { said },
        }
    }

    /// The kind, as one word, for a wire code and a log field.
    ///
    /// **Spelled once**, so the code a client dispatches on and the field a
    /// person reads in the log cannot come to name the same refusal two ways.
    pub fn kind(&self) -> &'static str {
        match self {
            NotMerged::Protected { .. } => "branch_protected",
            NotMerged::Conflicted { .. } => "conflicted",
            NotMerged::ChecksNotPassed { .. } => "checks_not_passed",
            NotMerged::NotOpen { .. } => "not_open",
            NotMerged::NoTool { .. } => "no_tool",
            NotMerged::BaseMoved { .. } => "base_moved",
            NotMerged::GateFailed { .. } => "gate_failed",
            NotMerged::Unchecked { .. } => "unchecked",
            NotMerged::Refused { .. } => "refused",
        }
    }
}
