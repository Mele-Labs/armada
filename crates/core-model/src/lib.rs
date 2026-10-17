//! The vocabulary every other crate agrees on: the Job record, its states and
//! transitions, the escalation triggers, the workflow definition, evidence.
//!
//! **What may not enter this crate: no runtime, no I/O, no vendor.**
//! `core-model` is the one crate every other crate depends on, so a dependency
//! added here is a dependency added everywhere — which is why `cargo tree` on
//! this crate is a gate rule rather than a preference. No async runtime, no VCS
//! library, no HTTP client, reachable at any depth. Serialisation lives here
//! only as derives on types defined here: reading untyped JSON belongs to
//! `store` and `ipc`, the two places bytes enter the process.
//!
//! **The log envelope is here** because a line shape retrofitted after five
//! crates are already logging is a rewrite of all five, and `actor` cannot be
//! reconstructed afterwards at all.
//!
//! **The Job record and both halves of its machine** are here too, built from
//! `domain/`, which is the authority on the outer one. The registry gives step
//! states no edge table, so the inner machine declares its own edges and says
//! on each why it is there — [`StepTarget`] carries the reasoning, and what M1
//! cannot reach is unreachable because no value names it.
//!
//! **`no_std`, except under test.** The attribute is conditional only because
//! the unit test harness needs `std` to link; every shipped build of this crate
//! is `no_std` and depends on nothing.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod added_step;
mod envelope;
mod job;
/// Kit's MCP servers and the two tiers that reach a Drone. `docs/concepts/kit.md`.
mod kit;
/// Which door an act came through, and whom friction got in the way of.
/// `docs/concepts/retro.md`.
mod retro;
/// A Studio, its nodes and its edges. `docs/concepts/studio.md`.
mod studio;
/// Something that runs at a moment in a Job. `docs/concepts/trigger.md`.
mod trigger;
/// The workflow a Session or a Job follows, and the ids it is spelled in.
mod workflow;

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
pub use job::{
    branch_named, criterion_numbered, next_criterion_number, CompleteWhen, CriterionOrigin,
    IssueSource, Landing, NotAtApproval, PolicyOverrides, PrModeTiers, ProposalEdit,
};
pub use job::collisions;
pub use job::{
    Decision, DroneQuestion, GivenAnswer, OfferedAnswer,
};
pub use job::{
    handle_of, names_a_credential, AcceptanceCriterion, AdmissionHold, AllowedCommand, Answered,
    Area, Attachment, Attempt, AutoMerge, Became, BlankBranch, Branch, Breakage, BreakageClaim,
    Bucket, BudgetHold, ChangedTest, Collision, Confidence, CriteriaOwed, CriterionSource,
    DependencyDirection, DependencyEdge, Dismissal, DispatchOrigin, DroneAssigned, DroneMoved,
    DronePresence, DroneStanding, Edge, EscalationTrigger, Facts, Finding, FixWaiter, FollowUp,
    GateManifest, GateOutcome, Guard, IllegalDroneMove, IllegalStepTransition,
    IllegalTransition, Job, JobEvent, JobNumber, JobReference, JobStatus, JobStep, LandedHold,
    NewJob, NewProposal, NotRunDisposition, NotRunReason, Origin, Pause, PausedBy, PilotReason,
    Proves, QueuedReason, Reach, Recourse, RedirectAlreadyWaiting, RedirectWaiting, Refusal,
    Refusals, ResolvedPolicies, Resumption, ReviewGate, ReviewRecord, ScopeClaim, ScopeRevision,
    ScopeRevisionOutcome, Side, Standing, StepEdge, StepEvent, StepEvidence, StepFrame,
    StepLevelTrigger, StepSeed, StepState, StepTarget, StepTransitioned, StepVerdict, Stuck,
    Subject, Target, TestChange, TestsInChange, TopLevelOrigin, TransitionReason, Transitioned,
    TriggerKind, TriggerLevel, Untested, Urgency, ViewStep, WhenBlocked, WhenRefused,
    WriteTargets, ADVANCING_STATUSES, CREDENTIAL_NAMES, EDGES, STEP_EDGES,
};
pub use job::{
    Apart, Approach, DropReason, FailReason, GroupAttempt, GroupEnded, GroupId, GroupMove,
    GroupRuns, GroupState, NewTask, NotAnUpdate, PlanAuthor, PlanChange, PlanEntry, PlanRefused,
    PlanTask, Shown, TaskCounts, TaskEdit, TaskId, TaskState, TaskTier, TaskUpdate, TierModels,
    WorkPlan, WorkingWindow,
};
pub use workflow::{
    AGREED_RETRIES, ARTIFACT_EXISTS, ASSERTION_WEAKENED_QUESTION, AdvanceGate, BadPattern,
    BlankModel, BlankTitle, CheckOutcome, Citation, CitedAt, ClearedFlag, ContextSource, Covers,
    CriterionId, DIFF_NONEMPTY, DecidedBy, DeclarePlanAt, DeclaredPaths, DroneId,
    EVERY_MANIFEST_CHECK, Effort, EvidenceRef, EvidenceScope, EvidenceType,
    FINDINGS_GENERIC_QUESTION, FINDINGS_NOT_TIED_QUESTION, FrozenWorkflow, GamingCheck,
    GamingFlag, GamingPattern, GateVerdict, Given, Iteration, JobId, JudgeCheck, JudgeCriterion,
    JudgeVerdict, Judgment, MANIFEST_CHECK, ManifestId, ModelName, NO_FINDINGS_QUESTION,
    Narrowing, OnRefusal, PLAN_RECORDED, PathPattern, PrMode, Prerequisite, ProposalId,
    RepoPath, ResolvedCheck, ResolvedStep, Runner, RunsAt, Spent, StepCheck, StepId, StepPhase,
    StepTuning, TAUTOLOGICAL_TEST_QUESTION, TEST_SCOPE_NARROWED_QUESTION, Title, WorkflowId,
    WorkflowSource, under,
};
pub use kit::{
    a_drone_resolves, KitServer, ManifestReach, ReachesDrones, ServerAddress, ServerName,
};
pub use retro::{Change, LandsIn, LessonState, Via, Whose};
pub use studio::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureServed, CaptureWindow,
    ContradictionOutcome, Drawing, EdgeRefused, EndedFinding, EpicRead, EpicTake, ForgeFacts,
    ForgeState, GatheringFinding, NotRewritable, NotScoutable, Recognised, Rewritten,
    ScoutCheckout, ScoutEnded, ScoutLook, ScoutOutcome, ScoutSource, ScoutSourceKind, Scouted,
    SketchBox, SketchDrawing, SketchJoin, SketchMalformed, SketchPicture, SketchPoint,
    SketchStroke, StateDoesNotFit, Studio, StudioAuthor, StudioCapture, StudioEdge, StudioEdgeId,
    StudioEdgeKind, StudioEdgeStanding, StudioFinding, StudioGraph, StudioId, StudioName,
    StudioNode, StudioNodeContent, StudioNodeId, StudioNodeKind, StudioNodeState, StudioPosition,
    StudioRelation, StudioRun, StudioRunKept, ToItself,
};
pub use added_step::{
    behind, placeable, AddedKind, AddedStep, Behind, Fired, Kept, Misplaced, NotRun, Placed,
};
pub use trigger::{
    can_hold, FixChoice, FrozenTrigger, OnTriggerFailure, RepairRecord, Trigger, TriggerFiring,
    TriggerIdentity, TriggerResolution, TriggerRuns, TriggerSkipped, TriggerSource, TriggerState,
    TriggerWhen, REPAIR_TRIES,
};
