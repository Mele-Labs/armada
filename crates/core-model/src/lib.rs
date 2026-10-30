//! The vocabulary every other crate agrees on: the workflow definition, the
//! Studio graph, Kit, the retro words, evidence.
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
//! **`no_std`, except under test.** The attribute is conditional only because
//! the unit test harness needs `std` to link; every shipped build of this crate
//! is `no_std` and depends on nothing.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod allowed;
mod credentials;
mod envelope;
/// Kit's MCP servers and the two tiers that reach a Drone. `docs/concepts/kit.md`.
mod kit;
/// The state machines a Job's tickets, pull requests, deploys and Jobs move through.
mod lifecycle;
mod policy;
/// Which door an act came through, and whom friction got in the way of.
/// `docs/concepts/retro.md`.
mod retro;
/// A Studio, its nodes and its edges. `docs/concepts/studio.md`.
mod studio;
/// Which tests reach each file in a repository. `docs/concepts/test-map.md`.
mod test_map;
/// A command Fleet runs on a cadence for one Job, and what its readings come to.
mod watch;
/// The workflow a Session or a Job follows, and the ids it is spelled in.
mod workflow;

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
pub use allowed::AllowedCommand;
pub use credentials::CREDENTIAL_NAMES;
pub use policy::{AutoMerge, ReviewGate};
pub use workflow::{
    ARTIFACT_EXISTS, AdvanceGate, BadPattern,
    BlankModel, BlankTitle, CheckOutcome, Citation, ContextSource, Covers,
    CriterionId, DIFF_NONEMPTY, DeclarePlanAt, DeclaredPaths, EVERY_MANIFEST_CHECK, Effort, EvidenceRef, EvidenceScope, EvidenceType,
    FrozenWorkflow, GateVerdict, Given, JudgeCheck, JudgeCriterion,
    JudgeVerdict, Judgment, Lands, MANIFEST_CHECK, ManifestId, ModelName, Narrowing, OnRefusal, PLAN_RECORDED, PathPattern, PrMode, Prerequisite, RepoPath, ResolvedCheck, ResolvedStep, Runner, RunsAt, StepCheck, StepEnd, StepId, StepPhase,
    Title, WorkflowId,
    WorkflowSource, under,
};
pub use kit::{
    a_drone_resolves, KitServer, ManifestReach, ReachesDrones, ServerAddress, ServerName,
};
pub use lifecycle::{
    AlreadyHeld, BlankKey, BlankScope, FleetFact, ForgeFact, HeldInstances, InstanceRef, Lifecycle,
    LifecycleId, LifecycleInstance, LifecycleScope, LifecycleState, LifecycleTransition,
    MalformedLifecycle, Moved, NotAnId, Observed, ScopeId, StateId, StateRead, StateSource, Standing,
    TrackerWords,
};
pub use retro::{Change, LandsIn, LessonState, Via, Whose};
pub use test_map::{EdgeGrain, EdgeSource, TestMapEdge};
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
pub use watch::{
    counted_state, ends_on, ends_passed, fill, set_aside_failing, span_seconds, watch_state_of,
    value_slot, BrokenReading, NotFilled,
    NotWeighed, Part, Reading, ReadingState, SetAside, Verdict, Watch, WatchState,
};
