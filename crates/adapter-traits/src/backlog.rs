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
}
