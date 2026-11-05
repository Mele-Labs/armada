//! What a client does with the calls a Job puts to a person, and with words typed for a running
//! Job. `#939`.
//!
//! **A Job on your words is a hosted Session**, so reading its thread, tuning it and fetching what
//! a message carried are the Sessions' own operations; what is here is the held calls and the two
//! acts on words.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AnswerCall, AskingToRun, CallsWaiting, ManifestId, ReadThought, RunOrNot, TellSession,
    ThoughtReading, ThoughtTarget,
};

use crate::daemon::Refusal;

/// Everything a client does with a held call and with words for a Job.
pub trait Conversations: Send + Sync + 'static {
    /// `read_thought` — words a person typed, read for which running Job they are for.
    /// **Total**: a reading that cannot be made is [`ThoughtReading::Question`], so the words
    /// are never taken for a thought about a Job that is not there.
    fn read_thought(&self, read: ReadThought) -> impl Future<Output = ThoughtReading> + Send;

    /// `tell_session` — a Job on your words passes words to a live Session. **Sent as
    /// `send_session_message` sends**, so a refusal is that send's, and on success
    /// `operator.words_sent` is published. Answers where the words went. A refused send publishes
    /// nothing.
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
    /// ask is its own**; a daemon with no such
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

    /// `list_calls` — every ask waiting on this person right now, across
    /// every repository. Fleet-wide, because what is waiting on a person is waiting on them wherever it came from.
    fn list_calls(&self) -> impl Future<Output = Result<CallsWaiting, Refusal>> + Send;

    /// `answer_call` — a person's answer, delivered inside the call that
    /// is waiting on it. [`Refusal::IllegalMove`] where nothing is waiting
    /// under that id, or where the answer is not one that call offered.
    fn answer_call(
        &self,
        said: AnswerCall,
    ) -> impl Future<Output = Result<CallsWaiting, Refusal>> + Send;
}
