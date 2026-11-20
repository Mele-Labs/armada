//! The forge telling Fleet that a pull request changed, so Fleet reads that one again at once
//! instead of at the next sweep.
//!
//! **An event carries no state.** It says only that something about the pull request moved,
//! so the whole of what Fleet takes from one is "read this pull request again". Events may repeat
//! and arrive out of order, and none is replayed after a disconnect: Fleet reads once whenever a
//! stream attaches.

use alloc::string::String;
use core::time::Duration;

/// One pull request's subscription, as the forge made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullSubscription {
    pub id: String,
    /// Where its stream is read, as the forge named it.
    pub events_url: String,
    /// How long the forge will still keep it, which is less than a day for one made earlier.
    pub left: Duration,
}

/// Why the forge did not do what was asked of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullEventsFailure {
    /// The account has events switched off. Nothing else will work until that changes.
    NotEnabled,
    /// Refused, or no such thing: asking again changes nothing.
    Forbidden,
    /// The token was refused, after being read again once.
    Unauthorized,
    /// The token already holds as many subscriptions as the forge allows.
    LimitReached,
    /// The forge named a wait.
    RetryAfter(Duration),
    /// A failure that may pass: no network, a timeout, a 5xx.
    Transient,
    /// The checkout's repository is not on this forge.
    NotOnThisForge,
    Other(String),
}

/// What a stream says while it is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStreamSignal {
    /// The stream is open. Nothing is replayed, so this is when to read the pull request.
    Attached,
    /// Something about the pull request changed.
    Changed,
    /// Bytes arrived, a heartbeat among them: the reader's chance to say it has stopped listening.
    Beat,
}

/// What the reader of a stream answers to each signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStreamAction {
    Keep,
    Stop,
}

/// How a stream ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullStreamEnd {
    /// The forge said the subscription is over: it expired or was deleted.
    SubscriptionEnded,
    /// The forge asked for the same address to be opened again.
    Reconnect,
    /// The stream ended without saying so: a dropped connection.
    Dropped,
    /// The reader answered [`PullStreamAction::Stop`].
    Stopped,
    /// It would not open.
    Failed(PullEventsFailure),
}

/// The forge's pull request events. Every call blocks, so a caller on a runtime makes it from a
/// thread that may.
pub trait PullEvents: Send + Sync {
    /// Subscribe to one pull request of the repository `in_repo` is a checkout of. **Asking for a
    /// pull request already subscribed to returns the one that exists.**
    fn subscribe(&self, in_repo: &str, number: u64) -> Result<PullSubscription, PullEventsFailure>;

    /// End a subscription. One already gone is ended.
    fn unsubscribe(&self, subscription: &str) -> Result<(), PullEventsFailure>;

    /// Whether the account may subscribe at all, without making one.
    fn probe(&self) -> Result<(), PullEventsFailure>;

    /// Read a subscription's stream until it ends, telling `signal` of each thing it says.
    fn listen(
        &self,
        events_url: &str,
        signal: &mut dyn FnMut(PullStreamSignal) -> PullStreamAction,
    ) -> PullStreamEnd;
}
