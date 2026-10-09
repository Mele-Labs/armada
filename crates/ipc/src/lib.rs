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

/// What is waiting on a person, in two buckets. **Derived from the Jobs Fleet
/// holds, never stored** — which is why nothing publishes `alert.raised`.
mod alerts;
/// A form's edits to `armada.yml`, as edits — the half of Journey 9's *Editing*
/// that changes only the lines it touches.
mod amending;
/// What `ask_person_to_approve` answers with. **Nothing moves**, so the Job's
/// own facts are drawn again at render time and never frozen here.
mod approval_ask;
/// What a person decides about a Job before it runs: the approval body,
/// `edit_job`, a criterion's origin and the branch list. Spike 022, slice 4.
mod approving;
/// A Drone's own run of a step's Checks, as a row of its own. Since 23.40.
mod asked;
/// How many times a step was worked, and what each run came to. **The record
/// held it and nothing served it** — see the module.
mod asking;
mod attempt;
mod breakage;
/// The build Fleet runs on, where it stands against `main`, and the request that
/// moves it. **Fleet-wide, and not a Job's field.**
mod building;
/// How many Drones Fleet may run, how many it is running, and what holds the
/// next one back. **Fleet-wide, and not a Job's field.**
mod capacity;
/// A scout's run, as its Finding records it, and the acts that start and stop one.
/// `#1292`.
mod capturing;
mod checks;
mod codec;
/// A command a Drone was not given, and what a person answers about it. **Two
/// paths to one answer** — while the Drone waits, or after the Job stopped.
mod commanding;
/// One Manifest as Fleet resolved it, past the summary a picker reads.
mod configured;
mod detail;
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
/// The processes Fleet is holding, and what one of them has been doing.
/// **Read off the roster, never off the Jobs.**
mod drones;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with, where `reading` is the half that reports.
mod editing;
mod enums;
mod error;
mod event;
/// What a cheap model said one command a Drone reached for does, for the
/// person deciding whether to allow it. **It decides nothing** — the offers
/// are the same whether or not anybody asks.
mod explaining;
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
mod history;
/// What Fleet is holding disk for, and the test each one did not pass.
/// **A piloted worktree is not on this wire at all** — `#367`.
mod holding;
/// A session Bridge hosts: its commands, its thread and the gate its first
/// write goes through. `docs/concepts/session.md`.
mod hosted_sessions;
mod ids;
mod job;
/// What Fleet did to a Job, out of the Job's own log. **The third voice the
/// activity log was designed around and nothing produced.**
mod journal;
mod judged;
/// Kit's MCP servers and each Manifest's word over one (`#1275`), and the
/// setup a person already works with, read to be shown (`#1491`).
mod kit;
/// Fleet's three changeable limits. **A value out of range does not decode.**
mod limits;
/// The merge line of each served repository, as Fleet serves it.
/// Every Check run one repository's Jobs asked for or ran. Since 23.40.
mod manifest_checks;
/// A possible `armada.yml` per workspace, and the edits and Write that finish it.
mod manifest_proposal;
/// The Evidence tool's transport. **A different seam** — Fleet to Drone, not
/// Fleet to Bridge — so it is a module rather than a flat re-export and none of
/// its types are in `operations/`.
pub mod mcp;
mod merge_hub;
mod merge_line;
/// `layout.json`: what a layout mod or the owner's own choice may say. `docs/concepts/layout-mods.md`.
pub mod layout;

/// The owner's key bindings: which keys an act answers in place of the registry's. `docs/concepts/key-bindings.md`.
pub mod key_bindings;
/// Mods: a directory on this machine that changes how Bridge looks.
mod mods;
/// `armada need`: a checkout says what it needs on a path, and Fleet answers from
/// the session ledger. `docs/capabilities/needs.md`.
mod needs;
/// Where two Jobs claim the same paths. **A fact on the card, never a
/// verdict** — nothing in it is readable as a refusal.
mod overlap;
/// Taking a Job over and the bundle a person is handed. `docs/concepts/pilot.md`.
mod piloting;
/// A person's Bridge preferences, kept the way `limits` are kept. **A value
/// out of the closed set does not save**, `limits`' reason one field over.
mod preferences;
mod proposing;
/// Which wire a build speaks: a hash `build.rs` takes of the wire surface.
mod protocol_id;
/// A pull request, by repository and number, and the acts a Session takes on one.
/// Since 23.48.
mod pull_requests;
/// A new cost ceiling for one Job, and which surface asked for it.
mod raising;
/// Fleet's last read of `armada.yml`, held rather than announced. **The one
/// shape here about the fleet and not about a Job**, beside `capacity`.
/// What a scout answers a read-in with. `#1293`.
mod read_in;
mod reading;
/// What giving one Job's worktree and branch back did, half by half.
/// **Two halves, because half of it happening is a real outcome.**
mod reclaimed;
/// A person's run of one Manifest entry, in a Job's worktree or in the main
/// checkout. **A rehearsal, never a verdict** — nothing in it is a Check row
/// or Evidence.
mod rehearsal;
/// What people wrote on a Job's pull request, and which of it a person picks.
/// **The one place this seam carries text from outside this machine.**
mod remarks;
/// What a person says went wrong, with the Job's own record attached.
mod report;
/// The repositories one Fleet serves, and adding one by folder.
mod repositories;
/// Who asked for a Check run, as a typed value. Since 23.40.
mod requester;
/// What one Job holds on this machine, and what came of asking whether it is
/// working. **The other axis from `spend`**, which answers the model's cost.
mod resources;
/// A Job's retro and the record it was read from, and the Lessons listing.
/// Since 23.12.
mod retro;
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
mod sessions;
mod setup;
/// What a step's harness produced, as a client is told about it.
mod showing;
/// Sleep mode: the switch and the night it keeps.
mod sleep;
/// What crossed the stream since a cursor, counted rather than carried.
/// **An agent's substitute for the socket it cannot hold.**
mod since;
mod slot_found;
/// A Studio, its nodes and edges, and the acts a client asks of one. `#1285`.
mod studio;
/// What `add_studio_node` carries: any content, a Picture staged rather than kept.
mod studio_added;
mod studio_sketch;
/// The Triggers a repository runs, saving one, and what a Job did with each.
/// Since 23.58.
mod added_steps;
mod trigger_holds;
mod triggers;
mod turn;
/// A step's Checks while the gate is running them, and the socket a running
/// Check's log is read over.
mod underway;
/// What the fleet has spent, against the ceilings that refuse the next Drone.
mod usage;
/// What is outstanding on a live Drone, and what a person sends it back.
mod waiting;
/// What a person said while walking a Prototype's mock, kept on the Job.
/// Since 23.18.
mod walk_notes;
/// The material a reviewing person reads, and what their note carries.
mod work;
/// A Job's plan and its tasks. **Not `work`'s `DeclaredPlan`**, which is where a
/// step said its work would be.
mod work_plan;

#[cfg(test)]
mod tests;

pub use alerts::{Alert, AlertList};
pub use amending::{
    CheckDraft, CommandDraft, EditManifest, EvidenceDraft, LinkDraft, ManifestDeclared,
    ManifestEdit, ManifestEdited, NamedCheck, NamedCommand, NamedPort, NarrowingDraft, PolicyWords,
    PortDraft,
};
pub use approval_ask::AskedApproval;
pub use approving::{
    moved_at, ApproveDispatch, ApproveWave, BranchRow, Branches, CompleteWhen, CriterionOrigin,
    CriterionWritten, DispatchSettings, EditJob, Effort, GateChoice, LandingChoice, LandingRule,
    LandingUnit, LandsWhen, PolicyOverrides, SetLandingTarget, StepTuning, ToProposer,
};
pub use asked::{AskedRun, AskedRunState};
pub use asking::{JudgeAnswer, JudgeAnswered, JudgeQuestion, SetWhenRefused, WhenRefused};
pub use attempt::{ended_at, first_started_at, Move, ResolvedPolicies, StepAttempt};
pub use breakage::{ClaimedBreakage, WaitingOnFix};
pub use building::{BuildPosition, BuildSource, BuildStage, ChangeFleetBuild, FleetBuildChanging, FleetBuildReport};
pub use capacity::{AdmissionHold, FleetCapacity};
pub use capturing::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureServed, CaptureStudioNote, CaptureWindow,
    StagedFrame, StudioCapture,
};
pub use checks::{CheckOutput, CheckRun, DeclaredCheck, DeclaredJudge};
pub use codec::{decode, encode, Undecodable, Unencodable};
pub use commanding::{
    AllowedCommandRow, AlwaysAllowScope, AnswerCommand, CommandAnswer, CommandInFlight, Reach,
    RemoveAllowedCommand, RemoveRepositoryAllowedCommand, RepositoryAllowedCommands, SetModel,
    SetWhenBlocked, WhenBlocked,
};
pub use configured::ManifestConfig;
pub use detail::{
    AreaRow, ChangedTestRow, DismissedRow, FindingDismissed, FindingQueued, FindingRow,
    FollowedRow, IssueFiled, JobConfidence, OpenedBecause, ProvesRow, Says, TestsSection,
    UntestedRow,
};
pub use detail::{
    Criterion, Currency, Dependency, FromStudio, JobDelivery, JobDetail, JobReview, JobSpend,
    JudgeInFlight, PullRequestChecks, PullRequestDetail, Refusal, ReplacedBy, Replaces, ReviewedBy,
    Settled, StepDetail, StepFacts, StepPass, Stuck, Verdict, WaveRound,
};
pub use drift::{Declaration, Drift, ManifestDrift, PackageScripts, Unfollowed};
pub use drones::{DroneDetail, DroneList, DroneState, DroneSummary, JobDrone, JobDrones};
pub use editing::{ManifestFile, ManifestSaved, SaveManifestFile};
pub use enums::{
    Actor, AdvanceGate, BudgetHold, CheckOutcome, CriterionSource, DependencyDirection,
    DronePresence, EvidenceType, GroupState, JobStatus, JudgeVerdict, LandsIn, LessonState,
    ManifestReach, Origin, PrMode, QueuedReason, ReachesDrones, Recourse, Resumption,
    ScoutSourceKind, Side, StepState, StudioAuthor, StudioEdgeKind, StudioEdgeStanding,
    StudioNodeKind, StudioNodeState, StudioRelation, TaskState, TaskTier, TopLevelOrigin, Urgency,
    Via, Whose,
};
pub use error::{RunId, WireError, WireValue};
pub use event::{
    ChangeKind, ChangedFile, Cursor, Delivered, DroneExited, DroneSpawned, Event,
    EvidenceSubmitted, JobAskedRun, JobAsking, JobChecking, JobCommandWaiting, JobCreated,
    JobFilesChanged, JobJudging, JobLanded, JobPaused, JobRemarksChanged, JobResumed,
    JobStateChanged, JobStepAdvanced, Missed, ProposalMoved, Reason, Resync, StreamMessage,
};
pub use explaining::CommandExplained;
pub use files::FilesFound;
pub use health::{FleetHealth, HelmActionAuthority, Probe, Unprobed};
pub use helm::{
    AskHelm, Blank, Freshness, HelmAsked, HelmChangedCheckout, HelmClosed, HelmContext,
    HelmConversation, HelmFresh, HelmMessage, HelmOpened, HelmScreen, HelmSilence, HelmText,
    HelmUnanswered,
};
pub use helm_call::{
    AnswerHelmCall, AskedChoice, AskedQuestion, AskingToRun, HelmAskingToRun, HelmCallAnswer,
    HelmCallAnswered, HelmCallInFlight, HelmCallSettled, HelmCallsWaiting, QuestionAnswer,
    RunOrNot, ASKS_A_QUESTION,
};
pub use helm_debug::{HelmDebugInfo, HelmDebugLine, HelmDebugSaid, HelmDebugText};
pub use history::{DroneMoved, JobHistory, Movement, Recorded, StatusMoved, StepMoved};
pub use holding::{
    ChangeSlotPool, CommitHome, HeldReason, RescueAct, RescueSlot, SlotAct, SlotCommit,
    SlotFinding, SlotFindingState, SlotHolding, SlotPoolChanged, SlotReleased, SlotRescued,
    SlotStranded, SlotVerdict, WorktreeHeld, WorktreeSlot, WorktreesHeld,
};
pub use hosted_sessions::{
    AnswerSessionAsk, CloseSession, ForkFrom, GateAnswer, GateHold, HandoffPlan, HandoffStep,
    HeldCommand, HostedFacts, MessagesHeld, PilotFrom, SendSessionMessage, SentFile,
    SessionAskState, SessionGate, SessionMode, SessionRow, SessionRowChanged, SessionTag,
    SessionSubagent, SessionThread, SessionTurn, SessionUpload, SessionVoice, SessionVoiceNamed, StartSession,
    TagKind, TaggedJob, TakeHeld, TerminalAsk, TerminalAsked, TuneSession,
};
pub use ids::{
    CriterionId, DroneId, Instant, JobId, ManifestId, ProposalId, QuestionId, StepId, StudioEdgeId,
    StudioId, StudioNodeId, WorkflowId,
};
pub use job::{
    AttachmentRef, DependencyEdge, JobForgotten, JobList, JobRequest, JobSummary, Paused,
    ProposeJob, ProposedCriterion, ProposedPlan, Redirection, Redispatched, RestartRequested,
    Subject, UnreadableJob,
};
pub use journal::{
    JobLog, JournalClosed, JournalMessage, JournalOpened, LogNote, NoteLevel, NotedField, Quiet,
};
pub use judged::{Citation, CitedAt, Cleared, Flagged, Given, Judged, KeptDeliverable};
pub use kit::{
    AddKitServer, ForgetKitServer, KitAllowedCommand, KitAllowedCommands, KitAllowedSource,
    KitInventory, KitServerRow, KitServers, RemoveKitAllowedCommand, ServerAddress,
    SetKitServerReach, SetManifestServerReach, SetupItem, SetupKindRow, SetupUnreadable,
    WhatWasRead,
};
pub use limits::{
    ChecksAtOnce, DiskFloorGib, DronesAtOnce, FleetLimits, LimitValues, MemorySparePercent,
    SaveLimits, Within,
};
pub use manifest_checks::{
    ManifestCheckLog, ManifestCheckRow, ManifestChecks, SOURCE_ASKED_RUN, SOURCE_GATE,
};
pub use manifest_proposal::{
    Band, EditManifestProposal, ManifestProposal, ManifestProposals, PolicyKey, ProposalEdit,
    ProposedCheck, ProposedCommand, ProposedId, ProposedPolicy, ProposedPort, ProposedRunner,
    ProposedSetup, Provenance, StatedCaps, WriteManifestProposal,
};
pub use merge_hub::{
    FixMain, FixesMain, FixesMainState, HubJob, HubMerged, HubPullCi, HubPullRequest, HubQueue,
    HubQueueState, MainChecking, MainCiState, MainFailedJob, MainMerge, MainRun, MainRunState,
    MainStanding, MergeLineHub,
};
pub use merge_line::{
    LandCheckState, LandOutputMessage, LandOutputOpened, LandState, MergeLine, MergeLineCheck,
    MergeLineEntry, MergeLinePullRequest, MergeLines,
};
pub use mods::{
    ModChecked, ModKind, ModList, ModPromoted, ModScaffolded, ModSummary, PromoteMod, ScaffoldMod,
    SetModEnabled,
};
pub use needs::{NeedAct, NeedAnswer, NeedCall, NeedLine, NeedList};
pub use overlap::{ScopeOverlap, SharedPath};
pub use piloting::{
    DroneNarrative, HandoffBundle, HandoffWorktree, PilotNote, PilotOutcome, Piloted, StoppedOn,
    TakeOver,
};
pub use preferences::{Preferences, SavePreference, DEFAULT_THEME};
pub use proposing::{
    ProposalInFlight, ProposalReach, ProposalSettings, ProposalSettled, ProposalStopped,
    StopProposal,
};
pub use pull_requests::{
    ForgeChecks, PullRequestStanding, PullRequestState, ReviewDispatched, ReviewPullRequest,
};
pub use raising::{CapRaise, RaisedBy, TurnRaise};
pub use read_in::{
    what_a_scout_read_in, ReadIn, ReadInCluster, ReadInContradiction, ReadInNote, ReadInRelation,
    MOST_CHARACTERS, MOST_CLUSTERS, MOST_CONTRADICTIONS, MOST_NOTES, MOST_RELATIONS,
};
pub use reading::{ManifestFault, ManifestMoved, ManifestReading, ManifestRefused};
pub use reclaimed::{
    BranchDeleted, DeleteBranch, ReclaimedBranch, ReclaimedSaved, ReclaimedWorktree,
    WorktreeReclaimed,
};
pub use rehearsal::{
    CheckoutRunDiff, CheckoutRunList, CheckoutRunMessage, CheckoutRunOpened, CheckoutRunRecord,
    CheckoutRunSheet, CheckoutRunUnderway, DiffAgainst, NamedRun, RunDiffReading, RunEntry,
    RunList, RunMessage, RunOpened, RunOutput, RunRecord, RunSheet, RunUnderway, StartCheckoutRun,
    StartRun, UnreadableRun, WorkspaceCommands,
};
pub use rehearsal::{
    CheckoutVerify, StartCheckoutVerify, VerifyGroup, VerifyStep, VerifyStepState,
};
pub use remarks::{InlineContext, JobRemarks, Remark, RemarksTakenUp};
pub use report::{Calibration, Claim, FileReport, Report, ReportId, ReportList, ReportOrigin};
pub use repositories::{AddRepository, CloneRepository, RepositoryList, RepositorySummary};
pub use requester::Requester;
pub use resources::{
    Asked, BriefContents, Finding, Held, JobExamined, JobProcess, JobResources, LogFile, LogKind,
    Look, WorktreeOnDisk,
};
pub use retro::{
    AnnotationFile, CheckRunBy, JobRetro, Lesson, Lessons, LinkedAnnotation, RecordAct,
    RecordAsked, RecordCheck, RecordNotMet, RecordPath, RecordRefusal, RecordSaid, RecordWaited,
    RetroAnswered, RetroChange, RetroChangeAnswered, RetroItem, RetroRecord, RetroSession,
    RetroState, RetroWritten,
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
pub use seeding::{DeclaredSeed, SeedWarmth, WorktreeSeeding};
pub use servers::{
    NamedServer, ServerCheckout, ServerEntry, ServerLink, ServerList, ServerMessage, ServerOpened,
    ServerPhase, ServerPort, ServerState, StartServer, StartedBy,
};
pub use sessions::{
    AnswerWaiting, Attachment, AttachmentNamed, AttachmentReport, AttachmentState, DismissWaiting,
    ClaimPullRequest, Holder, HolderKind, Owners, Ownership, PullRequestClaimed, RenameSession,
    SessionFact, SessionId, SessionList, SessionOrigin, SessionRecord, SessionReport, SessionState,
    SessionUsage, SetWaitingFor, ShowWindow, TerminalCommand, TerminalFacts, WaitingAct,
    WaitingActKind, WaitingInput, WaitingItem, WaitingMode, WaitingOption, WaitingSource,
};
pub use sleep::{OverrideSleep, SetSleep, SleepBlocked, SleepDecided, SleepLanded, SleepState, SleepWalk};
pub use setup::{
    LeftOutWorkflow, ManifestSummary, ModelChoices, OverriddenWorkflow, SaveWorkflow, StepPhase,
    WorkflowDefinition, WorkflowSaved, WorkflowScope, WorkflowStep, WorkflowSummary,
};
pub use showing::{KeptFrame, NamedSpec, ShowAgain, ShownAgain, ShownSet, SpecPicked};
pub use since::{EventTally, EventsSince};
pub use slot_found::{what_a_scout_found_in_a_slot, SlotFound, MOST_ITEMS, MOST_ITEM_CHARACTERS};
pub use studio::{
    AddStudioNode, ContradictionSettled, CreateStudio, DecideStudioEdge, DeferOnStudio,
    DispatchStudioDraft, EditStudioDraft, EditStudioLink, EpicRead, EpicTake, ForgeState,
    GroupStudioNodes, HelmStudioAct, MoveStudioNode, ProposeStudioEdge, RemoveStudioNodes,
    RenameStudio, SettleContradiction, StartStudioRun, StartStudioServer, Studio, StudioDeleted,
    StudioEdge, StudioHelmActed, StudioList, StudioNode, StudioNodeContent, StudioPosition,
    StudioRunHeld, StudioRunKept, StudioRunStarted, StudioServerStarted, StudioSummary,
    WriteUpStudioNode,
};
pub use studio_added::{AddedContent, StudioNodeAdded};
pub use studio_sketch::{
    EditStudioSketch, SketchBox, SketchDrawing, SketchDrawn, SketchJoin, SketchPicture,
    SketchPictureDrawn, SketchPoint, SketchStroke,
};
pub use added_steps::{
    AddStep, AddedPlaced, AddedRuns, AddedSkip, AddedSkipReason, AddedStep, AddedStepRemoved,
    EditAddedStep, JobAdditionChanged, KeptFrom, RemoveAddedStep,
};
pub use trigger_holds::{HoldAct, HoldSettled, JobAlert, JobAlertKind};
pub use triggers::{
    ChooseTriggerFix, JobTrigger, JobTriggerChanged, LeftOutTrigger, OverriddenTrigger,
    RemoveTrigger, RepairOf, SaveTrigger, TriggerDefinition, TriggerFiringState, TriggerFixChoice,
    TriggerFixChosen, TriggerLevel, TriggerList, TriggerMoment, TriggerPullRequest, TriggerRemoved,
    TriggerRepair, TriggerRuns, TriggerSaved, TriggerScope, TriggerSkip, TriggerSkipReason,
    TriggerSummary,
};
pub use turn::{
    BlockKind, CallArguments, Closed, Opened, Saw, Shown, Silence, TranscriptRow, TurnMessage,
    Voice, Withheld,
};
pub use underway::{
    CheckUnderway, ChecksUnderway, OutputClosed, OutputEnded, OutputLines, OutputMessage,
    OutputOpened,
};
pub use usage::{FleetUsage, ManifestSpend, Overspending};
pub use protocol_id::ProtocolId;
pub use waiting::{AskedOption, ChosenAnswer, QuestionInFlight, RedirectInFlight, RedirectWaiting};
pub use walk_notes::{CaptureWalkNote, RemoveWalkNote, WalkNote, WalkNotes};
pub use work::{
    ChangesRequested, DeclaredPlan, JobDiff, JobEvidence, JobFootprint, LineCount, Overruled,
    Submitted, TouchedFile, Work,
};
pub use work_plan::{
    AddTask, ChangedBy, DropTask, EditTask, JobPlanChanged, MovePlan, PlanGroup, PlanGroupRun,
    PlanTask, RestartTask, SetTiers, TaskCounts, TierModels, WorkPlan, WorkingWindow,
};
