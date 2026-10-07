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

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
pub use job::{
    branch_named, criterion_numbered, next_criterion_number, CompleteWhen, CriterionOrigin,
    IssueSource, Landing, NotAtApproval, PolicyOverrides, PrMode, ProposalEdit,
};
pub use job::{collisions, under};
pub use job::{
    handle_of, names_a_credential, AcceptanceCriterion, AdmissionHold, AdvanceGate, AllowedCommand,
    Answered, Area, Attachment, Attempt, AutoMerge, BadPattern, Became, BlankBranch, BlankModel,
    BlankTitle, Branch, Breakage, BreakageClaim, Bucket, BudgetHold, ChangedTest, CheckOutcome,
    Citation, CitedAt, ClearedFlag, Collision, Confidence, ContextSource, Covers, CriteriaOwed,
    CriterionId, CriterionSource, DecidedBy, DeclarePlanAt, DeclaredPaths, DependencyDirection,
    DependencyEdge, Dismissal, DispatchOrigin, DroneAssigned, DroneId, DroneMoved, DronePresence,
    DroneStanding, Edge, Effort, EscalationTrigger, EvidenceRef, EvidenceScope, EvidenceType,
    Facts, Finding, FixWaiter, FollowUp, FrozenWorkflow, GamingCheck, GamingFlag, GamingPattern,
    GateManifest, GateOutcome, GateVerdict, Given, Guard, IllegalDroneMove, IllegalStepTransition,
    IllegalTransition, Iteration, Job, JobEvent, JobId, JobNumber, JobReference, JobStatus,
    JobStep, JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment, LandedHold, ManifestId, ModelName,
    Narrowing, NewJob, NewProposal, NotRunDisposition, NotRunReason, OnRefusal, Origin,
    PathPattern, Pause, PausedBy, PilotReason, Prerequisite, ProposalId, Proves, QueuedReason,
    Reach, Recourse, RedirectAlreadyWaiting, RedirectWaiting, Refusal, Refusals, RepoPath,
    ResolvedCheck, ResolvedPolicies, ResolvedStep, Resumption, ReviewGate, ReviewRecord, Runner,
    RunsAt, ScopeClaim, ScopeRevision, ScopeRevisionOutcome, Side, Spent, Standing, StepCheck,
    StepEdge, StepEvent, StepEvidence, StepFrame, StepId, StepLevelTrigger, StepPhase, StepSeed,
    StepState, StepTarget, StepTransitioned, StepTuning, StepVerdict, Stuck, Subject, Target,
    TestChange, TestsInChange, Title, TopLevelOrigin, TransitionReason, Transitioned, TriggerKind,
    TriggerLevel, Untested, Urgency, ViewStep, WhenBlocked, WhenRefused, WorkflowId,
    WorkflowSource, WriteTargets, ADVANCING_STATUSES, ARTIFACT_EXISTS, CREDENTIAL_NAMES,
    DIFF_NONEMPTY, EDGES, EVERY_MANIFEST_CHECK, MANIFEST_CHECK, STEP_EDGES,
};
pub use job::{
    Apart, Approach, DropReason, FailReason, GroupAttempt, GroupEnded, GroupId, GroupMove,
    GroupRuns, GroupState, NewTask, NotAnUpdate, PlanAuthor, PlanChange, PlanEntry, PlanRefused,
    PlanTask, Shown, TaskCounts, TaskEdit, TaskId, TaskState, TaskTier, TaskUpdate, TierModels,
    WorkPlan, WorkingWindow, PLAN_RECORDED,
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
pub use trigger::{
    FrozenTrigger, OnTriggerFailure, Trigger, TriggerFiring, TriggerIdentity, TriggerResolution,
    TriggerRuns, TriggerSkipped, TriggerSource, TriggerState, TriggerWhen,
};
