//! The workflow a Session or a Job follows: its steps, what each declares, the
//! Checks and the Judge it runs, and the ids and types they are spelled in.
//!
//! **Nothing here names a Job.** A Session reads a frozen workflow, and the
//! ids and the closed sets it declares, with no Job record in the picture, so
//! these live beside `job` rather than in it. `job` re-exports every name below
//! for the Job code that still reaches it that way.
//!
//! `frozen` is the file `job/workflow.rs` was: a module cannot share its
//! parent's name without tripping `module_inception`.

mod check;
mod covers;
mod declared;
pub(crate) mod frozen;
mod gaming;
mod ids;
mod judge;
mod narrowing;
mod passes;
mod phase;
mod pr_mode;
mod prerequisite;
mod runner;
mod runs_at;
mod scope;
mod source;
mod tuning;
mod verdict;

#[cfg(test)]
mod tests;

pub use check::{CheckOutcome, StepCheck};
pub use covers::{BadPattern, Covers, PathPattern};
pub use declared::{AdvanceGate, EvidenceType};
pub use frozen::{
    FrozenWorkflow, ResolvedCheck, ResolvedStep, ARTIFACT_EXISTS, DIFF_NONEMPTY,
    EVERY_MANIFEST_CHECK, MANIFEST_CHECK, PLAN_RECORDED,
};
pub use gaming::{
    CitedAt, ClearedFlag, DecidedBy, EvidenceRef, GamingCheck, GamingFlag, GamingPattern,
};
/// What the Judge is asked about each judged pattern, as Armada ships it.
pub use gaming::{
    ASSERTION_WEAKENED_QUESTION, FINDINGS_GENERIC_QUESTION, FINDINGS_NOT_TIED_QUESTION,
    NO_FINDINGS_QUESTION, TAUTOLOGICAL_TEST_QUESTION, TEST_SCOPE_NARROWED_QUESTION,
};
pub(crate) use ids::id_newtype;
pub use ids::{
    BlankModel, BlankTitle, CriterionId, DroneId, JobId, ManifestId, ModelName, ProposalId,
    RepoPath, StepId, Title, WorkflowId,
};
pub use judge::{
    Citation, Given, JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment, OnRefusal, AGREED_RETRIES,
};
pub use narrowing::Narrowing;
pub use passes::{Iteration, Spent};
pub use phase::StepPhase;
pub use pr_mode::PrMode;
pub use prerequisite::Prerequisite;
pub use runner::Runner;
pub use runs_at::RunsAt;
pub use scope::{under, ContextSource, DeclarePlanAt, DeclaredPaths, EvidenceScope};
pub use source::WorkflowSource;
pub use tuning::{Effort, StepTuning};
pub use verdict::GateVerdict;
