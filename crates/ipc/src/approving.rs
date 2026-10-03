//! What a person decides about a Job before it runs, and how it reads back.
//! Spike 022, slice 4: #1641's approval body, #1699's `edit_job`, #1642's
//! criterion origin, #1605's branch list. **Since 23.8.**
//!
//! **One body carries the whole proposal**, because the values are decided
//! together: a partial apply would leave a Job half on a new workflow. Every
//! field is optional so that an approval with no body — every Bridge before
//! 23.8 — still approves the proposal as it stands.

use serde::{Deserialize, Serialize};

use crate::enums::CriterionSource;
use crate::ids::{CriterionId, StepId, WorkflowId};
use crate::work_plan::TierModels;
use crate::Instant;

/// `approve_dispatch`'s body: the proposal as the person leaves it at the
/// press. **A field left out is the proposal as it stands**, except
/// `drone_cap`: the body is the whole proposal, so a cap left out is no cap of
/// the Job's own, the machine's holding.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveDispatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The request, rewritten. `JobDetail.facts`'s name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<String>,
    /// Refused unless it is a workflow this Job's repository holds. **A new
    /// workflow brings its own steps**, so `gates` beside it name the new
    /// workflow's steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<WorkflowId>,
    /// One per step a person set. A step left out keeps the gate its workflow
    /// declares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<Vec<GateChoice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<CriterionWritten>>,
    /// Left out is the map as it stands; `{}` is Armada picking every tier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiers: Option<TierModels>,
    /// How many Drones this Job may run at once, inside the machine's own cap.
    /// **Kept from 23.8 and enforced from slice 5**: until then a Job runs one
    /// Drone at a time, which any cap allows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_cap: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<LandingChoice>,
}

/// `edit_job`'s body (#1699): a proposal's words, saved without releasing it.
/// **The fields `ApproveDispatch` shares**, in its shape, and only the ones a
/// person changed.
///
/// **An unknown field is refused, not dropped**: Bridge's panel sent `brief`
/// and `expects` before this body was agreed, and a save that kept the title
/// and silently lost the brief would read as done.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditJob {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<CriterionWritten>>,
}

/// What one step is gated by, as the approval sets it. **Four states, not
/// three bits** (#1532): the fourth is the repository deciding, which a step
/// declaring `manifest_rule:` keeps unless `overridden`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateChoice {
    pub step_id: StepId,
    /// Whether the step's Checks run at its gate. Unticked drops them.
    pub checks: bool,
    /// Whether the Judge looks. Unticked drops what it would read.
    pub judge: bool,
    /// Whether it stops for a person.
    pub you: bool,
    /// Whether this Job overrides the repository's rule for itself. **Holds
    /// for the life of the Job, however the rule moves** (spike 022, answer 4).
    #[serde(default)]
    pub overridden: bool,
}

/// One criterion as a person leaves it. **An absent id is a new line**, and
/// Fleet mints it one; an id is the line it was, reworded or not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionWritten {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<CriterionId>,
    pub text: String,
    pub source: CriterionSource,
}

/// Whether the unit of a branch is the Job or a group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandingUnit {
    #[default]
    Job,
    Group,
}

/// What has to happen before a Job counts as finished. **`delivered` is
/// honoured from 23.8 and `all_members_landed` from 23.13** (`landing.ts`,
/// `COMPLETE_WHEN_SERVED`); the approval refuses the other two rather than
/// keeping a setting nothing reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompleteWhen {
    PrMerged,
    AllMembersLanded,
    PrOpened,
    #[default]
    Delivered,
}

/// How the Job's work reaches the repository, as the approval sets it.
/// `land_together` does not cross (spike 022: where it lives is open).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandingChoice {
    /// The branch the pull request opens against. Left out is the Manifest's
    /// base. **Refused unless the repository holds it.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The branch the worktree is cut from. Left out is `target`'s reading.
    /// Refused unless the repository holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_ref: Option<String>,
    /// Refused at `group`: one branch per Job is all Fleet runs.
    #[serde(default)]
    pub branching: LandingUnit,
    /// `ready` or `draft`. Left out is `ready`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_mode: Option<crate::PrMode>,
    #[serde(default)]
    pub complete_when: CompleteWhen,
}

/// How one Job lands, frozen at approval, on `JobDetail.landing`. **Absent is
/// a Job approved before 23.8, or with no body**: cut from the base, landing in
/// it, ready.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandingRule {
    /// Absent is the Manifest's base, which is not a branch name to print.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_ref: Option<String>,
    pub pr_mode: crate::PrMode,
    /// What finishes the Job. Since 23.13, when `all_members_landed` became one
    /// Fleet runs; absent is a Fleet before it, whose Jobs finished delivered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complete_when: Option<CompleteWhen>,
}

impl From<&core_model::Landing> for LandingRule {
    fn from(landing: &core_model::Landing) -> LandingRule {
        LandingRule {
            target: landing.target.as_ref().map(|b| b.as_str().to_string()),
            from_ref: landing.from_ref.as_ref().map(|b| b.as_str().to_string()),
            pr_mode: landing.pr_mode.into(),
            complete_when: Some(CompleteWhen::from(landing.complete_when)),
        }
    }
}

impl From<core_model::CompleteWhen> for CompleteWhen {
    fn from(when: core_model::CompleteWhen) -> CompleteWhen {
        match when {
            core_model::CompleteWhen::Delivered => CompleteWhen::Delivered,
            core_model::CompleteWhen::AllMembersLanded => CompleteWhen::AllMembersLanded,
        }
    }
}

/// What Approve the plan sends at an Epic's plan gate (#1694): **every Job of
/// the wave it releases**, by id. Refused unless it names exactly the wave
/// Fleet holds — each Job its plan proposed that is still at
/// `awaiting_approval` — so a person releases what they read, all of it or
/// none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveWave {
    pub jobs: Vec<crate::JobId>,
}

/// What this Job's approval said in place of the repository's policies, on
/// `JobDetail.policy_overrides`. **The word as `armada.yml` writes it**, so a
/// surface reads it as it reads `ManifestSummary`'s. Absent is the repository
/// deciding.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_merge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_gate: Option<String>,
}

impl From<&core_model::PolicyOverrides> for PolicyOverrides {
    fn from(overrides: &core_model::PolicyOverrides) -> PolicyOverrides {
        PolicyOverrides {
            auto_merge: overrides.auto_merge.map(|p| p.as_written().to_string()),
            review_gate: overrides.review_gate.map(|p| p.as_written().to_string()),
        }
    }
}

/// Where a criterion's words came from (#1642). **Absent on the criterion is
/// nothing saying**: a Job no person dispatched, or one kept before 23.8.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CriterionOrigin {
    /// Read out of the issue the request linked. `url` is the forge's own
    /// address, the only one Bridge's main process opens.
    Issue {
        #[serde(rename = "ref")]
        reference: String,
        url: String,
    },
    /// Read out of the request a person typed.
    Prompt,
    /// Typed by a person, at the form or at the gate.
    Person,
}

/// The settings a person may set while typing the request, on `JobRequest`.
/// **Every field optional, and absent is decided elsewhere** (#1540): the
/// proposer picks the workflow, the machine's cap holds, the workflow's
/// delivering step decides how it lands.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<WorkflowId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiers: Option<TierModels>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_cap: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands: Option<LandsWhen>,
}

/// Whether anybody is asked before the work lands, set at dispatch on the
/// workflow's delivering step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandsWhen {
    Auto,
    YouAtReview,
}

/// `list_branches`' answer (#1605): the repository's local branches, the base
/// first. **The repository's own list**, read through git, not the floor
/// Bridge composed from the base and the worktrees it had seen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branches {
    pub branches: Vec<BranchRow>,
}

/// One branch a Job may start from or land in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRow {
    pub name: String,
    /// The Manifest's base, or the branch the adapter inferred as one. One at
    /// most.
    pub base: bool,
}

/// When a criterion's issue moved, read off the Job's source. Present only on
/// a criterion whose origin is that issue, and only once it moved.
pub fn moved_at(source: Option<&core_model::IssueSource>) -> Option<Instant> {
    source.and_then(|source| source.moved_at.as_ref().map(Instant::from))
}
