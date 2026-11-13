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
use crate::operator_board::{OperatorBoardChanged, OperatorDispatched};
use crate::operator_page::OperatorPageChanged;
use crate::thought::WordsSent;
use crate::hosted_sessions::{SessionRowChanged, SessionSubagentChanged};
use crate::merge_line::MergeLines;
use crate::mods::ModList;
use crate::preview::PreviewOffers;
use crate::protocol_id::ProtocolId;
use crate::reading::ManifestReading;
use crate::review_runs::ReviewRun;
use crate::rehearsal::CheckoutRunRecord;
use crate::repositories::RepositoryList;
use crate::app_runs::AppRun;
use crate::servers::ServerState;
use crate::settings::SettingsList;
use crate::model_downshift::ModelDownshiftState;
use crate::mac_acts::QueuedList;
use crate::projects::Projects;
use crate::agent_sign_in::AgentSignIn;
use crate::plan_usage::PlanUsage;
use crate::timeline::TimelineAppended;
use crate::autopilot::Autopilot;
use crate::autopilot_decisions::AutopilotDecisions;
use crate::watches::WatchList;
use crate::bug_reports::BugReports;
use crate::job_copy::KeepDraftsChanged;
use crate::lifecycles::JobLifecycles;
use crate::merge_line_view::MergeLineViews;
use crate::clashes::AllWriteClashes;
use crate::standing_calls::CallsChanged;
use crate::job_thread::{JobThreadChanged, JobThreadTail};
use crate::sessions::{SessionId, SessionRecord};
use crate::studio::{Studio, StudioDeleted, StudioJobActed};
use crate::terminals::JobTerminal;
use crate::job_browser::{BrowserAskAnswered, BrowserRequest};

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

/// `session.removed`: the Job row a client holds by `session_id` is gone, so it drops the row. **Only ever
/// a row held from another Mac**; this Mac's own Jobs end and stay listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRemoved {
    pub session_id: SessionId,
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
    // Every merge line as a screen draws it, whole, after anything it is read from changed.
    #[serde(rename = "merge_line_view.changed")]
    MergeLineViewChanged(MergeLineViews),
    // Every standing write clash of every served repository, whole, when the set moved.
    #[serde(rename = "write_clashes.changed")]
    WriteClashesChanged(AllWriteClashes),
    // What `get_preview_offers` answers, whole, when an offer or the Refresh row moved.
    #[serde(rename = "preview_offers.changed")]
    PreviewOffersChanged(PreviewOffers),
    // What calls the person, whole, and the news since the last one: when the list or the news moved.
    #[serde(rename = "calls.changed")]
    CallsChanged(CallsChanged),
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
    // A Job on your words acting on a Studio, as its own kind beside the `studio.changed` the
    // same write publishes. `#1288`.
    #[serde(rename = "studio.job_acted")]
    StudioJobActed(StudioJobActed),
    // A person's words were sent to a running Job.
    #[serde(rename = "operator.words_sent")]
    WordsSent(WordsSent),
    // The Operator's board moved: an arrival came, went into a call, was ruled, or was reordered.
    #[serde(rename = "operator.board_changed")]
    OperatorBoardChanged(OperatorBoardChanged),
    // The Operator did something with a dispatch it held: started a Job, folded it into a live one, or asked.
    #[serde(rename = "operator.dispatched")]
    OperatorDispatched(OperatorDispatched),
    // The Operator's page moved: an item joined Your turn or settled, the order, the top line.
    #[serde(rename = "operator.page_changed")]
    OperatorPageChanged(OperatorPageChanged),
    // A session after any fact about it, whole, so a client replaces the row it
    // holds. `docs/concepts/session.md`.
    #[serde(rename = "session.changed")]
    SessionChanged(SessionRecord),
    // One row of a hosted session's thread, appended or replaced by its id. Since 23.49.
    #[serde(rename = "session.row")]
    SessionRow(SessionRowChanged),
    // A subagent's own thread moved, for a client holding it to read `get_session_subagent` again.
    #[serde(rename = "session.subagent")]
    SessionSubagent(SessionSubagentChanged),
    // A row held from another Mac went: that Mac left it out of its rows, or was forgotten.
    #[serde(rename = "session.removed")]
    SessionRemoved(SessionRemoved),
    // A Job's shell opened or ended, whoever opened or ended it. Its output is
    // `observe_session_run`'s, on the run id these carry.
    #[serde(rename = "terminal.opened")]
    TerminalOpened(JobTerminal),
    #[serde(rename = "terminal.exited")]
    TerminalExited(JobTerminal),
    // An agent's `show_terminal`: Bridge brings this tab to the front without moving the person.
    #[serde(rename = "terminal.shown")]
    TerminalShown(JobTerminal),
    // An agent's act on its Job browser, held by Fleet until Bridge answers it by `request_id`.
    #[serde(rename = "browser.request")]
    BrowserRequest(BrowserRequest),
    // The owner answered or dismissed a page's ask in Needs you, for Bridge to settle in the tab.
    #[serde(rename = "browser.ask_answered")]
    BrowserAskAnswered(BrowserAskAnswered),
    // A repository's Autopilot (#2092), after any change to it, whole.
    #[serde(rename = "autopilot.changed")]
    AutopilotChanged(Autopilot),
    // The answers a running Autopilot gave and the person has not reviewed, whole.
    #[serde(rename = "autopilot.decisions_changed")]
    AutopilotDecisionsChanged(AutopilotDecisions),
    // The plan's windows after a session read them moving, whole. `docs/concepts/machine.md`.
    #[serde(rename = "plan_usage.changed")]
    PlanUsageChanged(PlanUsage),
    // The default model's step down after any change to it, whole. `docs/concepts/machine.md`, *Quota*.
    #[serde(rename = "model_downshift.changed")]
    ModelDownshiftChanged(ModelDownshiftState),
    // His projects after any write to one, and the tasks in none, whole, as `list_projects` answers them.
    // `docs/practices/protocol/projects.md`.
    #[serde(rename = "projects.changed")]
    ProjectsChanged(Projects),
    // The acts waiting for a Mac that could not be reached, every Mac's, whole, after one was kept, sent, refused or taken back.
    // `docs/practices/protocol/his-other-macs-fleets-half.md`.
    #[serde(rename = "queued.changed")]
    QueuedChanged(QueuedList),
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
    // The bug reports agents made of Armada, every one on this Mac, whole, after one was made, kept or dropped.
    #[serde(rename = "bug_reports.changed")]
    BugReportsChanged(BugReports),
    // What a Job holds and where it stands, whole, after an instance was taken on or moved.
    #[serde(rename = "job_lifecycles.changed")]
    JobLifecyclesChanged(JobLifecycles),
    // What changed in a Job's thread as Fleet folds it: the items from `from` to the end, and the head.
    #[serde(rename = "job_thread.tail")]
    JobThreadTail(JobThreadTail),
    // A change reached back further than a tail carries: a client that holds the item reads its window again.
    #[serde(rename = "job_thread.changed")]
    JobThreadChanged(JobThreadChanged),
    // A rule or a lifecycle a Job kept for every Job was begun, approved or dismissed in a repository, with the repository alone.
    #[serde(rename = "keep_drafts.changed")]
    KeepDraftsChanged(KeepDraftsChanged),
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
