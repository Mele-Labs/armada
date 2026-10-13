//! A Helm conversation's socket: a channel per conversation, the relay, the
//! routes that speak into it, and the record it is reported with. `#939`,
//! `#1367`.
//!
//! **`crate::observing`'s shape, one subject over**, and simpler for it: a
//! conversation outlives every process that answers in it, so its channel is
//! held for as long as Fleet runs and there is no hand-over between writers.
//! Drop-oldest, and a viewer that falls behind is told how many it lost.
//!
//! **A viewer that goes away is noticed while nothing is being said.** A
//! conversation can sit idle for days, so the relay reads the socket as well
//! as the channel; the Job socket only finds out on its next send.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{
    AskCommander, AskHelm, AskingToRun, CommanderMessage, CommanderOpened, HelmMessage,
    HelmOpened, ManifestId, Missed, ProtocolId, ReadThought, ThoughtSent,
};
use tokio::sync::broadcast;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Conversations;
use crate::scoped::InManifest;
use crate::served::Served;

/// How many messages a conversation's channel holds for a viewer not keeping
/// up — `crate::observing::WATCHING`'s number, for the same producer.
pub const HELM_BACKLOG: usize = 1024;

/// What a conversation's socket carries: a message that can say a viewer fell
/// behind, and which one is the last on a connection.
pub trait Spoken: Clone + serde::Serialize + Send + 'static {
    /// The message a viewer is sent where the bound dropped `dropped` before it
    /// read them.
    fn missed(dropped: u64) -> Self;

    /// Whether the connection ends after this one.
    fn ends_it(&self) -> bool;
}

impl Spoken for HelmMessage {
    fn missed(dropped: u64) -> HelmMessage {
        HelmMessage::Missed(Missed { dropped })
    }

    fn ends_it(&self) -> bool {
        matches!(self, HelmMessage::Closed(_))
    }
}

impl Spoken for CommanderMessage {
    fn missed(dropped: u64) -> CommanderMessage {
        CommanderMessage::Missed(Missed { dropped })
    }

    fn ends_it(&self) -> bool {
        false
    }
}

/// One conversation's messages going out. Cheap to clone; every clone offers
/// into the same channel. **Generic over the message** so a Job's Commander,
/// which is a conversation on Helm's host, is carried by the same channel and
/// the same relay and not a copy of them.
#[derive(Clone)]
pub struct ConversationFeed<M> {
    messages: broadcast::Sender<M>,
}

/// A repository's Helm conversation's feed.
pub type HelmFeed = ConversationFeed<HelmMessage>;

/// A Job's Commander's feed.
pub type CommanderFeed = ConversationFeed<CommanderMessage>;

impl<M: Clone> ConversationFeed<M> {
    pub fn new() -> ConversationFeed<M> {
        ConversationFeed {
            messages: broadcast::channel(HELM_BACKLOG).0,
        }
    }

    /// Offer a message. **Never blocks and never fails**: with nobody watching
    /// it is dropped, and the thread's record still holds it.
    pub fn offer(&self, message: M) {
        let _ = self.messages.send(message);
    }

    /// Listen. **Open this before reading the thread**, for `observing`'s
    /// reason.
    pub fn watch(&self) -> ConversationWatch<M> {
        ConversationWatch {
            inbound: self.messages.subscribe(),
        }
    }
}

impl<M: Clone> Default for ConversationFeed<M> {
    fn default() -> ConversationFeed<M> {
        ConversationFeed::new()
    }
}

/// One viewer's end of a conversation's channel.
pub struct ConversationWatch<M> {
    inbound: broadcast::Receiver<M>,
}

pub type HelmWatch = ConversationWatch<HelmMessage>;
pub type CommanderWatch = ConversationWatch<CommanderMessage>;

/// What the channel has for the socket next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationSeen<M> {
    Message(M),
    /// The bound dropped this many messages before this viewer read them.
    Missed(u64),
}

pub type HelmSeen = ConversationSeen<HelmMessage>;

impl<M: Clone> ConversationWatch<M> {
    /// The next message, or `None` once every feed is gone.
    pub async fn next(&mut self) -> Option<ConversationSeen<M>> {
        match self.inbound.recv().await {
            Ok(message) => Some(ConversationSeen::Message(message)),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Some(ConversationSeen::Missed(dropped)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

/// What one viewer is answered with, **assembled by the daemon** in the order
/// that loses nothing: the subscription, then the thread.
pub struct ObservedHelm {
    pub manifest_id: ManifestId,
    pub replying: bool,
    pub live: HelmWatch,
    /// The thread so far, oldest first.
    pub history: Vec<HelmMessage>,
    /// Older messages the thread left out.
    pub skipped: u64,
}

/// What one viewer of a Job's Commander is answered with, assembled as
/// [`ObservedHelm`] is.
pub struct ObservedCommander {
    pub job_id: ipc::JobId,
    pub replying: bool,
    pub live: CommanderWatch,
    pub history: Vec<CommanderMessage>,
    pub skipped: u64,
}

/// The thread, then everything said after, until the viewer goes, the thread is
/// started fresh, or the channel is gone.
pub(crate) async fn relay(socket: WebSocket, observed: ObservedHelm) {
    let ObservedHelm {
        manifest_id,
        replying,
        live,
        history,
        skipped,
    } = observed;
    let opened = HelmMessage::Opened(HelmOpened {
        protocol_id: ProtocolId::current(),
        manifest_id,
        replying,
        skipped,
    });
    relay_thread(socket, opened, history, live).await;
}

/// A Job's Commander's thread, on the relay Helm's is.
pub(crate) async fn relay_commander(socket: WebSocket, observed: ObservedCommander) {
    let ObservedCommander {
        job_id,
        replying,
        live,
        history,
        skipped,
    } = observed;
    let opened = CommanderMessage::Opened(CommanderOpened {
        protocol_id: ProtocolId::current(),
        job_id,
        replying,
        skipped,
    });
    relay_thread(socket, opened, history, live).await;
}

/// `opened`, the thread, then every message after it. **One relay for every
/// conversation**, so what a viewer is owed (the order, a `missed` for what it
/// lost, noticing it left while nothing is said) is decided once.
async fn relay_thread<M: Spoken>(
    mut socket: WebSocket,
    opened: M,
    history: Vec<M>,
    mut live: ConversationWatch<M>,
) {
    if !send(&mut socket, &opened).await {
        return;
    }
    for message in history {
        if !send(&mut socket, &message).await {
            return;
        }
    }
    loop {
        // Decided before anything is sent, so the read of the socket is dropped
        // before the send borrows it.
        let seen = tokio::select! {
            seen = live.next() => seen,
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => continue,
            },
        };
        let Some(seen) = seen else {
            return;
        };
        let (message, last) = match seen {
            ConversationSeen::Message(message) => {
                let last = message.ends_it();
                (message, last)
            }
            ConversationSeen::Missed(dropped) => (M::missed(dropped), false),
        };
        if !send(&mut socket, &message).await || last {
            return;
        }
    }
}

async fn send<M: serde::Serialize>(socket: &mut WebSocket, message: &M) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    socket.send(Message::Text(text)).await.is_ok()
}

/// One repository's conversation. **Asked before the upgrade**, so a Manifest
/// Fleet does not serve is refused at the moment it was asked.
pub(crate) async fn observe_helm<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_helm(scope.manifest()).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// The session as one quotable record. `#1367`.
pub(crate) async fn get_helm_debug_info<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_helm_debug_info(scope.manifest()).await {
        Ok(info) => answer(StatusCode::OK, &info, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Take a message. **202**: it is accepted and the reply is not here yet.
pub(crate) async fn ask_helm<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: AskHelm = match ipc::decode("a message to Helm", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_helm(asked, scope.manifest()).await {
        Ok(conversation) => answer(StatusCode::ACCEPTED, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Read words for where they belong. **Always 200**: a reading that could not be made is the
/// answer `question`, so the words go to Helm and are never lost.
pub(crate) async fn read_thought<D: Conversations>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let read: ReadThought = match ipc::decode("words to read", &body) {
        Ok(read) => read,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let reading = served.daemon().read_thought(read).await;
    answer(StatusCode::OK, &reading, served.run_id())
}

/// Say that words were delivered to a Session or a Job, so the thread keeps where they went.
pub(crate) async fn record_thought_sent<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let sent: ThoughtSent = match ipc::decode("where words went", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().record_thought_sent(sent, scope.manifest()).await {
        Ok(conversation) => answer(StatusCode::OK, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Put one call to the person, and **hold the response open until they answer**.
///
/// The one route on this surface that waits on a person. It is reached from the
/// agent door and from nowhere else: the caller is the CLI's own permission
/// prompt, not a model. `#1389`.
pub(crate) async fn ask_the_person<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    // **Placed by the connection**, as the door places a Helm session: a hosted
    // session's ask goes to its own thread and not to Helm's dock.
    let caller = crate::acting::asking().unwrap_or_else(crate::Caller::unplaceable);
    let asking: AskingToRun = match ipc::decode("a permission question", &body) {
        Ok(asking) => asking,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .ask_the_person_at(asking, scope.manifest(), caller)
        .await
    {
        Ok(decided) => answer(StatusCode::OK, &decided, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What is waiting on a person right now, Fleet-wide.
pub(crate) async fn list_helm_calls<D: Conversations>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_helm_calls().await {
        Ok(waiting) => answer(StatusCode::OK, &waiting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A person's answer, into the call that is waiting on it.
pub(crate) async fn answer_helm_call<D: Conversations>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let said: ipc::AnswerHelmCall = match ipc::decode("an answer to a Helm call", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_helm_call(said).await {
        Ok(waiting) => answer(StatusCode::OK, &waiting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Forget the stored session and the thread.
pub(crate) async fn start_helm_fresh<D: Conversations>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().start_helm_fresh(scope.manifest()).await {
        Ok(conversation) => answer(StatusCode::OK, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job's Commander. **Asked before the upgrade**, so a Job that is not
/// Fleet's is refused at the moment it was asked.
pub(crate) async fn observe_commander<D: Conversations>(
    State(served): State<Served<D>>,
    job: crate::reference::Resolved,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_commander(job).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| relay_commander(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// Take the owner's words for a Job's Commander. **202**: they are accepted and
/// the turn is not written yet.
pub(crate) async fn ask_commander<D: Conversations>(
    State(served): State<Served<D>>,
    job: crate::reference::Resolved,
    body: Bytes,
) -> Response {
    let asked: AskCommander = match ipc::decode("a message to a Commander", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().ask_commander(job, asked).await {
        Ok(conversation) => answer(StatusCode::ACCEPTED, &conversation, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
