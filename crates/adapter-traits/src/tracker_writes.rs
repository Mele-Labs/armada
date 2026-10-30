//! Writing to an issue: the three writes a saved rule may plan, and the trait that makes them.
//!
//! **Its own trait, beside a read-only [`Tracker`](crate::Tracker), and answered only by
//! [`Trackers::writer`](crate::Trackers::writer).** A caller holding the reader cannot write, which
//! is what `the_tracker_trait_has_no_write_method` holds. The 17 Oct rule that Fleet writes nothing
//! on the owner's behalf stands for anything unasked: the one door here is a rule he saved, done at
//! once where he set it to act and on his press where he set it to ask.
//!
//! **What a tracker can do is asked before anything is planned**, with [`TrackerWrites::can`], which
//! runs nothing: an act the tracker cannot make is never put in front of the owner to press.

use alloc::format;
use alloc::string::String;

use crate::NotTracked;

/// One write to one issue, as a tracker is asked to make it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueWrite<'a> {
    /// Put it in the status the tracker calls `status`. **The tracker's own word**, which the
    /// Workspace's ticket lifecycle gives to the state being moved to.
    Move { status: &'a str },
    /// Give it to `to`, by the tracker's own name for them.
    Assign { to: &'a str },
    /// Close it. `status` is the word the Workspace gives to closed, where it gives one: a tracker
    /// that closes by moving to a status needs it, and one that closes outright does not.
    Close { status: Option<&'a str> },
}

impl IssueWrite<'_> {
    /// What is being done, as a noun for [`NotTracked::doing`].
    pub fn doing(&self) -> &'static str {
        match self {
            IssueWrite::Move { .. } => "moving the issue",
            IssueWrite::Assign { .. } => "assigning the issue",
            IssueWrite::Close { .. } => "closing the issue",
        }
    }

    /// The verb of the sentence that says a tracker cannot: `move`, `assign`, `close`.
    pub fn verb(&self) -> &'static str {
        match self {
            IssueWrite::Move { .. } => "move",
            IssueWrite::Assign { .. } => "assign",
            IssueWrite::Close { .. } => "close",
        }
    }
}

/// Writing to the issues of one repository's tracker.
///
/// **Every method answers "this tracker cannot" by default**, so a tracker that has no call for a
/// write says so and compiles. Every method is the operator's credentials, as with
/// [`Delivery`](crate::Delivery).
pub trait TrackerWrites {
    /// Whether this tracker can make `write`, and in a sentence why not where it cannot. **Pure**:
    /// it runs no process and reads no network, so planning can ask it for every act. An answer of
    /// `Ok` is one [`write`](TrackerWrites::write) does not then refuse as unable.
    fn can(&self, write: &IssueWrite<'_>) -> Result<(), String> {
        Err(not_able(write))
    }

    /// Make `write` to the issue `reference` names, as [`Tracker::read_issue`](crate::Tracker::read_issue)
    /// writes it, in the repository checked out at `in_repo`. **Idempotent where the tracker is**:
    /// a move to the status it is in, or a close of a closed issue, is not an error here.
    fn write(&self, in_repo: &str, reference: &str, write: &IssueWrite<'_>) -> Result<(), NotTracked> {
        let _ = (in_repo, reference);
        Err(NotTracked::of(write.doing(), not_able(write)))
    }
}

/// The sentence for a write a tracker has no call for.
pub fn not_able(write: &IssueWrite<'_>) -> String {
    format!("this tracker cannot {} an issue", write.verb())
}

/// What answers for a tracker nothing here writes to: the default of
/// [`Trackers::writer`](crate::Trackers::writer).
pub struct ReadOnly;

impl TrackerWrites for ReadOnly {}
