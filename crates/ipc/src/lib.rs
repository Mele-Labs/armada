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
/// A scout's run, as its Finding records it, and the acts that start and stop one.
/// `#1292`.
mod capturing;
mod checks;
mod codec;
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
/// Whether the repository still has what `armada.yml` names. **A read of the
/// repository**, where `reading` is a read of the file — a `run` line naming a
/// deleted script parses perfectly and says nothing.
mod drift;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with, where `reading` is the half that reports.
mod editing;
mod enums;
mod error;
mod event;
/// What `search_files` found under the checkout, for the `@` mention popup.
mod files;
/// What Fleet can say about its own health, and what it cannot. **Not
/// Doctor**, whose grid is ten modules and is not built.
mod health;
/// One repository's Helm conversation, and the socket its replies come on.
mod helm;
/// A call a Helm session made that its person's settings do not cover.
mod helm_call;
/// One repository's Helm session as one quotable record. `#1367`.
mod helm_debug;
/// Helm reading a thought for where it belongs, and recording where it went.
mod thought;
/// What happened across the Fleet and when, for the Cockpit's timeline.
mod timeline;
/// Who held a slot and what was checked out in it, for the side panel's Bay.
mod bay_history;
/// What Fleet is holding disk for, and the test each one did not pass.
/// **A piloted worktree is not on this wire at all** — `#367`.
mod holding;
/// A session Bridge hosts: its commands, its thread and the gate its first
/// write goes through. `docs/concepts/session.md`.
mod hosted_sessions;
mod ids;
/// The Cockpit's flight controls a person arranged in Kit, and a shell action run from one.
mod flight_controls;
/// A repository's open issues, as Fleet read them. #91.
mod issues;
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
/// Who asked for a Check run, as a typed value. Since 23.40.
mod requester;
/// What an agent says about one pull request, and the review tools it is asked with.
mod review_runs;
/// Natural Search, and where a change runs for the Walk tool.
mod review_walks;
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
/// A Command that stays running, held by Fleet. **Lifecycle on `/events`,
/// output on a socket of its own.**
mod servers;
/// The session ledger: what a harness reports of a session, and what Fleet
/// answers. `docs/concepts/session.md`.
mod session_checks;
/// A note pinned on a page, sent to a Session whole. `send_session_note`, `send_session_notes`.
mod session_notes;
mod slot_asking;
mod session_retro_row;
mod session_starts;
mod session_waits;
mod session_summary;
mod run_memory;
mod session_import;
mod sessions;
mod setup;
mod sketch;
/// Autopilot: a repository working its own backlog. #2092.
mod autopilot;
/// Sleep mode: the switch and the night it keeps.
mod sleep;
/// How much of the plan's windows is used.
mod plan_usage;
/// The default model stepping down while the weekly window runs out.
mod model_downshift;
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
/// What a hosted Session is doing now, as Fleet saw it.
mod session_activity;
/// What a Session has built: its phase, pieces and road, read off what Fleet observed.
mod session_build;
mod session_plan;
/// How a Job's work is to be delivered, and the one thing that is asked about it.
mod session_delivery;
/// A Session's workflow rail and its judges: the step it is on, and the concerns a judge raises.
mod session_rail;
/// What crossed the stream since a cursor, counted rather than carried.
/// **An agent's substitute for the socket it cannot hold.**
mod since;
mod slot_found;
/// A Studio, its nodes and edges, and the acts a client asks of one. `#1285`.
mod studio;
/// What `add_studio_node` carries: any content, a Picture staged rather than kept.
mod studio_added;
mod studio_sketch;
mod triggers;
/// The lines a running log carries over its socket, and why one stopped arriving.
mod underway;
mod walk_steps;

#[cfg(test)]
mod tests;

pub use annotations::{
    AnnotationMarked, MarkAnnotationDone,
};
pub use amending::{
    CheckDraft, CommandDraft, EditManifest, EvidenceDraft, LinkDraft, ManifestDeclared,
    ManifestEdit, ManifestEdited, NamedCheck, NamedCommand, NamedPort, NarrowingDraft,
    PolicyWords, PortDraft,
};
pub use building::{
    BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildChanging,
    FleetBuildReport,
};
pub use capturing::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureServed, CaptureStudioNote,
    CaptureWindow, StagedFrame, StudioCapture,
};
pub use checks::{
    CheckOutput, CheckRun, DeclaredCheck, DeclaredJudge,
};
pub use codec::{
    decode, encode, Undecodable, Unencodable,
};
pub use commanding::{
    AllowedCommandRow, RemoveRepositoryAllowedCommand, RepositoryAllowedCommands,
};
pub use configured::ManifestConfig;
pub use drift::{
    Declaration, Drift, ManifestDrift, PackageScripts, Unfollowed,
};
pub use editing::{
    ManifestFile, ManifestSaved, SaveManifestFile,
};
pub use enums::{
    Actor, AdvanceGate, CheckOutcome, EvidenceType, LandsIn, LessonState, ManifestReach, PrMode,
    ReachesDrones, ScoutSourceKind, StudioAuthor, StudioEdgeKind, StudioEdgeStanding,
    StudioNodeKind, StudioNodeState, StudioRelation, Via, Whose,
};
pub use error::{
    RunId, WireError, WireValue,
};
pub use event::{
    Cursor, Delivered, Event, Missed, Resync, StreamMessage,
};
pub use files::FilesFound;
pub use health::{
    FleetHealth, HelmActionAuthority, Probe, Unprobed,
};
pub use helm::{
    AskHelm, Blank, Freshness, HelmAsked, HelmContext, HelmFresh, HelmScreen, HelmSession,
    HelmText, HelmUnanswered,
};
pub use thought::{
    HelmThoughtRouted, ReadThought, TellSession, ThoughtKind, ThoughtReading, ThoughtSent,
    ThoughtTarget,
};
pub use bay_history::{
    BayHistory, BayHistoryAsk, BayHistoryEntry, BayHolder, BayHolderKind, MOST_BAY_HISTORY,
};
pub use timeline::{
    FleetTimeline, ServiceCall, TimelineAppended, TimelineAsk, TimelineEvent, TimelineKind,
    TimelineSubject, TimelineTone, DEFAULT_TIMELINE_LIMIT, MOST_TIMELINE_ITEMS, MOST_TIMELINE_LIMIT,
};
pub use helm_call::{
    AnswerHelmCall, AskedChoice, AskedQuestion, AskingToRun, HelmAskingToRun, HelmCallAnswer,
    HelmCallAnswered, HelmCallInFlight, HelmCallSettled, HelmCallsWaiting, QuestionAnswer,
    ReachesTo, RunOrNot, handoff_figure, ASKS_A_QUESTION, HANDOFF_QUIET, HANDOFF_REMIND, HANDOFF_REMIND_AT,
    HANDOFF_REMIND_QUESTION, HANDOFF_STAY, HANDOFF_YES,
};
pub use helm_debug::{
    HelmDebugInfo, HelmDebugLine, HelmDebugSaid, HelmDebugText,
};
pub use holding::{
    ReclaimedSaved, ChangeSlotPool, CommitHome, RescueAct, RescueSlot, SlotAct, SlotCommit,
    SlotFinding, SlotFindingState, SlotHolderKind, SlotHolding, SlotLastHolder, SlotPoolChanged,
    SlotReleased, SlotRescued, SlotStranded, SlotVerdict, WorktreeSlot, WorktreesHeld,
};
pub use hosted_sessions::{
    AnswerSessionAsk, ChosenHold, CloseSession, ForkFrom, GateAnswer, GateHold, HeldCommand,
    HoldKept, HoldMoved, HostedFacts, LinkedEvent, MessagesHeld, MoveSession,
    ReleaseSessionHolds, RunSessionCommand, SendSessionMessage, SentFile, SessionAskState,
    SessionGate, SessionHoldsReleased, SessionMode, SessionRow, SessionRowChanged, SessionTag,
    SetSessionWorkflow, SessionSubagent, SessionThread, SessionTurn, SessionUpload,
    SessionVoice, SessionVoiceNamed, StartSession, StopSession, RetryHandoff, PauseSession, ResumeSession, SessionPaused, SetSessionAutoLand, TagKind, TakeHeld, TerminalAsk,
    TerminalAsked, TuneSession,
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
pub use issues::{
    IssueComment, IssueList, IssueRead, IssueState, LinkedPullRequest, LinkedPullRequestState,
    TrackedIssue,
};
pub use kit::{
    AddKitServer, ForgetKitServer, KitAllowedCommand, KitAllowedCommands, KitAllowedSource,
    KitInventory, KitServerRow, KitServers, OutputStyleFacts, RemoveKitAllowedCommand,
    ServerAddress, SetKitServerReach, SetManifestServerReach, SetupItem, SetupKindRow,
    SetupUnreadable, WhatWasRead,
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
    ModChecked, ModKind, ModList, ModPromoted, ModScaffolded, ModSummary, PromoteMod,
    ScaffoldMod, SetModEnabled,
};
pub use clashes::{
    ClashFile, ClashHolder, ClashSide, WriteClash, WriteClashes,
};
pub use territory::{
    AreaKid, AreasOfRepository, HolderTouches, RepositoryArea, RepositoryAreas, SlotTouches,
    TouchesInRepository,
};
pub use preferences::{
    Preferences, SavePreference, DEFAULT_THEME,
};
pub use settings::{
    SaveSettings, Setting, SettingApplies, SettingKind, SettingValue, SettingsList,
    SettingsRefused,
};
pub use pull_requests::{
    FailureWeight, ForgeChecks, PullRequestCheckRow, PullRequestCheckState, PullRequestCommit,
    PullRequestEntry, PullRequestFile, PullRequestFileOf, PullRequestFilePatch,
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
pub use repositories::{
    AddRepository, CloneRepository, CreateProject, RepositoryList, RepositorySummary, RequestRepository,
};
pub use requester::Requester;
pub use run_memory::{
    DropRunFact, KeptOnThisMac, RememberedRun, RunCloud, RunFactState, RunMemory, RunMemoryPlace, RunPlace, RunWorkspace, SetKeptOnThisMac, SetRunMemoryPlace,
};
pub use session_import::{
    ImportFolder, ImportFolders, ImportGit, ImportRuns, InstallAgentMod, ModInstallStep, ModInstalled, RunFact,
    RunFactKind,
};
pub use review_runs::{
    PullReviewHolder, ReviewChange, ReviewFinding, ReviewImpact, ReviewLink, ReviewPlace,
    ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, ReviewRuns, ReviewSeverity,
    ReviewStep, ReviewStepSource, ReviewTheme, ReviewTool, ReviewToolKind, ReviewToolLeftOut,
    ReviewToolScope, ReviewToolSource, ReviewTools, SaveReviewTool, StartReviewRun,
};
pub use review_walks::{
    ForgetReviewSearch, OpenReviewWalk, RememberedPlace, RememberedPlaces, ReviewMatch, ReviewRequest, ReviewRequestMethod,
    ReviewWalk, ReviewWalkState, SaveWalkPlace, StartReviewSearch, WalkHow, WalkPlace, WalkPlaceFound,
    WalkSource, WalkTried, SEARCH_RUN_SCOPE,
};
pub use retro::{
    Lesson, Lessons, RecordAct, RecordAsked, RecordRefusal, RecordSaid, Retro, RetroAnswered,
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
pub use servers::{
    NamedServer, ServerCheckout, ServerEntry, ServerLink, ServerList, ServerMessage,
    ServerOpened, ServerPhase, ServerPort, ServerState, StartServer, StartedBy,
};
pub use slot_asking::{
    AskForSlot, AskForSlotAct, SlotAsked, SlotAskStanding,
};
pub use session_checks::{
    SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckOutputMessage,
    SessionCheckOutputOpened, SessionCheckState, SessionCheckStop,
};
pub use session_retro_row::{
    PartialRetroItem, RetroStage,
};
pub use sketch::{
    RawScene, ShowSketch, Sketch,
};
pub use session_starts::{
    ForkThisSession, HandToSession, SessionHeld, SessionLink, StartAnotherSession,
};
pub use session_waits::{
    ChangeWaiting, ReadSessionWait, SessionWait, StartAfter, StartBase, StartCondition,
    StartWaitingNow, WaitBy, WaitPull, WaitReading, WaiterEvent, WaiterSide,
};
pub use session_summary::{
    HeldPullRequest, SessionSummaries, SessionSummary, SessionView, TurnState,
};
pub use sessions::{
    AnswerWaiting, Attachment, AttachmentNamed, AttachmentReport, AttachmentState,
    DismissWaiting, ClaimPullRequest, Holder, HolderKind, MainRedTaken, Owners, Ownership,
    PullRequestClaimed, RenameSession, TakeMainRed, SessionFact, SessionId, SessionList,
    SessionOrigin, ProposedField, SessionProposal, SessionRecord, SessionReport,
    SessionStanding, SessionState, SessionRunMessage, SessionRunUnderway, StartSessionRun,
    StopSessionRun, WriteSessionRun, ResizeSessionRun, SessionUsage, SetWaitingFor, AllDone,
    FinishUp, FinishedUp, ShowWindow, SignIn, TakeWindowSignIn, origins_as_detail, WindowSignIn,
    WindowTake, AskInWindow, WindowPlace, TerminalCommand, TerminalFacts, WaitingAct,
    WaitingActKind, WaitingInput, WaitingItem, WaitingMode, WaitingOption, WaitingSource,
    WalkTold, WindowClosed, WindowFailed, WithdrawWindow,
};
pub use autopilot::{
    Autopilot, AutopilotCap, AutopilotList, AutopilotPick, AutopilotPickState,
    AutopilotProposal, AutopilotQuoted, AutopilotSettings, AutopilotState, ProposeAutopilot,
};
pub use context_breakdown::{
    ContextBreakdown, ContextCategory, ContextKind, ContextMcpTool, ContextMemoryFile,
};
pub use turn_usage::{BashUse, SessionTurnUsage, TurnUsage};
pub use usage_rules::{UsageRule, UsageRuleLimit, UsageRuleTrip, UsageRules};
pub use usage_summary::{
    DayUnits, NamedUnits, PriorUsage, RestUsage, SessionUsageSummary, TokenUnits, UsageBreakdown, UsageSummary,
};
pub use loadout::{
    LoadoutItem, LoadoutItemAsked, LoadoutItemBody, LoadoutItemFormat, LoadoutItems, LoadoutKind, LoadoutSource,
    LoadoutState, LoadoutSummary, LoadoutTurn, LoadoutUse, LoadoutUseKind, SessionLoadout, LOADOUT_ITEM_CAP,
};
pub use session_activity::{
    SessionActivity, SessionTodo, SessionTodoState,
};
pub use session_delivery::DeliveryChoice;
pub use session_plan::{
    ApprovePlan, CheckedPlan, FileTouch, PlanChangeLine, PlanChanges, PlanGroupInput, PlanMark,
    PlanHold, PlanHoldWhy, PlanMoved, PlanRemoved, PlanRenamed, PlanSketch, PlanSketchInput, PlanTaskInput,
    PlanTaskState, RecordPlan, SendPlanChanges, SessionPlan, SessionPlanGroup, SessionPlanTask,
    TouchAct, TouchBy, MOST_MARKS, TASK_LIST_IS_THE_PLAN,
};
pub use session_rail::{
    ConcernFrom, ConcernHow, ConcernSettled, DismissConcern, JudgeConcern, JudgeReading,
    JudgeSession, WaitingCite, SendConcern,
};
pub use session_build::{
    BuildPhase, BuildPiece, BuildPieceKind, BuildStop, BuildStopId, ReportedTask,
    ReportedTaskStatus, SessionBuild,
};
pub use model_downshift::{
    ModelDownshift, ModelDownshiftState,
};
pub use plan_usage::{
    PlanUsage, PlanWindow,
};
pub use accounts::{
    AccountFlow, Accounts, BoundHow, BoundRepository, Profile, SetRepositoryProfile, SetUpState,
    SetUpSteps, SignInEnded, SignInOutcome, SignOut, SignedOut, UnmatchedRepository,
};
pub use sleep::{
    OverrideSleep, ReviewSleep, SetSleep, SleepBlocked, SleepDecided, SleepLanded, SleepState,
    SleepWalk,
};
pub use setup::{
    LeftOutWorkflow, ManifestFromKit, ManifestKitName, ManifestSummary, ModelChoices, ModelsRead, OfferedModel,
    OverriddenWorkflow, SaveWorkflow, StepPhase,
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
    HelmStudioAct, MoveStudioNode, ProposeStudioEdge, RemoveStudioNodes, RenameStudio,
    SettleContradiction, StartStudioRun, StartStudioServer, Studio, StudioDeleted, StudioEdge,
    StudioHelmActed, StudioList, StudioNode, StudioNodeContent, StudioPosition, StudioRunHeld,
    StudioRunKept, StudioRunStarted, StudioServerStarted, StudioSummary, WriteUpStudioNode,
};
pub use studio_added::{
    AddedContent, StudioNodeAdded,
};
pub use studio_sketch::{
    EditStudioSketch, SketchBox, SketchDrawing, SketchDrawn, SketchJoin, SketchPicture,
    SketchPictureDrawn, SketchPoint, SketchStroke,
};
pub use triggers::{
    LeftOutTrigger, OverriddenTrigger, RemoveTrigger, SaveTrigger, TriggerDefinition,
    TriggerLevel, TriggerList, TriggerMoment, TriggerRemoved, TriggerRuns, TriggerSaved,
    TriggerScope, TriggerSkip, TriggerSkipReason, TriggerSummary,
};
pub use underway::{
    OutputClosed, OutputEnded, OutputLines,
};
pub use protocol_id::ProtocolId;
pub use walk_steps::{
    WalkPoint, WalkStep, WalkTarget,
};
