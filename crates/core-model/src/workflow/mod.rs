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
mod evidence_ref;
mod ids;
mod judge;
mod lands;
mod narrowing;
mod phase;
mod pr_mode;
mod prerequisite;
mod runner;
mod runs_at;
mod scope;
mod source;
mod step_end;
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
pub use evidence_ref::EvidenceRef;
pub(crate) use ids::id_newtype;
pub use ids::{
    BlankModel, BlankTitle, CriterionId, ManifestId, ModelName, RepoPath, StepId, Title, WorkflowId,
};
pub use judge::{
    Citation, Given, JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment, OnRefusal,
};
pub use lands::Lands;
pub use narrowing::Narrowing;
pub use phase::StepPhase;
pub use pr_mode::PrMode;
pub use prerequisite::Prerequisite;
pub use runner::Runner;
pub use runs_at::RunsAt;
pub use scope::{under, ContextSource, DeclarePlanAt, DeclaredPaths, EvidenceScope};
pub use source::WorkflowSource;
pub use step_end::StepEnd;
pub use tuning::Effort;
pub use verdict::GateVerdict;
