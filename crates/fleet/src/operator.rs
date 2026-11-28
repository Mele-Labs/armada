//! The Operator: one long-running agent session on this machine, hosted by Fleet. `armada-ehhu`, and the
//! design page *Helm becomes the Operator*, revision 41.
//!
//! **Fleet detects, the session decides.** Fleet tells the session what arrives that no Job owns (`session`),
//! keeps its cards (`cards`) and its feed (`feed`), and builds the page that shows them (`paging`). It rules
//! on nothing itself: the one-shot Policy that ruled on each arrival is gone.
//!
//! **While the session is not running** an arrival goes as it did with the Operator off: a dispatch starts as
//! asked, a conflict or a landing question stands in the sending Job's own Needs you, and a report, a note, an
//! error, an issue or a red main reaches nobody and stays where it is.
//!
//! **Only what nobody owns reaches it**: a note, an error, a bug report, a new issue, a red main. A Job's own
//! events are never handed to it, and nothing a Job does waits on it. An end and a merge are heard once
//! (`hearing`) and go to the feed.

mod arriving;
mod briefing;
mod cards;
mod cataloging;
mod desk;
mod disking;
mod dispatching;
mod elsewhere;
mod erroring;
mod feed;
mod hearing;
mod journal;
mod orders;
mod paging;
mod placing;
mod raising;
mod reviewing;
mod seat;
mod session;
mod settings;
mod sources;
mod sourcing;
mod telling;
mod turning;
pub(crate) use turning::utc_millis;

use std::sync::{Mutex, MutexGuard, PoisonError};

use core_model::{Actor, Timestamp};

pub use arriving::{Arrival, Arrived, InLine, LiveJob, Source};
pub(crate) use elsewhere::plan_overlap;
#[cfg(test)]
pub(crate) use erroring::fingerprint;
pub use hearing::keep_the_ends_told;
pub use orders::{Order, Orders};
pub use placing::{Outcome, Place};
pub use raising::keep_the_raises;
pub use session::keep_the_operator_up;
pub(crate) use seat::Seat;
pub use sources::{BridgeError, NewIssue, Note, Report};
pub(crate) use sourcing::Sources;

/// One row of the Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logged {
    pub arrival: String,
    pub arrived: Arrived,
    pub source: Source,
    pub session_id: String,
    pub manifest_id: String,
    pub subject: String,
    pub outcome: Outcome,
    pub other: Option<String>,
    pub shared: Vec<String>,
    pub because: String,
    pub by: Actor,
    /// When the item was handed to the Operator.
    pub handed: Timestamp,
    pub at: Timestamp,
    /// Where a person's drag put a Job in Waiting to start.
    pub place: Option<Place>,
    pub issue: Option<String>,
}

impl Logged {
    /// The row as the store keeps it.
    pub fn kept(&self) -> store::KeptRuling {
        store::KeptRuling {
            arrival: self.arrival.clone(),
            arrived: self.arrived.word().to_string(),
            source: self.source.word().to_string(),
            source_session: self.source.session().map(str::to_string),
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.subject.clone(),
            outcome: self.outcome.word().to_string(),
            other_session: self.other.clone(),
            shared: self.shared.clone(),
            because: self.because.clone(),
            call: None,
            by: self.by.as_wire().to_string(),
            handed: self.handed.as_str().to_string(),
            at: self.at.as_str().to_string(),
            place: self.place.as_ref().map(Place::word),
            first_session: None,
            refused: Vec::new(),
            answer: None,
            issue: self.issue.clone(),
        }
    }
}

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}
