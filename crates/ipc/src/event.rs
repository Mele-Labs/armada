//! What travels the socket: the resync, the events, and the admission that
//! events were dropped.
//!
//! **`docs/practices/protocol.md`, *The first socket*, owns the decisions here**
//! — why a reconnection resyncs rather than replays, what a resync can and
//! cannot rebuild, why the stream is global with no subscription, and why an
//! unproduced kind is not stubbed.
//!
//! One `#[serde(tag = "kind")]` enum is the closed set a rule compares against
//! `operations/` row by row, so a split would assemble that roster from two
//! places.

use serde::{Deserialize, Serialize};

use crate::act_failed::ActFailed;
use crate::helm_call::{HelmAskingToRun, HelmCallAnswered};
use crate::thought::HelmThoughtRouted;
use crate::hosted_sessions::SessionRowChanged;
use crate::merge_line::MergeLines;
use crate::mods::ModList;
use crate::protocol_id::ProtocolId;
use crate::reading::ManifestReading;
use crate::review_runs::ReviewRun;
use crate::rehearsal::CheckoutRunRecord;
use crate::repositories::RepositoryList;
use crate::app_runs::AppRun;
use crate::servers::ServerState;
use crate::settings::SettingsList;
use crate::model_downshift::ModelDownshiftState;
use crate::agent_sign_in::AgentSignIn;
use crate::plan_usage::PlanUsage;
use crate::sleep::SleepState;
use crate::timeline::TimelineAppended;
use crate::autopilot::Autopilot;
use crate::watches::WatchList;
use crate::sessions::SessionRecord;
use crate::studio::{Studio, StudioDeleted, StudioHelmActed};
use crate::terminals::JobTerminal;

/// A position in the stream. Monotonic, assigned by Fleet, never reused.
///
/// It orders messages and lets a client see a gap for itself. It is **not** a
/// resume token: a reconnection resyncs to current state, and nothing accepts a
/// cursor back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cursor(u64);

impl Cursor {
    pub fn at(position: u64) -> Self {
        Cursor(position)
    }
    pub fn position(&self) -> u64 {
        self.0
    }
}

/// One message from Fleet to a connected client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum StreamMessage {
    /// The first message on every connection, and the message after a drop.
    Resync(Resync),
    Event(Delivered),
    /// The bound was reached and the oldest were dropped. **Always followed by
    /// a resync**, because a client that knows only the count cannot repair
    /// what it holds — and a client that believes its history is complete when
    /// it is not renders a Board that is quietly wrong.
    Missed(Missed),
}

/// Current state, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resync {
    /// Restated on the stream so a client that reached the socket without
    /// reading the runtime file still learns what it is talking to.
    pub protocol_id: ProtocolId,
    /// The position this state is current as of. The next `Event` carries a
    /// later one.
    pub cursor: Cursor,
}

/// One event, at its position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Delivered {
    pub cursor: Cursor,
    pub event: Event,
}

/// How many events the client will never see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Missed {
    pub dropped: u64,
}

/// An unsolicited push. The `kind` is the dotted name `operations/` keys it
/// under, so a rule can compare the two without a mapping in between.
///
/// Not `Eq`: `act.failed` carries a `WireError`, whose field values include a float.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "manifest.reread")]
    ManifestReread(ManifestReading),
    // A person's run in the main checkout ended.
    #[serde(rename = "checkout_run.finished")]
    CheckoutRunFinished(CheckoutRunRecord),
    // A server's three lifecycle facts, each carrying the instance whole so a
    // row is replaced rather than patched. Its output is `observe_server`'s.
    #[serde(rename = "server.starting")]
    ServerStarting(ServerState),
    #[serde(rename = "server.serving")]
    ServerServing(ServerState),
    #[serde(rename = "server.exited")]
    ServerExited(ServerState),
    // A run after any of its servers' three facts, whole, so a row is replaced rather than patched.
    #[serde(rename = "app_run.changed")]
    AppRunChanged(AppRun),
    // The list `list_repositories` answers, whole, whenever it changes.
    #[serde(rename = "repositories.changed")]
    RepositoriesChanged(RepositoryList),
    // What `get_merge_lines` answers, whole, whenever a line moved on disk.
    #[serde(rename = "merge_lines.changed")]
    MergeLinesChanged(MergeLines),
    // The mods on this machine, whole, whenever one was added, changed or switched.
    #[serde(rename = "mods.changed")]
    ModsChanged(ModList),
    // settings.json, whole, after a save or a hand edit — or a refusal of one.
    #[serde(rename = "settings.changed")]
    SettingsChanged(SettingsList),
    // A Studio after any write to it, whole, so an open whiteboard replaces what it holds. `#1285`.
    #[serde(rename = "studio.changed")]
    StudioChanged(Studio),
    #[serde(rename = "studio.deleted")]
    StudioDeleted(StudioDeleted),
    // Helm's act on a Studio, as its own kind beside the `studio.changed` the
    // same write publishes. `#1288`.
    #[serde(rename = "studio.helm_acted")]
    StudioHelmActed(StudioHelmActed),
    // Helm's reading sent a person's words to a Session instead of to Helm.
    #[serde(rename = "helm.thought_routed")]
    HelmThoughtRouted(HelmThoughtRouted),
    // A Helm session reached for something the person's own settings do not
    // cover, and is held inside the call until they answer. `#1389`.
    #[serde(rename = "helm.asking_to_run")]
    HelmAskingToRun(HelmAskingToRun),
    // What became of one such ask, whoever ended it.
    #[serde(rename = "helm.call_answered")]
    HelmCallAnswered(HelmCallAnswered),
    // A session after any fact about it, whole, so a client replaces the row it
    // holds. `docs/concepts/session.md`.
    #[serde(rename = "session.changed")]
    SessionChanged(SessionRecord),
    // One row of a hosted session's thread, appended or replaced by its id. Since 23.49.
    #[serde(rename = "session.row")]
    SessionRow(SessionRowChanged),
    // A Job's shell opened or ended, whoever opened or ended it. Its output is
    // `observe_session_run`'s, on the run id these carry.
    #[serde(rename = "terminal.opened")]
    TerminalOpened(JobTerminal),
    #[serde(rename = "terminal.exited")]
    TerminalExited(JobTerminal),
    // Sleep mode, or a repository's Autopilot (#2092), after any change to it, whole.
    #[serde(rename = "sleep.changed")]
    SleepChanged(SleepState),
    #[serde(rename = "autopilot.changed")]
    AutopilotChanged(Autopilot),
    // The plan's windows after a session read them moving, whole. `docs/concepts/machine.md`.
    #[serde(rename = "plan_usage.changed")]
    PlanUsageChanged(PlanUsage),
    // The default model's step down after any change to it, whole. `docs/concepts/machine.md`, *Quota*.
    #[serde(rename = "model_downshift.changed")]
    ModelDownshiftChanged(ModelDownshiftState),
    // The agents' account signed out or in, or its sign-in started or ended, whole.
    // `docs/concepts/session/a-job-whose-sign-in-expired.md`.
    #[serde(rename = "agent_sign_in.changed")]
    AgentSignInChanged(AgentSignIn),
    // A run of an agent about one pull request moved: queued, running, done or failed, whole.
    // `docs/concepts/pull-review.md`, *Agent runs*.
    #[serde(rename = "review_run.changed")]
    ReviewRunChanged { run: ReviewRun },
    // A place held up for a walk of a pull request moved: checking out, starting, ready, failed or
    // closed, whole. `docs/concepts/pull-review.md`, *Walk*.
    #[serde(rename = "review_walk.changed")]
    ReviewWalkChanged { walk: crate::review_walks::ReviewWalk },
    // A Job's watches after any change to one, whole, as `list_watches` answers them: a start, a
    // reading, a weighing, a stop or an end. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
    #[serde(rename = "watches.changed")]
    WatchesChanged(WatchList),
    // One entry Fleet just kept for the timeline. Since 23.82.
    #[serde(rename = "timeline.appended")]
    TimelineAppended(TimelineAppended),
    // A background act failed after its request was answered, so the refusal rides the stream.
    #[serde(rename = "act.failed")]
    ActFailed(ActFailed),
}

impl Event {
    /// The dotted name this publishes under — `operations/`'s own key.
    ///
    /// **Read out of the serialisation, never matched.** A `match` here would
    /// be a second spelling of the `#[serde(rename)]` lines above, and the two
    /// would disagree the first kind somebody added to one of them.
    pub fn kind(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|value| value.get("kind")?.as_str().map(str::to_string))
            // Unreachable: the enum is externally tagged with `kind` and every
            // variant's body is a struct. Answered rather than panicked,
            // because this runs on the publish path of every event.
            .unwrap_or_default()
    }

    /// The Manifest this event's body names at its top level, where it names
    /// one. Read out of the serialisation for [`Event::kind`]'s reason.
    pub fn about(&self) -> Option<String> {
        let value = serde_json::to_value(self).ok()?;
        value.get("manifest_id")?.as_str().map(str::to_string)
    }
}
