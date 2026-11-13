//! The wire vocabulary, as DTOs rather than domain types.
//! `From<core_model::Job> for ipc::JobSummary` at the Fleet boundary is where
//! redaction becomes an explicit, visible step. **A domain type on the wire is
//! a redaction decision nobody made.**
//!
//! `ProtocolId` is a hash `build.rs` takes of these types, the operation
//! inventory and Bridge's mirror of them, and codegen emits the same hash for
//! Bridge. Both outputs are checked in, so a cross-language change is a build
//! failure rather than a runtime surprise. This is one of two crates permitted to deserialize.
//!
//! **The conversion lives here rather than in Fleet.** The *decision* is
//! Fleet's, since Fleet is the only caller, but the `impl` cannot be: neither
//! `core_model::Job` nor `ipc::JobSummary` belongs to `fleet`, so the orphan
//! rule puts every `From` at this boundary here. What Fleet keeps is which
//! conversion to call and what to pass it.
//!
//! **Nothing here restates a spelling.** `core-model` carries an
//! `as_wire`/`from_wire` pair beside every enum and the wire value **is** the
//! registry key, so every closed set below spells the domain value through it.
//! There is no variant-to-string `match` here — the defect just removed from
//! `store`, and the one a second vocabulary always becomes.
//!
//! **Not the full protocol surface.** `crates/ipc/operations/` inventories
//! every operation; the types here serve what M1 needs, and a command adds a
//! type only where a Job is not what it answers with.

/// The account a machine is signed in to, its profiles, and the sign-in in flight.
mod accounts;
/// A note Bridge's annotation layer wrote, marked done by the session it was sent to.
pub mod annotations;
/// A form's edits to `armada.yml`, as edits — the half of Journey 9's *Editing*
/// that changes only the lines it touches.
mod amending;
/// The build Fleet runs on, where it stands against `main`, and the request that
/// moves it. **Fleet-wide, and not a Job's field.**
mod building;
/// The branches Jobs offer for the owner's preview, and the Refresh row built from them.
mod preview;
/// A scout's run, as its Finding records it, and the acts that start and stop one.
/// `#1292`.
mod capturing;
mod checks;
/// An agent's word that a Fleet notice was wrong, and the review that settles it.
mod disputes;
/// An agent's report that Armada itself got something wrong, kept on this Mac.
mod bug_reports;
/// Workspaces, lifecycles and rules, and where a Job's tickets, pull requests and deploys stand. Reads only.
mod lifecycles;
/// Saving and deleting one lifecycle, rule or autonomy default in Kit.
mod piece_saving;
mod piece_drafts;
mod workflow_drafts;
mod workflow_versions;
mod workflow_migrating;
mod codec;
/// A body's whole shape, read off its type, for a refusal to name.
pub mod shape;
/// A command a Drone was not given, and what a person answers about it. **Two
/// paths to one answer** — while the Drone waits, or after the Job stopped.
mod commanding;
/// A copy of a person's own piece committed on a Job's branch.
mod committing;
/// One Manifest as Fleet resolved it, past the summary a picker reads.
mod configured;
/// One entry written into a JSON document Armada does not own. Here for
/// `codec`'s reason: bytes nobody in this process typed.
pub mod document;
/// The agent's door: the MCP half of the HTTP surface, and the tool set
/// `build.rs` emits from `operations/`'s own `agent_access` column.
pub mod door;
/// `armada job`'s half of the door: the list it asks for, the call it makes and the answer it reads.
pub mod job_cli;
/// Whether the repository still has what `armada.yml` names. **A read of the
/// repository**, where `reading` is a read of the file — a `run` line naming a
/// deleted script parses perfectly and says nothing.
mod drift;
/// What the Workflows page reads beside a definition's text, and its acts on a copy.
mod workflow_page;
/// A Job's own copy of its workflow, lifecycles and rules, and what was changed in it.
mod job_copy;
/// A change to a workflow an agent drafts for the owner to approve.
mod workflow_changes;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with, where `reading` is the half that reports.
mod editing;
mod enums;
mod error;
mod event;
/// The text of Copy debug report, and the reading of Fleet's log it holds names from.
pub mod debug_report;
pub mod log_trouble;
/// What `search_files` found under the checkout, for the `@` mention popup.
mod files;
/// What Fleet can say about its own health, and what it cannot. **Not
/// Doctor**, whose grid is ten modules and is not built.
mod health;
/// A call a Job made that its person's settings do not cover.
mod call;
/// The Operator's first shortlist of Jobs for words a person typed, and the words sent on.
mod thought;
/// What happened across the Fleet and when, for the Cockpit's timeline.
mod timeline;
/// Who held a slot and what was checked out in it, for the side panel's Bay.
mod bay_history;
/// A Job's processes, their load, its worktrees and logs, and ending one process.
mod session_system;
/// What Fleet is holding disk for, and the test each one did not pass.
/// **A piloted worktree is not on this wire at all** — `#367`.
mod holding;
/// A session Bridge hosts: its commands, its thread and the gate its first
/// write goes through. `docs/concepts/session.md`.
mod hosted_sessions;
mod ids;
/// A Job's thread as a screen draws it, folded by Fleet: `get_job_thread` and `job_thread.tail`.
mod job_thread;
/// The state a Jobs-list row draws, on `SessionRecord::mark`.
mod job_mark;
/// Whether a Job offers Compact, on `SessionRecord::compact`.
mod job_compact;
/// A Job's lanes as a screen draws them, on `SessionRecord::lanes`.
mod job_lanes;
/// How full a limit is, as a ring draws it: `RingBand`.
mod ring_band;
/// When the person was last in a Job: `view_session`, and `SessionRecord::viewed_at`.
mod job_viewing;
/// The merge line as a screen draws it, read by Fleet: `get_merge_line_view` and `merge_line_view.changed`.
mod merge_line_view;
/// What calls the person and which of it he has not opened, read by Fleet: `get_calls`, `calls.changed` and `open_call`.
mod standing_calls;
/// The Cockpit's flight controls a person arranged in Kit, and a shell action run from one.
mod flight_controls;
/// Pieces made into a package, read, and added to the person's own Kit.
mod sharing;
/// A repository's open issues, as Fleet read them. #91.
mod issues;
/// What names an issue a Session works on, the forge's number or a tracker's key, and `claim_issue`.
mod issue_reference;
mod issue_queries;
/// Projects: per repository, tasks a person wrote and issues he linked, in order.
mod projects;
/// The values a person sets for how work behaves, and each with the layer it came from.
mod values;
/// Kit's MCP servers and each Manifest's word over one (`#1275`), and the
/// setup a person already works with, read to be shown (`#1491`).
mod kit;
/// Fleet's three changeable limits. **A value out of range does not decode.**
mod limits;
/// The merge line of each served repository, as Fleet serves it.
/// A possible `armada.yml` per workspace, and the edits and Write that finish it.
mod manifest_proposal;
mod merge_hub;
mod merge_line;
/// `layout.json`: what a layout mod or the owner's own choice may say. `docs/concepts/layout-mods.md`.
pub mod layout;

/// The owner's key bindings: which keys an act answers in place of the registry's. `docs/concepts/key-bindings.md`.
pub mod key_bindings;
/// How the owner has Bridge draw a time: three switches. `docs/concepts/machine.md`.
pub mod time_format;
/// The folders every session may reach past its slots, and the ones it may not. `docs/concepts/session.md`.
pub mod reach_folders;
/// How far each of the Operator's roles goes, for the machine and for one repository. `crates/config/settings.toml`, *Operator positions per repository*.
/// The Operator's board: what it is ruling on, what waits for it and what it settled. `crates/ipc/operations/get_operator_board.toml`.
pub mod operator_board;
/// An error Bridge showed the person, sent to the Operator. `crates/ipc/operations/report_bridge_error.toml`.
pub mod operator_errors;
/// The box at the foot of the Operator panel, and the standing orders it turns words into. `crates/ipc/operations/talk_to_operator.toml`.
pub mod operator_talk;
/// The Operator's page: Your turn and the digest beside it. `crates/ipc/operations/operator_page.toml`.
pub mod operator_page;
/// The workflows switched off, for the machine and for one repository. `crates/config/settings.toml`, *Workflows switched off*.
pub mod workflows_off;
/// Mods: a directory on this machine that changes how Bridge looks.
mod mods;
/// Where two agents' changes in one repository conflict. `docs/concepts/fleet.md`, *Write clashes*.
mod clashes;
/// What each served repository holds and where each agent is changing it, for the deck's map.
/// `docs/concepts/fleet.md`, *The territory*.
mod territory;
/// A person's Bridge preferences, kept the way `limits` are kept. **A value
/// out of the closed set does not save**, `limits`' reason one field over.
mod preferences;
/// settings.json: every setting as Fleet resolved it, the save, and a refusal.
mod settings;
/// Kit against the shared home: each row's state, the acts that settle one, and what syncs.
mod shared_home;
/// Which wire a build speaks: a hash `build.rs` takes of the wire surface.
mod protocol_id;
/// A pull request, by repository and number, and the acts a Session takes on one.
/// Since 23.48.
mod pull_requests;
/// The pull requests waiting on a person as four lists, and the acts of a review on one.
mod pull_reviews;
/// Fleet's last read of `armada.yml`, held rather than announced. **The one shape here about the
/// fleet and not about a Job**, beside `capacity`. What a scout answers a read-in with. `#1293`.
mod read_in;
mod reading;
/// A person's run of one Manifest entry, in a Job's worktree or in the main checkout. **A
/// rehearsal, never a verdict** — nothing in it is a Check row or Evidence.
mod rehearsal;
/// The repositories one Fleet serves, and adding one by folder.
mod repositories;
/// A file of a Job read by Fleet for a client that cannot read the agent's disk: `get_repository_file`.
mod repository_file;
/// Who asked for a Check run, as a typed value. Since 23.40.
mod requester;
/// Requests: an HTTP request the owner or an agent sends, its collections and environments.
mod requests;
/// `send_requests`: one template sent for each body file and each value, answered a line each.
mod requests_batch;
/// A saved request's runs, kept after the Job that sent them ends.
mod request_runs;
/// A Judge on demand: a read-only call on one artifact of a Job, answered with notes.
mod judge;
/// What an agent says about one pull request, and the review tools it is asked with.
mod review_runs;
/// Natural Search, and where a change runs for the Walk tool.
mod review_walks;
/// The owner's notes on a pull request's lines, and a Job resolving one.
mod pull_request_notes;
/// A Session's retro and the record it was read from, and the Lessons listing.
/// Since 23.12.
mod retro;
/// A guided review of the open retro items, and a question about one. Since
/// 23.74.
mod retro_review;
mod retro_session;
/// What Scan found in a repository nobody set up for Armada. **Evidence,
/// never a proposal** — every finding carries the file it came from.
mod scan;
mod scouting;
mod seeding;
/// A repository's app started from Bridge: several servers held as one run.
mod app_runs;
/// A Command that stays running, held by Fleet. **Lifecycle on `/events`,
/// output on a socket of its own.**
mod servers;
/// The session ledger: what a harness reports of a session, and what Fleet
/// answers. `docs/concepts/session.md`.
mod session_checks;
/// A note pinned on a page, sent to a Session whole. `send_session_note`, `send_session_notes`.
mod job_dispatch;
mod session_notes;
/// A hosted job's MCP servers, read and acted on. `get_session_mcp`, `act_on_session_mcp`.
mod session_mcp;
/// The replies a Job's quick bar offers that no setting holds. `get_session_quick`.
mod session_quick;
mod slot_asking;
mod slot_choice;
mod session_retro_row;
mod session_starts;
mod started_jobs;
mod session_waits;
mod session_summary;
mod run_memory;
mod logbook;
mod teaching;
mod taught_mods;
mod notices;
mod session_import;
mod draft_document;
mod pre_pr;
mod sessions;
/// A Job's shell, shared by the person and its agent.
mod terminals;
/// The Job browser: a page in Bridge the agent and the owner drive together.
mod job_browser;
mod setup;
mod phone;
mod macs;
mod mac_acts;
mod show_file;
mod package_manifest;
/// A Job's sketches, drafts and pages as one record. `get_job_artifacts`.
mod artifact;
mod sketch;
/// A sketch's scene read whole, as a client draws it.
mod sketch_scene;
/// A screen node's layout tree read whole, as a client draws it.
mod sketch_screen;
mod style_sources;
/// What a Job draws a `mock` with, read once. `get_design_catalogue`.
mod design_catalogue;
/// A Job's document put to the person, and his answer.
mod document_answer;
/// What a Job waits on the person for, grouped once. `get_needs_you`.
mod needs_you;
/// Which tests reach a set of paths, and which paths nothing reaches. `get_tests_reaching`.
mod test_map;
/// A background act that failed after its reply.
mod act_failed;
/// Autopilot: a repository working its own backlog. #2092.
mod autopilot;
/// What a running Autopilot answered for the person, to review.
mod autopilot_decisions;
/// How much of the plan's windows is used.
mod plan_usage;
/// The default model stepping down while the weekly window runs out.
mod model_downshift;
/// The agents' account signed out, for the whole Fleet.
mod agent_sign_in;
/// What fills a session's context window.
mod context_breakdown;
/// What a session's agent knows, was told, and used each turn, and where each came from.
mod loadout;
/// What a session's turns cost.
mod turn_usage;
/// The token rules Fleet tells a Job about, and the Jobs it told.
mod usage_rules;
/// What a window of turns cost, summed.
mod usage_summary;
/// What the burn was spent on, and who spent it.
mod usage_spent;
/// Where Jobs' time went: one Job's, a window's, and the hours and builds the Usage page draws.
mod time_summary;
/// The picture a Job last showed, which the native done card leads with.
mod lead_picture;
/// What a hosted Session is doing now, as Fleet saw it.
mod session_activity;
/// What a Session has built: its phase, pieces and road, read off what Fleet observed.
mod session_build;
mod session_plan;
/// How a Job's work is to be delivered, and the one thing that is asked about it.
mod session_delivery;
/// What crossed the stream since a cursor, counted rather than carried.
/// **An agent's substitute for the socket it cannot hold.**
mod since;
mod slot_found;
/// A Studio, its nodes and edges, and the acts a client asks of one. `#1285`.
mod studio;
/// What `add_studio_node` carries: any content, a Picture staged rather than kept.
mod studio_added;
mod studio_sketch;
/// The lines a running log carries over its socket, and why one stopped arriving.
mod underway;
mod walk_steps;
/// A command Fleet runs on a cadence for one Job, the agent's weighing of it, and its line.
mod watches;
/// A watch tried before it starts, the recipes there are, and one kept on this machine.
mod watch_tools;

#[cfg(test)]
mod tests;

pub use annotations::{
    AnnotationMarked, MarkAnnotationDone,
};
pub use disputes::{DisputeFleet, DisputeSpotted, FleetDisputed};
pub use bug_reports::{BugFixed, BugReport, BugReportDropped, BugReported, BugReports, ReportBug};
pub use workflow_drafts::{DraftBy, DraftChange, PublishWorkflowDraft, PutWorkflowDraft, WorkflowDraft, WorkflowDraftDiscarded, WorkflowPublished};
pub use workflow_versions::{BasedOn, RevertWorkflow, VersionOrigin, WorkflowVersion, WorkflowVersions};
pub use workflow_migrating::{AffectedJob, AffectedJobs, MigrateJobs, MigrateResult, MigratedJobs};
pub use piece_drafts::{PieceDraft, PieceDraftDiscarded, PiecePublished, PublishPieceDraft, PutPieceDraft};
pub use piece_saving::{
    AutonomyToSave, DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceDeleted, LifecyclePieceKind,
    LifecyclePieceSaved, SaveLifecyclePiece, TeachTicketWord, TicketWordTaught,
};
pub use lifecycles::{
    ActKindDefault, AppliedLifecycle, AutonomyLayer, RepositoryAutonomy, AppliedRule, HeldInstance, InFlight, InstanceLink, InstanceMove, InstanceStanding, JobLifecycles,
    LifecycleDefinition, LifecycleRule, LifecycleStateDefinition, LifecycleTransition, PieceKept, PieceLeftOut,
    PieceRank, PieceScope, RepositoryLifecycleRules, RepositoryLifecycles, RuleAct, RuleAmong,
    RuleActReading, RuleCondition, RuleDoing, RuleOnFailure, RuleTarget, RuleWhen, StateReadFrom,
};
pub use amending::{
    CheckDraft, CommandDraft, EditManifest, EvidenceDraft, LinkDraft, ManifestDeclared,
    ManifestEdit, ManifestEdited, NamedCheck, NamedCommand, NamedPort, NarrowingDraft,
    PolicyWords, PortDraft,
};
pub use building::{
    BuildPosition, BuildSource, BuildStage, BuildSwitcher, RestartAct, ChangeFleetBuild, FleetBuildChanging, FleetBuildKept,
    FleetBuildReport, FleetBuildReverting, FleetBuildSnapshotting, FleetBuildSwapping, KeptBuildPrevious,
    KeptBuildRunning, KeptSnapshot, LeftOutBranch, OriginBranch, PickFleetBuildJob, PickedJob, SnapshotState,
};
pub use preview::{
    OfferPreview, PreviewOfferRow, PreviewOffered, PreviewOffers, PreviewRefresh, PreviewRefreshSkipped, PreviewState,
    PreviewWithdrawn, SkipPreviewRefresh, WithdrawPreview,
};
pub use capturing::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureServed, CaptureStudioNote,
    CaptureWindow, StagedFrame, StudioCapture,
};
pub use checks::{
    CheckOutput, CheckRun, DeclaredCheck, DeclaredJudge,
};
pub use codec::{
    decode, decode_body, encode, Undecodable, Unencodable,
};
pub use commanding::{
    AllowedCommandRow, RemoveRepositoryAllowedCommand, RepositoryAllowedCommands,
};
pub use configured::ManifestConfig;
pub use job_copy::{
    ChangeJobCopy, ChangeSessionSteps, ChangedBy, CopyPieceKind, JobCopy, JobCopyChange, JobCopyPiece, JobCopyStep, JobPieceProposed,
    KeepDraft, KeepDrafts, KeepDraftsChanged, KeepJobPiece, KeepJobWorkflow, ProposeWorkflowChange, StepChange,
};
pub use workflow_changes::{
    AskWorkflowChange, ArmadaProposed, ChangeOrigin, ChangeState, ChangeStep, ProposeNewWorkflow, ProposeToArmada,
    SendBackWorkflowChange, WorkflowChange, WorkflowChangeApproved, WorkflowChangeRef, WorkflowChanges,
    WorkflowSupportBuilding,
};
pub use workflow_page::{
    StepReading, WorkflowCopy, WorkflowCopyDeleted, WorkflowSteps,
};
pub use drift::{
    Declaration, Drift, ManifestDrift, PackageScripts, Unfollowed,
};
pub use editing::{
    ManifestFile, ManifestSaved, SaveManifestFile,
};
pub use enums::{
    Actor, AdvanceGate, CheckOutcome, EdgeGrain, EdgeSource, EvidenceType, LandsIn, LessonState,
    ManifestReach, PrMode, ReachesDrones, ScoutSourceKind, StudioAuthor, StudioEdgeKind,
    StudioEdgeStanding, StudioNodeKind, StudioNodeState, StudioRelation, Via, Whose,
};
pub use error::{
    RunId, WireError, WireValue,
};
pub use event::{
    Cursor, Delivered, Event, Missed, Resync, SessionRemoved, StreamMessage,
};
pub use files::FilesFound;
pub use debug_report::DebugReport;
pub use health::{
    FleetHealth, ActionAuthority, Probe, Unprobed,
};
pub use thought::{
    WordsSent, ReadThought, TellSession, ThoughtKind, ThoughtReading,
    ThoughtTarget,
};
pub use session_system::{
    ProcessStopped, SessionSystem, StopProcess, SystemLog, SystemProcess, SystemProcessState, SystemWorktree,
};
pub use bay_history::{
    BayHistory, BayHistoryAsk, BayHistoryEntry, BayHolder, BayHolderKind, MOST_BAY_HISTORY,
};
pub use timeline::{
    FleetTimeline, ServiceCall, TimelineAppended, TimelineAsk, TimelineEvent, TimelineKind,
    TimelineSubject, TimelineTone, DEFAULT_TIMELINE_LIMIT, MOST_TIMELINE_ITEMS, MOST_TIMELINE_LIMIT,
};
pub use call::{
    AskedChoice, AskedQuestion, AskingToRun, CallKind, CallAnswer, CallInFlight, QuestionAnswer,
    ReachesTo, RunOrNot, handoff_figure, ASKS_A_QUESTION, HANDOFF_QUIET, HANDOFF_REMIND, HANDOFF_REMIND_AT,
    HANDOFF_REMIND_QUESTION, HANDOFF_STAY, HANDOFF_YES,
};
pub use holding::{
    ReclaimedSaved, ChangeSlotPool, CommitHome, RescueAct, RescueSlot, SlotAct, SlotCommit,
    SlotFinding, SlotFindingState, SlotHolderKind, SlotHolding, SlotLastHolder, SlotPoolChanged,
    SlotReleased, SlotRescued, SlotStranded, SlotVerdict, WorktreeSlot, WorktreesHeld,
};
pub use job_dispatch::{DispatchJob, DispatchKind, DispatchMessage, DispatchThen, JobDispatched, ParkedIdea, ParkedIdeas, RemoveParkedIdea};
pub use hosted_sessions::{
    AnswerSessionAsk, BeginSession, ChosenHold, CloseSession, DraftHow, DraftSettled, EditSessionMessage, ForkFrom, GateAnswer, GateHold, HeldCommand,
    HoldKept, HoldMoved, HostedFacts, LinkedEvent, MessagesHeld, MoveSession,
    ReleaseSessionHolds, RunSessionCommand, SendSessionMessage, SendSessionMessageBody, SentFile, SentSketch, SessionAskState,
    SessionGate, SessionHoldsReleased, SessionMode, SessionRow, SessionRowChanged, SessionSubagentChanged, SessionTag,
    SetSessionWorkflow, SkipSessionStep, JumpSessionStep, SessionSubagent, SessionThread, SessionTurn, SessionUpload, ToolMessage, MessagePeer, ToolSubagent, ToolSubagentState,
    SessionVoice, SessionVoiceNamed, StartSession, StopSession, RetryHandoff, PauseSession, ResumeSession, SnoozeSession, SessionPaused, StartAgain, MendSessionTools, StartFailed, SetSessionAutoLand, SetSessionProject, TagKind, TakeHeld, TerminalAsk,
    TerminalAsked, TuneSession, MessageReach, MessageTo,
};
pub use session_mcp::{
    ActOnSessionMcp, McpServerAct, McpServerState, McpSignIn, SessionMcp, SessionMcpActed,
    SessionMcpServer,
};
pub use session_quick::SessionQuick;
pub use merge_line_view::{MainRedJob, MainView, MergeLineView, MergeLineViews, PullTone, PullView};
pub use job_mark::{JobGroup, JobLamp, JobMark, JobState};
pub use job_compact::{CompactHow, CompactOff, CompactOffer};
pub use job_lanes::{
    JobLanesHeld, JobLanesView, LaneAutonomy, LaneCarriedTicket, LaneChange, LaneCopy, LaneEdge, LaneFired, LaneFiredAct,
    LaneInstance, LaneKept, LaneKind, LaneLifecycle, LaneLifecycleKind, LaneLifecycleState, LaneMoment, LaneNode,
    LaneNodeKind, LaneNodeRole, LaneOutcome, LanePending, LaneRef, LaneRule, LaneStep, LaneView,
};
pub use repository_file::RepositoryFile;
pub use ring_band::{RingBand, DANGER_FROM, WARNING_FROM};
pub use job_viewing::ViewSession;
pub use standing_calls::{CallArrival, CallArrivalKind, CallTone, CallsChanged, OpenStandingCall, StandingCall, StandingCallKind, StandingCalls};
pub use job_thread::{
    against_of_detail, file_call_of, kept_from, revision_paths_of_detail, revisions_of_detail, scene_of_detail, sketch_call_of,
    SketchCall, CardSettled, ChangedValue, FleetAct, FleetActKind, FleetActState, JobThread, JobThreadChanged, JobThreadRead,
    JobThreadTail, Lapsed, OpenCall, PlanCardState, ThreadAsk, ThreadDocument, ThreadHead, ThreadItem, ThreadPlan, ThreadSketch,
    ThreadTrail, TrailBay, TrailOpened,
    THREAD_ITEMS_MOST, THREAD_ITEMS_SERVED, THREAD_TAIL_MOST,
};
pub use session_notes::{
    Candidate, NoteCrop, NoteElement, NotePlace, NoteSent, NoteState, NoteTake, NoteWalk,
    NotesSent, SendSessionNote, SendSessionNotes, SessionNote,
};
pub use ids::{
    Instant, ManifestId, StepId, StudioEdgeId, StudioId, StudioNodeId, WorkflowId,
};
pub use committing::{CommitToRepository, CommittedToRepository, PieceToCommit};
pub use flight_controls::{
    ApproveRepositoryTile, RepositoryTile, TileAsk, FlightControl, FlightControlKind, FlightControls, FlightControlsRead,
    RepositoryFlightControls, RunSessionShell, SaveRepositoryFlightControls, SessionShellRan,
    FLIGHT_CONTROLS_VERSION,
};
pub use sharing::{
    AddPackage, BindUnboundPiece, Landed, PackageAdded, PackagePiece, PackageRead, PackageText,
    PieceAdded, PieceKind, PieceToShare, SharePieces, SharedPackage, TileList,
    UnboundPiece, UnboundPieces, PACKAGE_VERSION,
};
pub use sharing::{KitManifestSaved, SaveKitManifest};
pub use shared_home::{
    ChooseHomeSide, HomeItem, HomeKind, HomeKindSwitch, HomeKinds, HomeLeft, HomeNamed, HomePlanned, HomeProfile, HomeRow, HomeRowAt,
    HomeRowState, HomeSide, HomeSynced, HomeUnreadable, SetHomeItem, SetHomeKind, SetHomeRepository, SharedHomeRead,
};
pub use issue_reference::{ClaimIssue, IssueClaimed, IssueReference};
pub use issue_queries::{
    InstallTool, InstalledTool, IssueQueries, IssueQueryGuide, SaveIssueQuery, SavedIssueQuery, SearchIssues, WriteIssueQuery,
    WrittenIssueQuery,
};
pub use values::{KitValuesSaved, LookSet, ResolvedAutonomy, ResolvedReviews, ResolvedTracker, ResolvedValue, ResolvedValues, ResolvedWorkflow, ReviewsSet, SaveKitValues, SetProjectWorkflow, ValueLayer, ValuesSet};
pub use projects::{
    EditProject, EditedTaskFields, LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, MoveProjectTaskRepository, NewProject, NewProjectIssue,
    NewProjectTask, Project, ProjectAct, ProjectActFromMac, ProjectHolder, ProjectLeftOut, ProjectMember, ProjectPlace, ProjectRow, ProjectRowRef, ProjectTask, ProjectTaskState, Projects, TaskIssue, TaskPull, TaskPullState,
    TickProjectTask,
};
pub use issues::{
    IssueComment, IssueField, IssueList, IssueRead, IssueState, LinkedPullRequest, LinkedPullRequestState,
    ProjectIssueList, TrackedIssue,
};
pub use kit::{
    AddKitServer, ForgetKitServer, InstallKitPiece, KitAllowedCommand, KitAllowedCommands,
    KitAllowedSource, KitInventory, KitPieceInstalled, KitPieceKind, KitPieceRemoved,
    KitServerRow, KitServers, KitSourceChoice, KitSourceFound, LookAtKitSource, OutputStyleFacts,
    RemoveKitAllowedCommand, RemoveKitPiece, RestoreKitPiece, ServerAddress, SetKitServerReach,
    SetManifestServerReach, SetupItem, SetupKindRow, SetupUnreadable, WhatWasRead,
};
pub use limits::{
    ChecksAtOnce, DiskFloorGib, DronesAtOnce, FleetLimits, LimitValues, MemorySparePercent,
    SaveLimits, Within,
};
pub use manifest_proposal::{
    Band, EditManifestProposal, ManifestProposal, ManifestProposals, PolicyKey, ProposalEdit,
    ProposedCheck, ProposedCommand, ProposedId, ProposedPolicy, ProposedPort, ProposedRunner,
    ProposedSetup, Provenance, WriteManifestProposal,
};
pub use merge_hub::{
    CatchUpCheckout, CheckoutCaughtUp, CheckoutDiff, CheckoutDiffAsk, CheckoutPullRequest,
    CheckoutSentAsPullRequest, CheckoutToPullRequest, ClaimHolder, FixingElsewhere,
    HeldCheckout, HubMergeable, HubMerged, HubPullCi, HubPullRequest, HubQueue, HubSession,
    HubQueueState, HubReview, MainChecking, MainCiState, MainFailedJob, MainMerge, MainRun,
    MainRunState, MainStanding, MergeLineHub, PullRequestOwner,
};
pub use merge_line::{
    Settled, LandCheckState, LandOutputMessage, LandOutputOpened, LandState, MergeLine,
    MergeLineCheck, MergeLineEntry, MergeLinePullRequest, MergeLines,
};
pub use mods::{
    InstrumentModule, ModAim, ModChecked, ModKind, ModList, ModOrigin, ModPromoted, ModPullRequest, ModScaffolded, ModSessionStarted, ModSummary, TabModule,
    PromoteMod, ScaffoldMod, SetModEnabled, StartModSession, TrackerTry, TryTrackerMod,
};
pub use clashes::{
    AllWriteClashes, ClashFile, ClashHolder, ClashSide, RepositoryClashes, WriteClash, WriteClashes,
};
pub use logbook::{ForSubagent, Recalled, RecalledEntry, SubagentAsk};
pub use territory::{
    AreaKid, AreasOfRepository, HolderTouches, RepositoryArea, RepositoryAreas, SessionFileChange,
    SlotTouches, TouchesInRepository,
};
pub use preferences::{
    AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, SavePreference, TakeIn, DEFAULT_THEME,
};
pub use settings::{
    SaveSettings, Setting, SettingApplies, SettingKind, SettingValue, SettingsList,
    SettingsRefused,
};
pub use pull_requests::{
    FailureWeight, ForgeChecks, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit,
    PullRequestDraft, PullRequestEntry, PullRequestFile, PullRequestFileOf, PullRequestFilePatch,
    PullRequestFilePatches,
    PullRequestFileStatus, PullRequestMergeable, PullRequestPage, PullRequestReviewState, PullRequestReviewer,
    PullRequestReviewerKind,
    PullRequestStanding, PullRequestState, PullRequestVerdict, PullRequestWriteKind,
    WritePullRequest,
};
pub use pull_reviews::{
    DiffSide, PullReviewChecks, PullReviewComment, PullReviewList, PullReviewLists,
    PullReviewRow, PullReviewThread, PullReviewThreadComment, PullThreadResolved,
    ResolvePullThread, MOST_PER_LIST,
};
pub use read_in::{
    what_a_scout_read_in, ReadIn, ReadInCluster, ReadInContradiction, ReadInNote,
    ReadInRelation, MOST_CHARACTERS, MOST_CLUSTERS, MOST_CONTRADICTIONS, MOST_NOTES,
    MOST_RELATIONS,
};
pub use reading::{
    ManifestFault, ManifestMoved, ManifestReading, ManifestRefused,
};
pub use rehearsal::{
    ChangeKind, ChangedFile, CheckoutRunDiff, CheckoutRunList, CheckoutRunMessage,
    CheckoutRunOpened, CheckoutRunRecord, CheckoutRunSheet, CheckoutRunUnderway, DiffAgainst,
    NamedRun, RunDiffReading, RunEntry, RunOutput, StartCheckoutRun, UnreadableRun,
    WorkspaceCommands,
};
pub use rehearsal::{
    CheckoutVerify, StartCheckoutVerify, VerifyGroup, VerifyStep, VerifyStepState,
};
pub use request_runs::{
    RequestRun, RequestRunAnswer, RequestRunJob, RequestRunSent, RequestRunStatus, RequestRunSummary, RequestRuns,
};
pub use requests_batch::{RequestLine, RequestTemplate, RequestVary, RequestsSent, SendRequests};
pub use requests::{
    DeleteRequestEnvironment, Placed, RequestBy, RequestCollection, RequestEnvironment, RequestEnvironmentRead,
    RequestBodySaved, RequestEnvironmentWrite, RequestHeader, RequestPlace, RequestSaved, RequestSent,
    RequestToSave, RequestToSend, RequestVariable, Requests, RequestsFile, RequestsNotRead,
    SaveRequestCollection, SaveRequestEnvironment, SaveRequestToCollection, SavedRequest,
    SavedRequestNamed, SendRequest, VariableRead, pick as pick_json,
    VariableWrite, REQUESTS_VERSION, SERVER_BASE_VARIABLE, SERVER_ENVIRONMENT_PREFIX,
};
pub use repositories::{
    AddRepository, CloneRepository, CreateProject, ForgeRepositories, ForgeRepository, RepositoryList,
    RepositorySummary, RequestRepository,
};
pub use requester::Requester;
pub use logbook::{
    EditLogbookEntry, Logbook, LogbookEarlier, LogbookEntry, LogbookEvidence, ForgottenLogbookEntry, LogbookKind, LogbookReplacedBy,
    LogbookSection, LogbookState, LogbookTidy, LogbookTidyLast, LogbookTopic, LockLogbookEntry, AppliedOrganizeLogbook,
    ApplyOrganizeLogbook, OrganizeAction, OrganizeLogbook, OrganizedLogbook, OrganizingLogbook, DismissedOrganizeLogbook, LogbookOrganize, OrganizeProposal, TeachLogbookEntry, TeachSource, TidiedLogbook, TidyLogbook,
};
pub use teaching::{
    ProposedAddress, ProposedFleet, ProposedRule, ReadTeaching, SaveTeaching, SavedTeaching, TeachingLayout, TeachingNote, TeachingProposal, TeachingReach,
    TeachingSource, TeachingSourceKind, TeachingWords, TweakChange, WordRule, WordRules,
};
pub use taught_mods::{AddressFound, CommandsFile, Condition, Does, FleetFile, FleetRule, Instead, PaletteCommand, RuleScope, WordsAddress, WordsFile};
pub use notices::{Notice, NoticeScope, SetNoticeMute};
pub use run_memory::{
    DropRunFact, RememberedRun, RunCloud, RunFactState, RunMemory, RunMemoryPlace, RunPlace, RunWorkspace, SetRunMemoryPlace,
};
pub use session_import::{
    ImportFolder, ImportFolders, ImportGit, ImportRuns, InstallAgentMod, ModInstallStep, ModInstalled, RunFact,
    RunFactKind,
};
pub use judge::{
    AskJudge, DismissJudgeNote, JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeNoteSettled, JudgeReading,
    JudgeState, MarkJudgeRead, SendJudgeNote,
};
pub use review_runs::{
    PullReviewHolder, ReviewChange, ReviewFinding, ReviewImpact, ReviewLink, ReviewPlace,
    ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, ReviewRuns, ReviewSeverity,
    ReviewStep, ReviewStepSource, ReviewTheme, ReviewTool, ReviewToolKind, ReviewToolLeftOut, ReviewTourStop,
    ReviewToolScope, ReviewToolSource, ReviewTools, SaveReviewTool, StartReviewRun,
};
pub use pull_request_notes::{
    NoteSide, NoteStatus, PullRequestNote, PullRequestNotes, ResolvePullRequestNote, SavePullRequestNote,
    SendPullRequestNotes,
};
pub use review_walks::{
    ForgetReviewSearch, OpenReviewWalk, RememberedPlace, RememberedPlaces, ReviewMatch, ReviewRequest, ReviewRequestMethod,
    ReviewWalk, ReviewWalkState, SaveWalkPlace, StartReviewSearch, WalkHow, WalkPlace, WalkPlaceFound,
    WalkSource, WalkTried, SEARCH_RUN_SCOPE,
};
pub use retro::{
    Lesson, Lessons, RecordAct, RecordAsked, RecordRefusal, RecordSaid, RecordSplit, RecordTime, Retro, RetroAnswered,
    RetroChange, RetroChangeAnswered, RetroItem, RetroRecord, RetroSession, RetroState,
    RetroWritten,
};
pub use retro_session::LessonSessionStarted;
pub use retro_review::{
    AskLessonAnswer, AskRole, AskTurn, LessonAsk, LessonReview, ReviewEntry, ReviewLessons,
    SetAside,
};
pub use scan::{
    CiCommand, ComposeService, DeclaredPort, EvidenceStrength, MissingName, NotRead,
    PackageWorkspaces, RepositoryScan, Runnable, ScannedWorkspace, ToolFile, ToolSection,
    WorkspaceGlob, WorkspaceGlobs,
};
pub use scouting::{
    AskScout, ReadInLink, ScoutCheckout, ScoutEnded, ScoutOutcome, ScoutSource, StartScout,
    StopScout,
};
pub use seeding::{
    DeclaredSeed, SeedWarmth,
};
pub use app_runs::{
    AppRun, AppRunCheckout, AppRunDraft, AppRunDraftProcess, AppRunLink, AppRunList, AppRunOpensIn,
    AppRunProcess, AppRunState, NamedAppRun, PinAppRunPort, SaveAppRun, SavedAppRun, UnpinAppRunPort,
};
pub use servers::{
    NamedServer, ServerCheckout, ServerEntry, ServerLink, ServerList, ServerMessage,
    ServerOpened, ServerPhase, ServerPort, ServerState, StartServer, StartedBy,
};
pub use slot_choice::{FreeSlot, LinePlace, MoveInLine, SlotHolder, SlotHolderJob, SlotKeeps};
pub use slot_asking::{
    AskForSlot, AskForSlotAct, SlotAsked, SlotAskStanding,
};
pub use session_checks::{
    CheckBlame, SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckOutputMessage,
    SessionCheckOutputOpened, SessionCheckState, SessionCheckStop,
};
pub use session_retro_row::{
    PartialRetroItem, RetroStage,
};
pub use phone::{
    CloudflareSignIn, CloudflareState, GatewaySetup, Mac, MacColour, MacLink, PairedPhone, PhoneGateway, Served, SetPhoneGateway,
    TailscaleState,
};
pub use mac_acts::{DispatchOn, JobAct, JobMessage, MacAct, Queued, QueuedList, StartOn, TakeBack};
pub use macs::{ClaimMacLink, MacNamed, MacPairLink, OtherMacBugReports, OtherMacEvent, OtherMacProjects, OtherMacSessions};
pub use show_file::{ShowFile, ShownText, ShownVideo};
pub use test_map::{ReachingCheck, ReachingTest, RecordedLap, RecordedLaps, TestsReaching};
pub use document_answer::{
    AnswerDocument, DocumentHow, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_KEPT_AT, DOCUMENT_REVISION, DOCUMENT_SETTLED,
    DOCUMENT_SETTLED_AT, DOCUMENT_WORDS,
};
pub use artifact::{ArtifactBy, ArtifactForm, ArtifactRevision, ArtifactState, JobArtifact};
pub use sketch::{
    page_mocks, page_scene_problems, AnswerSketch, MockNode, RawScene, ShowSketch, Sketch, SketchHow, MOCK_GLYPHS, SKETCH_ASKED, SKETCH_DRAWN,
    SKETCH_SETTLED, SKETCH_SETTLED_AT, SKETCH_WITHDRAWN, SKETCH_WORDS,
};
pub use sketch_scene::{Scene, SceneColor, SceneEdge, SceneHead, SceneKind, SceneLine, SceneNode, ScenePath, ScenePoint, SceneShape, SceneStroke, WirePart, SCENE_ICONS};
pub use sketch_screen::{
    ScreenAct, ScreenAlign, ScreenChrome, ScreenElement, ScreenField, ScreenFrom, ScreenGap, ScreenLayout, ScreenMessage, ScreenPart, ScreenRow, ScreenSize, ScreenSizeWord, ScreenText, ScreenVariant,
};
pub use package_manifest::{read_package_manifest, PackageManifest};
pub use design_catalogue::{
    CatalogueArea, CatalogueAsked, CatalogueComponent, CatalogueContents, CataloguePattern, CatalogueToken, DesignCatalogue, TokenFamily, TokenGroup, TokenRole,
};
pub use style_sources::{KeptFont, KeptSheet, KeptStyles, StyleKind, StyleSource, StyleSources, StyleToken};
pub use session_starts::{
    ForkThisSession, HandToSession, SessionHeld, SessionLink, HeldOrRefused, SessionsHeld, StartAnotherSession,
    StartAnotherSessions, AGENT_START_UNPLACED,
};
pub use started_jobs::{
    StartedAct, StartedAnswer, StartedAnswers, StartedCommand, StartedCommands, StartedJob, StartedResult, StartedSessions, StartedStep,
    STARTED_UNPLACED,
};
pub use session_waits::{
    ChangeWaiting, ReadSessionWait, SessionWait, StartAfter, StartBase, StartCondition,
    StartWaitingNow, WaitBy, WaitPull, WaitReading, WaiterEvent, WaiterSide,
};
pub use session_summary::{
    HeldPullRequest, SessionSummaries, SessionSummary, SessionView, TurnState,
};
pub use sessions::{
    AnswerWaiting, OperatorRead, HeldOpenBy, PullRequestWait, Attachment, AttachmentHistory, AttachmentNamed, AttachmentReport, AttachmentState,
    AttachmentVersion, ClosedWaiting, ClosedWaitingList, ConfirmWaiting, DismissWaiting, DoneWaiting, ReopenWaiting, ClaimPullRequest, Holder, HolderKind, MainRedTaken, Owners, Ownership,
    PullRequestClaimed, NameSession, RenameSession, TakeMainRed, SessionFact, SessionId, SessionList,
    SessionOrigin, ProposedField, SessionProposal, SessionRecord, SessionStarted, StepVisit, SessionReport,
    ConfirmLater, DeferLater, LaterDeferred, LaterHow, LaterSettled, SessionLater, SettleLater,
    SessionStanding, SessionState, SessionStopped, SessionRepairing, FleetAway, StopReason, StopRepair, StopRestart, ToolMissing, ToolsMissing, RunningKind, RunningWork, SessionRunMessage, SessionRunUnderway, StartSessionRun,
    StopSessionRun, WriteSessionRun, ResizeSessionRun, SessionUsage, SetWaitingFor, AllDone,
    FinishUp, FinishedUp, ShowWindow, SignIn, TakeWindowSignIn, origins_as_detail, WindowSignIn,
    WindowTake, AskInWindow, ProposeDraft, WindowPlace, TerminalCommand, TerminalFacts, WaitingAct,
    WaitingActKind, WaitingInput, WaitingItem, WaitingMode, WaitingOption, WaitingSource, OptionPicture, PartPicked, PicturePart, AnswerWaitingParts,
    WalkTold, OpenNativeWalk, NativeWalkOpened, WindowClosed, WindowFailed, WithdrawDocument, WithdrawDraft, WithdrawSketch, WithdrawWindow,
};
pub use needs_you::{
    NeedsYou, NeedsYouAct, NeedsYouDismiss, NeedsYouGlyph, NeedsYouGlyphKind, NeedsYouRow, NeedsYouSection, NeedsYouSectionKind,
};
pub use draft_document::{ConvertDraftToDocument, DraftDocument};
pub use pre_pr::{CheckBeforePr, PrePrCheckRan, PrePrChecked};
pub use terminals::{StartTerminalJob, TerminalJobStarted};
pub use terminals::{
    JobTerminal, JobTerminals, ListTerminals, ListedTerminal, OpenTerminal, ReadTerminal, RenameTerminal, ShowTerminal, StartTerminalRun, TerminalScreen,
    TerminalTyped, TypeInTerminal,
};
pub use job_browser::{
    AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserAskAnswered, BrowserAskNoted, BrowserClick, BrowserError,
    BrowserEvaluate, BrowserLook, BrowserOpen, BrowserRequest, BrowserRequestAnswered, BrowserType, BrowserWaitFor,
    EndBrowserAsk, RaiseBrowserAsk,
};
pub use act_failed::{ActFailed, BackgroundAct};
pub use autopilot::{
    Autopilot, AutopilotCap, AutopilotDecision, AutopilotList, AutopilotPick, AutopilotPickState,
    AutopilotProposal, AutopilotQuoted, AutopilotSettings, AutopilotState, AutopilotWork, ProposeAutopilot,
};
pub use autopilot_decisions::{
    AutopilotDecided, AutopilotDecisions, OverrideAutopilotDecision, ReviewAutopilotDecision,
};
pub use context_breakdown::{
    ContextBreakdown, ContextCategory, ContextKind, ContextMcpTool, ContextMemoryFile,
};
pub use turn_usage::{BashUse, SessionTurnUsage, TurnUsage};
pub use usage_rules::{UsageRule, UsageRuleLimit, UsageRuleTrip, UsageRules};
pub use usage_spent::{SpentBy, SpentJob, SpentOn, SpentSplit, SpentWindow, UsageSpent};
pub use usage_summary::{
    DayUnits, NamedUnits, PriorUsage, RestUsage, SessionUsageSummary, TokenUnits, UsageBreakdown, UsageSummary,
};
pub use lead_picture::LeadPicture;
pub use time_summary::{
    BuildFigures, HourFigures, JobSplit, JobTime, SlowJob, UsageHours, UsageTime, UsualTime,
};
pub use loadout::{
    LoadoutItem, LoadoutItemAsked, LoadoutItemBody, LoadoutItemFormat, LoadoutItems, LoadoutKind, LoadoutSource,
    LoadoutState, LoadoutSummary, LoadoutTurn, LoadoutUse, LoadoutUseKind, SessionLoadout, LOADOUT_ITEM_CAP,
};
pub use session_activity::{
    SessionAccount, SessionActivity, SessionLive, SessionTodo, SessionTodoState, SubagentActivity,
};
pub use session_delivery::{DeliveryChoice, DeliveryPicked, DeliveryPickedFrom, StartHeld};
pub use session_plan::{
    ApprovePlan, CheckedPlan, FileTouch, GateChecks, GateClose, GateHeld, PlanChangeLine, PlanChanges, PlanGate, PlanGroupInput, PlanMark,
    PlanHold, PlanHoldWhy, PlanLandsIn, PlanLandsInPr, PlanLandsInState, PlanMoved, PlanOverlap, PlanRemoved, PlanRenamed, PlanSketch, PlanSketchInput, PlanTaskInput,
    PlanTaskState, RecordPlan, SendPlanChanges, SessionPlan, SessionPlanGroup, SessionPlanTask,
    TaskStale, TouchAct, TouchBy, FIRST_PULL_REQUEST, MOST_LANDS_IN_CHARS, MOST_MARKS, TASK_LIST_IS_THE_PLAN,
};
pub use session_build::{
    BuildPhase, BuildPiece, BuildPieceKind, BuildStop, BuildStopId, ReportedTask,
    ReportedTaskStatus, SessionBuild,
};
pub use model_downshift::{
    ModelDownshift, ModelDownshiftState,
};
pub use agent_sign_in::AgentSignIn;
pub use plan_usage::{
    PlanUsage, PlanWindow,
};
pub use accounts::{
    AccountFlow, Accounts, BoundHow, BoundRepository, Profile, ProfileHome, SetRepositoryProfile, SetUpState,
    SetUpSteps, SignInEnded, SignInOutcome, SignOut, SignedOut, UnmatchedRepository,
};
pub use setup::{
    LeftOutWorkflow, ManifestFromKit, ManifestKitName, ManifestSummary, ModelChoices, ModelsRead, OfferedModel,
    Lands, OverriddenWorkflow, SaveWorkflow, StepPhase,
    WorkflowDefinition, WorkflowSaved, WorkflowScope, WorkflowStep, WorkflowSummary,
};
pub use since::{
    EventTally, EventsSince,
};
pub use slot_found::{
    what_a_scout_found_in_a_slot, SlotFound, MOST_ITEMS, MOST_ITEM_CHARACTERS,
};
pub use studio::{
    AddStudioNode, ContradictionSettled, CreateStudio, DecideStudioEdge, DeferOnStudio,
    EditStudioDraft, EditStudioLink, EpicRead, EpicTake, ForgeState, GroupStudioNodes,
    StudioJobAct, MoveStudioNode, ProposeStudioEdge, RemoveStudioNodes, RenameStudio,
    SettleContradiction, StartStudioRun, StartStudioServer, Studio, StudioDeleted, StudioEdge,
    StudioJobActed, StudioList, StudioNode, StudioNodeContent, StudioPosition, StudioRunHeld,
    StudioRunKept, StudioRunStarted, StudioServerStarted, StudioSummary, WriteUpStudioNode,
};
pub use studio_added::{
    AddedContent, StudioNodeAdded,
};
pub use studio_sketch::{
    EditStudioSketch, SketchBox, SketchDrawing, SketchDrawn, SketchJoin, SketchPicture,
    SketchPictureDrawn, SketchPoint, SketchStroke,
};
pub use watches::{
    collapsed, read_watch_line, watch_line, DeleteWatch, DismissWait, LoginShell, StartWatch, StopWatch, WatchList, WatchPart,
    WatchReading, WatchReadingState, WatchSetAside, WatchStarts, WatchState, WatchSummary,
    ReadWatchLog, WatchEvidence, WatchLog, WatchVerdict, WatchVerdictGiven, WatchVerdictWord, WeighWatch,
};
pub use watch_tools::{
    SaveWatchRecipe, TryWatch, WatchRecipeList, WatchRecipeListed, WatchRecipeSaved,
    WatchRecipeSource, WatchTried, WatchTries,
};
pub use underway::{
    OutputClosed, OutputEnded, OutputLines,
};
pub use protocol_id::ProtocolId;
// The Macs setup that leads him: its one press, the Macs on his account, and pairing one without a link.
pub use phone::{MacSetup, MacSetupStep};
pub use macs::{AskMacPair, FoundMac, FoundMacs};
pub use walk_steps::{
    WalkPoint, WalkStep, WalkTarget,
};
