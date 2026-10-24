//! The one tracker Fleet writes to: a backlog with a copy on every machine, which meet only when
//! something syncs them. `docs/concepts/tracker.md`, *Beads*.
//!
//! **Its own trait, beside a read-only [`Tracker`](crate::Tracker).** Reading stays the
//! tracker's, so a forge or Jira is still never written to; only a repository whose tracker
//! answers [`Trackers::backlog`](crate::Trackers::backlog) is.

use alloc::string::String;

use crate::NotTracked;

/// What this machine's copy was last written to. **Moves on every write, a sync's included, and on
/// no read**, so a mark that differs from the one held after a sync means something wrote since.
/// Compared, never read into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyMark(String);

impl CopyMark {
    pub fn of(written: impl Into<String>) -> CopyMark {
        CopyMark(written.into())
    }
}

/// Where one issue of a backlog stands. A task is `Open` until it is started, and `Closed` once it is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeadState {
    Open,
    InProgress,
    Closed,
}

impl BeadState {
    /// The word the ledger keeps it as.
    pub fn word(self) -> &'static str {
        match self {
            BeadState::Open => "open",
            BeadState::InProgress => "in_progress",
            BeadState::Closed => "closed",
        }
    }

    pub fn read(word: &str) -> Option<BeadState> {
        [BeadState::Open, BeadState::InProgress, BeadState::Closed].into_iter().find(|one| one.word() == word)
    }
}

/// Writing to a backlog and keeping this machine's copy in step with the others'.
///
/// **A write lands in this machine's copy only.** Another machine sees it after this one syncs and
/// then it does, and two writes to one field settle on a sync with the later one winning on both
/// copies, so nothing a write answers is settled until a sync and a read back say so.
pub trait Backlog {
    /// Pull the other machines' writes into this copy, merge, and push. Runs for seconds.
    fn sync(&self, in_repo: &str) -> Result<(), NotTracked>;

    /// This copy's [`CopyMark`]. `None` where there is no copy to read, and then nothing but the
    /// timer syncs it.
    fn mark(&self, in_repo: &str) -> Option<CopyMark>;

    /// Assign `reference` to `actor` and start it, in this copy. **Refused where this copy says
    /// someone else holds it**, and not settled where it does not: see the trait.
    fn claim(&self, in_repo: &str, reference: &str, actor: &str) -> Result<(), NotTracked>;

    /// File `title` as a child of `parent`, and its reference. **Not idempotent**: a second call
    /// files a second issue, so a caller keeps what it was answered before it asks again.
    fn create_child(&self, in_repo: &str, parent: &str, title: &str) -> Result<String, NotTracked>;

    /// Give `reference` a new title.
    fn retitle(&self, in_repo: &str, reference: &str, title: &str) -> Result<(), NotTracked>;

    /// Put `reference` in `state`. `reason` is what a close says, and is ignored otherwise. **Closing
    /// is not refused for a blocker or an open child**: the caller decides when a thing is done.
    fn put_state(&self, in_repo: &str, reference: &str, state: BeadState, reason: &str) -> Result<(), NotTracked>;

    /// `blocked` waits on `blocker`. Already so is not an error.
    fn block(&self, in_repo: &str, blocked: &str, blocker: &str) -> Result<(), NotTracked>;

    /// `blocked` no longer waits on `blocker`. Not waiting already is not an error.
    fn unblock(&self, in_repo: &str, blocked: &str, blocker: &str) -> Result<(), NotTracked>;

    /// Close `reference` and every child under it, at any depth, that is not closed yet, each with
    /// `reason`. One already closed keeps the reason it was closed with.
    fn close_tree(&self, in_repo: &str, reference: &str, reason: &str) -> Result<(), NotTracked>;
}
