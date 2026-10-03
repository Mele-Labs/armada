//! A person's edit to a Job at its approval gate. Spike 022, slice 4 (#1641).
//!
//! **The last moment what a Job is held to may move.** Its words, its
//! workflow, each step's gate and its criteria freeze at the approval press
//! (#1581), not at creation, so a person can correct a proposal before
//! anything runs. After the press [`Job::proposal_edited`] refuses, and no
//! other method rewrites any of them.
//!
//! **No event**, for [`Job::on_branch`]'s reason: nothing moved, and the store
//! keeps the columns as their own authority. A child of `record` for its
//! private fields, as `proposing` is.

use crate::envelope::Timestamp;
use crate::job::approval::{NotAtApproval, ProposalEdit};
use crate::job::status::JobStatus;
use crate::job::step::rows_at_creation;

use super::Job;

impl Job {
    /// Replace what a person edited before approving. **Refused anywhere but
    /// `awaiting_approval`**: before it there is nothing yet to edit, and after
    /// it the Job runs what was approved.
    ///
    /// The step rows are made again from `edit.steps`, stamped `at`: none has
    /// started, and a workflow changed at the gate has different steps.
    pub fn proposal_edited(
        &self,
        edit: ProposalEdit,
        at: &Timestamp,
    ) -> Result<Job, NotAtApproval> {
        if self.status != JobStatus::AwaitingApproval {
            return Err(NotAtApproval {
                status: self.status,
            });
        }
        let mut job = self.clone();
        job.title = edit.title;
        job.facts = edit.facts;
        job.workflow = edit.workflow;
        job.workflow_frozen = true;
        job.steps = rows_at_creation(&self.id, edit.steps, at);
        job.acceptance_criteria = edit.acceptance_criteria;
        Ok(job)
    }
}
