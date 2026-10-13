//! One repository's Helm conversation: watch it, speak into it, start it over.
//! `#939`.
//!
//! **A fourth surface, not three methods spread over two of the others.** The
//! conversation is one thing that may move: `#73` keeps a host in Bridge a
//! switch rather than a rebuild, and a surface split across `Queries` and
//! `Commands` would make that switch in two places. What these three answer is
//! whole on its own, so another implementor can answer all of it.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AnswerCommanderCard, AnswerHelmCall, AskCommander, AskHelm, AskTheOwner, AskingToRun,
    CommanderCard, CommanderCardAnswered, CommanderConversation, EventsSince, HelmBoxFacts,
    HelmCallsWaiting, HelmConversation, HelmDebugInfo, ManifestId, ReadThought, RepairSent,
    RunOrNot, SendRepairDrone, TellSession, ThoughtReading, ThoughtSent, ThoughtTarget, TuneHelm,
};

use crate::conversing::{ObservedCommander, ObservedHelm};
use crate::daemon::{Refusal, StoredFile};
use crate::reference::Resolved;

/// Everything a client does with a Helm conversation.
pub trait Conversations: Send + Sync + 'static {
    /// `observe_helm` — the thread so far, then every message that follows.
    ///
    /// **It answers before the socket opens**, for `Queries::observe_job`'s
    /// reason, and the subscription is already open inside what comes back.
    /// `manifest_id` names the repository; absent is the one Fleet started in.
    /// [`Refusal::Unacceptable`] where Fleet serves no such Manifest.
    fn observe_helm(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ObservedHelm, Refusal>> + Send;

    /// `observe_commander` — one Job's Commander: the thread so far, then every
    /// message that follows. `observe_helm`'s shape with a Job for its subject,
    /// and answered before the socket opens for the same reason. #2166.
    fn observe_commander(
        &self,
        job: Resolved,
    ) -> impl Future<Output = Result<ObservedCommander, Refusal>> + Send;

    /// `ask_commander` — take the owner's words for a Job's Commander and answer
    /// at once. **The turn is the socket's.** Words that arrive while a turn is
    /// being written wait for the one after it, and never interrupt. #2166.
    /// [`Refusal::IllegalMove`] where the Job has no worktree to host it in.
    fn ask_commander(
        self: Arc<Self>,
        job: Resolved,
        asked: AskCommander,
    ) -> impl Future<Output = Result<CommanderConversation, Refusal>> + Send;

    /// `ask_the_owner` — put a card to the owner for a Job's Commander. **The
    /// card is kept and the call answers at once**: it waits until he answers,
    /// however long, and is not a held call. #2166.
    ///
    /// [`Refusal::IllegalMove`] where the Job already has a card out, naming
    /// its question, or where the Job has ended.
    fn ask_the_owner(
        self: Arc<Self>,
        job: Resolved,
        asked: AskTheOwner,
    ) -> impl Future<Output = Result<CommanderCard, Refusal>> + Send;

    /// `send_repair_drone` — a repair Drone onto the Job's own branch and
    /// worktree, for a Job's Commander. **The Drone is on its way and the call
    /// answers at once**; the stopped step is restarted when it hands in. #2170.
    ///
    /// [`Refusal::IllegalMove`] where the Job is not held for a person, a Drone
    /// is still on it, its worktree is gone, no step stopped, or a repair is
    /// already out.
    fn send_repair_drone(
        self: Arc<Self>,
        job: Resolved,
        sending: SendRepairDrone,
    ) -> impl Future<Output = Result<RepairSent, Refusal>> + Send;

    /// `answer_commander_card` — the owner's pick, recorded and handed to the
    /// Commander as the reason it is woken for. #2166.
    ///
    /// [`Refusal::IllegalMove`] where the card is not the one out, or did not
    /// offer the option.
    fn answer_commander_card(
        self: Arc<Self>,
        job: Resolved,
        answered: AnswerCommanderCard,
    ) -> impl Future<Output = Result<CommanderCardAnswered, Refusal>> + Send;

    /// `ask_helm` — take a message and answer at once. **The reply is the
    /// socket's**, so this returns before a session has started, and a message
    /// sent while a reply is being written waits its turn rather than refusing.
    fn ask_helm(
        self: Arc<Self>,
        asked: AskHelm,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;

    /// `tune_helm` — set the model and effort the next reply runs on, and tell every watcher.
    /// [`Refusal::Unacceptable`] for an effort that is not `low`, `medium` or `high`.
    fn tune_helm(
        &self,
        tuned: TuneHelm,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmBoxFacts, Refusal>> + Send;

    /// `get_helm_file` — what a message to Helm carried, by its `SentFile.id`.
    fn get_helm_file(
        &self,
        manifest_id: Option<ManifestId>,
        file: String,
    ) -> impl Future<Output = Result<StoredFile, Refusal>> + Send;

    /// `get_helm_debug_info` — the session as one quotable record, taken at
    /// this moment. `#1367`. [`Refusal::Unacceptable`] where Fleet serves no
    /// such Manifest, the same as `observe_helm`.
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

    /// `start_helm_fresh` — forget the stored session and the thread, so the
    /// next message starts over. [`Refusal::IllegalMove`] while a reply is being
    /// written: forgetting a session under a process still writing into it
    /// would leave its answer nowhere.
    fn start_helm_fresh(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;

    /// `read_thought` — words typed into Helm's box, read for where they belong.
    /// **Total**: a reading that cannot be made is [`ThoughtReading::Question`], so the words
    /// go to Helm and are never lost.
    fn read_thought(&self, read: ReadThought) -> impl Future<Output = ThoughtReading> + Send;

    /// `record_thought_sent` — the words were delivered to a Session or a Job: keep a row in
    /// the thread and publish `helm.thought_routed`. **Nothing is sent to Helm's session.**
    /// [`Refusal::Unacceptable`] where Fleet serves no such Manifest.
    fn record_thought_sent(
        &self,
        sent: ThoughtSent,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<HelmConversation, Refusal>> + Send;

    /// `tell_session` — Helm passes words to a live Session. **Sent as `send_session_message`
    /// sends**, so a refusal is that send's, and on success the same row and `helm.thought_routed`
    /// as [`record_thought_sent`](Conversations::record_thought_sent) are kept. Answers where the
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
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
        _caller: crate::Caller,
    ) -> impl Future<Output = Result<RunOrNot, Refusal>> + Send {
        self.ask_the_person(asking, manifest_id)
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
