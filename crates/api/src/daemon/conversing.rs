//! One repository's Helm: find its Session, speak into it, start it over. `#939`.
//!
//! **Helm is a hosted Session**, so reading its thread, tuning it and fetching what a message
//! carried are the Sessions' own operations; what is here is what only Helm has: the lookup of
//! the Session and the message with the person's whereabouts.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AnswerHelmCall, AskHelm, AskingToRun, EventsSince, HelmCallsWaiting, HelmDebugInfo,
    HelmSession, ManifestId, ReadThought, RunOrNot, SessionRecord, TellSession, ThoughtReading,
    ThoughtSent, ThoughtTarget,
};

use crate::daemon::Refusal;

/// Everything a client does with a Helm conversation.
pub trait Conversations: Send + Sync + 'static {
    /// `get_helm_session` — the repository's live Helm Session and its thread, or none.
    /// `manifest_id` names the repository; absent is the one Fleet started in.
    /// [`Refusal::Unacceptable`] where Fleet serves no such Manifest.
    fn get_helm_session(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmSession, Refusal>> + Send;

    /// `ask_helm` — a message to the repository's Helm Session, **started by this one if none is
    /// live**. Sent as `send_session_message` sends, with a line of the person's whereabouts ahead
    /// of the words in the turn. The reply is the Session's thread, on `session.row`. Answers the
    /// Session's record.
    fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `get_helm_debug_info` — the session as one quotable record, taken at
    /// this moment. `#1367`. [`Refusal::Unacceptable`] where Fleet serves no
    /// such Manifest, the same as `get_helm_session`.
    fn get_helm_debug_info(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmDebugInfo, Refusal>> + Send;

    /// What a Helm session's `get_events_since` was answered, kept so the
    /// record can report it. **Not an operation**, [`Queries::owned_jobs`](crate::daemon::Queries::owned_jobs)'s
    /// shape: the door has already answered the call by the time this is told.
    ///
    /// Told by the transport rather than read by the daemon, for
    /// `get_events_since`'s own reason — the positions are the broadcaster's.
    fn helm_polled(
        &self,
        manifest_id: ManifestId,
        counted: EventsSince,
    ) -> impl Future<Output = ()> + Send;

    /// `start_helm_fresh` — end the repository's live Helm Session, so the next message starts a
    /// new one. Answers with no session. Nothing happens where none is live.
    fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmSession, Refusal>> + Send;

    /// `read_thought` — words typed into Helm's box, read for where they belong.
    /// **Total**: a reading that cannot be made is [`ThoughtReading::Question`], so the words
    /// go to Helm and are never lost.
    fn read_thought(&self, read: ReadThought) -> impl Future<Output = ThoughtReading> + Send;

    /// `record_thought_sent` — the words were delivered to a Session or a Job: keep a `sent` row in
    /// the Helm Session's thread and publish `helm.thought_routed`. **Nothing is sent to Helm's
    /// process.** [`Refusal::Unacceptable`] where Fleet serves no such Manifest.
    fn record_thought_sent(
        &self,
        sent: ThoughtSent,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `tell_session` — Helm passes words to a live Session. **Sent as `send_session_message`
    /// sends**, so a refusal is that send's, and on success the same row and `helm.thought_routed`
    /// as [`record_thought_sent`](Conversations::record_thought_sent) are kept in the Helm Session's thread. Answers where the
    /// words went. A refused send records nothing.
    fn tell_session(
        self: Arc<Self>,
        tell: TellSession,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ThoughtTarget, Refusal>> + Send;

    /// `ask_the_person` — put one call to the person and **do not return until
    /// they have answered or the hold has run out**. `#1389`.
    ///
    /// The only method on this seam that waits on a person rather than on
    /// Fleet: the session's own process is inside the tool call for as long as
    /// this takes, which is what makes the answer land in the same turn.
    /// Silence answers [`RunOrNot::Deny`], never an allow.
    fn ask_the_person(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RunOrNot, Refusal>> + Send;

    /// `ask_the_person`, for the connection that made it. **A hosted session's
    /// ask is its own** and never reaches Helm's dock; a daemon with no such
    /// session answers as [`ask_the_person`](Conversations::ask_the_person)
    /// does.
    fn ask_the_person_at(
        self: Arc<Self>,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
        _caller: crate::Caller,
    ) -> impl Future<Output = Result<RunOrNot, Refusal>> + Send {
        async move { self.ask_the_person(asking, manifest_id).await }
    }

    /// `list_helm_calls` — every ask waiting on this person right now, across
    /// every repository. Fleet-wide, because the dock is.
    fn list_helm_calls(&self) -> impl Future<Output = Result<HelmCallsWaiting, Refusal>> + Send;

    /// `answer_helm_call` — a person's answer, delivered inside the call that
    /// is waiting on it. [`Refusal::IllegalMove`] where nothing is waiting
    /// under that id, or where the answer is not one that call offered.
    fn answer_helm_call(
        &self,
        said: AnswerHelmCall,
    ) -> impl Future<Output = Result<HelmCallsWaiting, Refusal>> + Send;
}
