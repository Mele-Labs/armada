//! What stands between a checkout and its base, the review a pull request carries, and the refusal every call shares.

use super::*;

/// What came of bringing a pull request's branch up to a base that moved,
/// **in place of** closing and reopening it — `#663`.
///
/// **Every conflict leaves the branch exactly as it was.** Nothing is reading
/// the result the way a Drone reads [`BroughtUpToDate::Conflicted`]: the Job
/// is already done, so a conflict here is answered by restoring the branch
/// untouched and telling a person, never by leaving markers in a checkout
/// nobody is about to open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptCurrent {
    /// The branch was rebased onto `onto` — the base's tip at the moment of
    /// the attempt — and pushed with `--force-with-lease`.
    Rebased { onto: String, commits: usize },
    /// The rebase conflicted, so the branch was left exactly as it was.
    /// `onto` is the base's tip this was attempted against, carried so a
    /// caller can tell "already tried this base and it conflicted" from "the
    /// base moved again" without retrying on every sweep.
    Conflicted { onto: String, files: Vec<String> },
    /// The branch this pull request derived is gone — nothing to rebase, and
    /// nothing this call can do about it.
    NoBranch,
    /// A tool would not run.
    NotDelivered(NotDelivered),
}

/// Where the repository every worktree is cut from stands, after a merge was
/// noticed.
///
/// **Fast-forward or nothing.** This runs in a repository a person is working
/// in, so every reading that is not "clean, on the branch that merged, and one
/// fast-forward behind it" is [`LeftAlone`](RepositoryStanding::LeftAlone) with
/// the reason said out loud.
///
/// **Two of the three name a commit and the third cannot**, which is what stops
/// `#474` proving a tree nobody committed. See
/// [`caught_up_to`](RepositoryStanding::caught_up_to).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepositoryStanding {
    /// It moved, by how much, and onto what. `head` is the commit the base is
    /// on **now** and not the merge commit the forge named, so a merge that
    /// arrived behind two others is proved once, at the tip.
    MovedOn {
        base: String,
        commits: usize,
        head: String,
    },
    /// It already had what merged, and what it is on. Ordinary, and the answer
    /// whenever a person pulled before Armada got to it — which is not a person
    /// having run anything, so it still names a commit.
    AlreadyHadIt { base: String, head: String },
    /// Nothing was touched, and why — a person's uncommitted work, a checkout
    /// that is not on the base, no remote, or a history that would not
    /// fast-forward. **No commit, and that is the point.**
    LeftAlone { why: String },
}

impl RepositoryStanding {
    /// The commit a run may be made against, where this standing left one.
    ///
    /// **`None` is the whole of the refusal**, and it is a shape rather than a
    /// rule: there is no other way to reach a run, so declining on a dirty
    /// worktree is not a check somebody remembered to write.
    pub fn caught_up_to(&self) -> Option<&str> {
        match self {
            RepositoryStanding::MovedOn { head, .. }
            | RepositoryStanding::AlreadyHadIt { head, .. } => Some(head),
            RepositoryStanding::LeftAlone { .. } => None,
        }
    }
}

/// A pull request's contents, assembled before anything is opened.
///
/// **Two owned strings and no builder.** What goes in them is assembled by the
/// caller from its own record; this type exists so that what reaches the forge
/// is a value that was built somewhere a test can read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    title: String,
    body: String,
    draft: bool,
}

impl Review {
    /// Offered for review, which is what every pull request was before a
    /// person could choose a draft.
    pub fn assembled(title: impl Into<String>, body: impl Into<String>) -> Review {
        Review {
            title: title.into(),
            body: body.into(),
            draft: false,
        }
    }

    /// The same contents, opened as a draft. Spike 022, slice 4: a person
    /// chose it at the Job's approval.
    pub fn as_draft(self) -> Review {
        Review {
            draft: true,
            ..self
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn draft(&self) -> bool {
        self.draft
    }
}

/// Why a step of delivery did not happen.
///
/// **One shape rather than an associated type.** The outcomes a caller acts on
/// differently — no remote, a conflict, no tool — are variants of the values
/// above, so what is left over is a command that would not run and the sentence
/// it gave, which no caller matches on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotDelivered {
    /// What was being attempted, as a noun a sentence can be built around.
    pub doing: &'static str,
    /// What the tool said, verbatim.
    pub said: String,
}

impl NotDelivered {
    pub fn of(doing: &'static str, said: impl Into<String>) -> NotDelivered {
        NotDelivered {
            doing,
            said: said.into(),
        }
    }

    /// A sentence for a person, built beside the value rather than by whichever
    /// crate caught it — the shape
    /// [`WorktreeSpecRefused`](crate::WorktreeSpecRefused) takes.
    pub fn said(&self) -> String {
        let mut out = String::from(self.doing);
        out.push_str(" did not happen: ");
        out.push_str(&self.said);
        out
    }
}

/// A checkout Fleet would not catch up, and what stands in the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldBack {
    pub base: String,
    /// The branch checked out, where it is not the base. `HEAD` when detached.
    /// The counts below are then the local base branch's.
    pub on: Option<String>,
    /// Tracked files with uncommitted changes, while the base is checked out.
    /// Untracked files never block a fast-forward and are not named.
    pub uncommitted: Vec<String>,
    /// Those of `uncommitted` that the remote base also changed **to something
    /// else**, so an autostash could stop on a conflict. A file whose content
    /// is byte for byte the remote base's is in `already_on_base` instead.
    pub clashing: Vec<String>,
    /// Those of `uncommitted` that the remote base also changed and whose
    /// content here is exactly what it now holds: the change went up some other
    /// way, and a pull over them takes nothing from anyone.
    pub already_on_base: Vec<String>,
    /// Commits the remote base has that the checkout does not.
    pub behind: usize,
    /// Commits the checkout has that the remote base does not.
    pub ahead: usize,
}

/// What pulling a held checkout up came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutPulled {
    /// git's exit code. `1` where git exited clean and left the stashed work
    /// conflicted, and `-1` where git did not run or was stopped by a signal.
    pub code: i32,
    /// What git printed, both streams, for a person to read.
    pub output: String,
}

/// A line for a person about where the base came from. Built here so the two
/// callers that report it cannot word it two ways.
pub fn how_the_base_was_found(base: &Base) -> String {
    match base.was_declared() {
        true => "`base:` in armada.yml".to_string(),
        false => "inferred — nothing declares one".to_string(),
    }
}

/// Failed CI runs started again. #905.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rerun {
    /// How many runs the forge was asked to start again.
    pub runs: usize,
}

/// Why failed CI runs were not started again, in the words the forge or the tool used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRerun {
    pub said: String,
}

/// An issue the forge filed. #906.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiledIssue {
    /// Where the issue is, as the forge printed it.
    pub url: String,
}

/// Why no issue was filed, in the words the forge or the tool used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotFiled {
    pub said: String,
}
