//! A Job created at `proposing`, and the one move that gives it a workflow.
//!
//! A dispatched request is a Job from the press, with the request as its title
//! and no workflow, steps or plan — `.claude/decisions/2026-09-30-a-dispatched-request-is-a-job.md`.
//!
//! **The workflow is unfrozen rather than optional.** It has no steps and names
//! at most what the proposer settled on; [`Job::frozen_workflow`] tells the two
//! apart. Every other reader is reached past approval, and an empty step list
//! already answers each of their lookups with nothing.
//!
//! [`Job::answered`] is the only way to the gate: [`Job::transition`] refuses
//! it, since crossing is when a workflow freezes and steps are made. A child of
//! `record` for its private fields, because that file is near the length limit.

use alloc::vec::Vec;

use crate::envelope::{Actor, Timestamp, Ulid};
use crate::job::event::JobEvent;
use crate::job::fields::{
    AcceptanceCriterion, Attachment, DependencyEdge, DispatchOrigin, Facts, Origin, ScopeRevision,
    Subject, TopLevelOrigin, Urgency, WriteTargets,
};
use crate::job::handle::JobNumber;
use crate::job::ids::{JobId, ManifestId, ModelName, ProposalId, Title, WorkflowId};
use crate::job::status::JobStatus;
use crate::job::step::{rows_at_creation, StepSeed};
use crate::job::transition::{IllegalTransition, TransitionReason};
use crate::job::workflow::FrozenWorkflow;

use super::{Job, NewJob, Transitioned};

/// Everything a dispatched request decides before anybody has read it. No
/// status, no workflow and no steps, for [`NewJob`]'s reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewProposal {
    pub id: JobId,
    /// The request as typed: the row's title until the proposer writes one.
    pub title: Title,
    pub owner_manifest_id: ManifestId,
    /// What configuration decides, until the proposer names one.
    pub model: ModelName,
    /// Never absent: a Job at `proposing` is one by being proposed.
    pub proposal_id: ProposalId,
    pub number: JobNumber,
    /// The request again, as the brief. It stays the brief once a title lands.
    pub facts: Facts,
    /// Promoted at dispatch, so the head of the plan carries them on.
    pub attachments: Vec<Attachment>,
}

/// What the proposer's answer freezes into the Job it was reading for —
/// exactly the fields it decides, and none of the Job's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    /// Replaces the request, which is already the brief.
    pub title: Title,
    pub workflow: FrozenWorkflow,
    /// One per step of `workflow`, in order.
    pub steps: Vec<StepSeed>,
    pub urgency: Urgency,
    pub atomic: bool,
    pub model: ModelName,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub dependencies: Vec<DependencyEdge>,
    pub write_targets: Option<WriteTargets>,
    pub subject: Option<Subject>,
    pub facts: Facts,
    pub scope_revisions: Vec<ScopeRevision>,
}

impl NewJob {
    /// The part of a drafted Job an answer freezes into a `proposing` one.
    ///
    /// Drops the identity the `proposing` Job already has — id, number,
    /// proposal, Manifest, attachments, `redispatched_from`, gate manifests —
    /// so Fleet drafts the head as it drafts any Job, and the store rebuilds
    /// one from its row the same way.
    pub fn into_answer(self) -> Answered {
        Answered {
            title: self.title,
            workflow: self.workflow,
            steps: self.steps,
            urgency: self.urgency,
            atomic: self.atomic,
            model: self.model,
            acceptance_criteria: self.acceptance_criteria,
            dependencies: self.dependencies,
            write_targets: self.write_targets,
            subject: self.subject,
            facts: self.facts,
            scope_revisions: self.scope_revisions,
        }
    }
}

/// A workflow with no steps, naming the one the proposer settled on or none.
fn unfrozen(settled: Option<WorkflowId>) -> FrozenWorkflow {
    FrozenWorkflow::frozen(
        settled.unwrap_or_else(|| WorkflowId::carried(Ulid::carried(""))),
        alloc::string::String::new(),
        0,
        Vec::new(),
    )
}

impl Job {
    /// A dispatched request, before the proposer has answered.
    pub fn create_proposing(new: NewProposal, origin: TopLevelOrigin, at: Timestamp) -> Job {
        let NewProposal {
            id,
            title,
            owner_manifest_id,
            model,
            proposal_id,
            number,
            facts,
            attachments,
        } = new;
        let draft = NewJob {
            id,
            title,
            workflow: unfrozen(None),
            owner_manifest_id,
            urgency: Urgency::Normal,
            atomic: false,
            model,
            acceptance_criteria: Vec::new(),
            steps: Vec::new(),
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            proposal_id: Some(proposal_id),
            number,
            facts,
            scope_revisions: Vec::new(),
            attachments,
        };
        let mut job = Job::create(draft, origin.into(), None, JobStatus::Proposing, at);
        job.workflow_frozen = false;
        job
    }

    /// One of a split's extras, at `awaiting_approval` with `dispatched_by`
    /// naming the head and no step.
    ///
    /// The decision record makes the extras the head's members through
    /// `dispatched_by`. No step of the head dispatched them, and nobody
    /// approved them as part of anything, so the origin stays who sent the
    /// request and each takes its own approval.
    pub fn create_split(new: NewJob, head: JobId, origin: TopLevelOrigin, at: Timestamp) -> Job {
        let by = DispatchOrigin {
            job_id: head,
            step_id: None,
        };
        Job::create(
            new,
            Origin::from(origin),
            Some(by),
            JobStatus::AwaitingApproval,
            at,
        )
    }

    /// Note the workflow the proposer settled on before it finished — a
    /// choice and not a freeze, so a call that dies after it leaves the
    /// workflow on the escalated Job, as the owner took on 30 Sep 2026. A
    /// no-op on a Job whose workflow is frozen.
    pub fn workflow_settled(&self, settled: WorkflowId) -> Job {
        let mut job = self.clone();
        if !self.workflow_frozen {
            job.workflow = unfrozen(Some(settled));
        }
        job
    }

    /// The proposer answered: freeze its workflow, make the steps, and cross
    /// to `awaiting_approval`. Refused anywhere but `proposing`.
    pub fn answered(
        &self,
        answer: Answered,
        by: Actor,
        at: Timestamp,
    ) -> Result<Transitioned, IllegalTransition> {
        if self.status != JobStatus::Proposing {
            return Err(IllegalTransition::NothingToAnswer { from: self.status });
        }
        let event = JobEvent::recorded(
            self.id.clone(),
            JobStatus::Proposing,
            JobStatus::AwaitingApproval,
            TransitionReason::Unqualified,
            by,
            at.clone(),
        );
        let mut job = self.clone();
        job.status = JobStatus::AwaitingApproval;
        job.title = answer.title;
        job.workflow = answer.workflow;
        job.workflow_frozen = true;
        job.steps = rows_at_creation(&self.id, answer.steps, &at);
        job.urgency = answer.urgency;
        job.atomic = answer.atomic;
        job.model = answer.model;
        job.acceptance_criteria = answer.acceptance_criteria;
        job.dependencies = answer.dependencies;
        job.write_targets = answer.write_targets;
        job.subject = answer.subject;
        job.facts = answer.facts;
        job.scope_revisions = answer.scope_revisions;
        Ok(Transitioned { job, event })
    }
}
