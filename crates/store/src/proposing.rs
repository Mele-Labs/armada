//! Which reading a Job was minted by, and the one column that says so.
//!
//! **The only thing on the record that says two Jobs are the same request.** A
//! proposal that splits writes no edge between the Jobs it mints — they may run
//! in any order, so there is nothing to sequence — and until this column the
//! split survived only in two titles. A request naming a bug and an addition
//! became two Jobs on 9 Sep 2026, the first landed both, and the second was
//! dispatched into a base that already held its work.
//!
//! **A column on `jobs`, not a table.** At most one per Job, written once at
//! creation and never afterwards, and only ever read beside the Job it belongs
//! to — [`crate::delivery`]'s argument for the same shape.
//!
//! **The column is the authority for its field.** No event carries a proposal
//! id, so there is nothing to fold and [`crate::read`] reads it straight back.

/// Version 34 — the reading a Job was minted by.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// One nullable column and no backfill. Null is a Job nobody proposed, which is
/// exactly what every Job written before this reads as — the proposals that
/// minted them are over and their ids were never kept.
pub(crate) const V34: &str = r#"
ALTER TABLE jobs ADD COLUMN proposal_id TEXT;
"#;

use core_model::{
    FrozenWorkflow, Job, JobStatus, NewJob, NewProposal, Origin, Timestamp, Transitioned, Ulid,
    WorkflowId,
};

use crate::columns;
use crate::error::{fault, RowError, WriteError};
use crate::fold::{Moved, RecordedEvent};
use crate::open::Store;
use crate::write::{append_transition, refused_move, write_steps};

impl Store {
    /// Note the workflow the proposer has settled on, on a Job still at
    /// `proposing`. A choice and not a freeze: `workflow` stays null, so a call
    /// that dies after this leaves the escalated Job naming it. A Job that has
    /// moved on is left alone — its workflow is frozen.
    pub fn record_workflow_settled(&mut self, job: &Job) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE jobs SET workflow_id = ?2 WHERE job_id = ?1 AND status = 'proposing'",
                (job.id().as_str(), job.workflow_id().as_str()),
            )
            .map_err(fault("writing the settled workflow"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Record a proposer's answer: the fields it froze, the step rows it made,
    /// and `proposing -> awaiting_approval`, in one transaction.
    ///
    /// **The one write that rewrites a Job's frozen columns**, because the
    /// answer is what freezes them. Compare-and-swap on `proposing`, for
    /// `record_transition`'s reason: a stop landing as the answer arrives must
    /// leave one of the two, never both.
    pub fn record_answered(&mut self, answered: &Transitioned) -> Result<i64, WriteError> {
        let job = &answered.job;
        let event = &answered.event;
        let workflow = job
            .frozen_workflow()
            .ok_or_else(|| WriteError::NotAnAnswer {
                job_id: job.id().clone(),
            })?;
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the answer"))
            .map_err(WriteError::Database)?;
        let updated = tx
            .execute(
                "UPDATE jobs SET status = ?2, title = ?3, workflow_id = ?4, workflow = ?5,
                     urgency = ?6, atomic = ?7, model = ?8, acceptance_criteria = ?9,
                     dependencies = ?10, subject_kind = ?11, subject_ref = ?12, facts = ?13,
                     scope_revisions = ?14, write_targets_known = ?15
                 WHERE job_id = ?1 AND status = ?16",
                rusqlite::params![
                    job.id().as_str(),
                    event.to().as_wire(),
                    job.title().as_str(),
                    job.workflow_id().as_str(),
                    columns::write_workflow(workflow),
                    job.urgency().as_wire(),
                    job.atomic(),
                    job.model().as_str(),
                    columns::write_acceptance_criteria(job.acceptance_criteria()),
                    columns::write_dependencies(job.dependencies()),
                    job.subject().map(|subject| subject.kind.as_str()),
                    job.subject().map(|subject| subject.reference.as_str()),
                    job.facts().as_str(),
                    columns::write_scope_revisions(job.scope_revisions()),
                    job.write_targets().is_some(),
                    JobStatus::Proposing.as_wire(),
                ],
            )
            .map_err(fault("writing the answer"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(refused_move(&tx, event)?);
        }
        write_steps(&tx, job)?;
        crate::write::write_targets(&tx, job)?;
        let seq = append_transition(&tx, event)?;
        tx.commit()
            .map_err(fault("committing the answer"))
            .map_err(WriteError::Database)?;
        Ok(seq)
    }
}

/// The workflow a Job at `proposing` names: the `workflow_id` column, settled
/// or blank, with no definition and no steps — the shape `Job::create_proposing`
/// itself holds, so the rebuild hands the constructor nothing it did not have.
pub(crate) fn settled(workflow_id: String) -> FrozenWorkflow {
    FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried(workflow_id)),
        String::new(),
        0,
        Vec::new(),
    )
}

/// Whether this Job was created at `proposing`: it has frozen no workflow, or
/// its log opens on a move out of `proposing`. Creation has no event, so the
/// log's first row is the only record of where a Job began.
pub(crate) fn born_proposing(frozen: bool, events: &[RecordedEvent]) -> bool {
    !frozen
        || events
            .first()
            .is_some_and(|first| first.under() == JobStatus::Proposing)
}

/// A Job created at `proposing`, rebuilt: the constructor, then the answer
/// where the log opens with one. Answers with the events still to replay.
///
/// **The answer is put back through `Job::answered` from the row's columns**,
/// which the answer wrote, for the reason the fold puts every move through the
/// machine: a history it would not admit fails to rebuild.
pub(crate) fn rebuilt(
    new: NewJob,
    origin: Origin,
    frozen: bool,
    created_at: Timestamp,
    events: &[RecordedEvent],
) -> Result<(Job, &[RecordedEvent]), RowError> {
    let job_id = new.id.clone();
    let top_level = origin
        .top_level()
        .ok_or(RowError::ColumnNotReconstructable {
            job_id: job_id.clone(),
            column: "origin",
            value: origin.as_wire().to_string(),
        })?;
    // A Job at `proposing` is one by being proposed; without the id, the row
    // is one written before its workflow was frozen, which is the old refusal.
    let proposal_id = new.proposal_id.clone().ok_or(RowError::WorkflowNotFrozen {
        job_id: job_id.clone(),
    })?;
    let proposal = NewProposal {
        id: new.id.clone(),
        title: new.title.clone(),
        owner_manifest_id: new.owner_manifest_id.clone(),
        model: new.model.clone(),
        proposal_id,
        number: new.number,
        facts: new.facts.clone(),
        attachments: new.attachments.clone(),
    };
    let created = Job::create_proposing(proposal, top_level, created_at);
    if !frozen {
        return Ok((created.workflow_settled(new.workflow.id().clone()), events));
    }
    let Some((first, rest)) = events.split_first() else {
        return Ok((created, events));
    };
    if !matches!(
        first.moved(),
        Moved::Job {
            to: JobStatus::AwaitingApproval,
            ..
        }
    ) {
        return Ok((created, events));
    }
    let answered = created
        .answered(new.into_answer(), first.actor(), first.at().clone())
        .map_err(|cause| RowError::IllegalRecordedTransition {
            job_id,
            seq: first.seq(),
            cause,
        })?;
    Ok((answered.job, rest))
}
