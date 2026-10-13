//! Words to a Job's Drone, a restart, and the work sent back with a note. Each is one line in
//! `commanding`'s `Commands` impl, which cannot be split across files and reached the 1200-line
//! rule.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ChangesRequested, JobId, JobSummary, Redirection};

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;
// The wire's `Redirection` is a struct with a public field; Fleet's is a
// newtype that cannot hold an empty instruction. Both names are in scope here,
// which is the one place they meet.
use crate::resume::Redirection as Instruction;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The work goes back with a note, to the Drone that is standing at the
    /// gate.
    ///
    /// **An empty note is refused here rather than sent**, for the reason
    /// `redirect_drone` refuses one: a Drone told nothing at all resumes with
    /// exactly the information that was not enough, which is the review
    /// appearing to work and changing nothing.
    pub(super) async fn changes_requested(
        self: Arc<Self>,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        // Walk notes are read before the blank check, so a blank note carrying
        // them is taken and one carrying none is refused as it always was.
        let id = job_id.to_domain();
        let walked = self.with_walk_notes(&id, &note).await;
        let (text, carried) = walked.map_err(|why| self.refusal(why))?;
        let said = Instruction::saying(&text).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::request_changes(&fleet, &job_id.to_domain(), &said).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        // Only once the note has landed, so a refused send leaves them unsent.
        let sent = self.walk_notes_sent(&id, &carried).await;
        sent.map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A structured instruction to a Drone that is escalated and idle.
    ///
    /// **An empty instruction is refused here rather than sent.** A Drone told
    /// nothing at all resumes the step it stopped on with exactly the
    /// information that failed, which is the redirect appearing to work and
    /// changing nothing. `Redirection::saying` is where the emptiness is
    /// caught; this only carries the refusal out.
    pub(super) async fn drone_redirected(
        self: Arc<Self>,
        job_id: JobId,
        instruction: Redirection,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        let said = Instruction::saying(&instruction.instruction)
            .ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.redirect(&job_id.to_domain(), &said, by).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Words to one Drone of the Job, by id: `crate::one_drone`.
    pub(super) async fn one_drone_redirected(
        self: Arc<Self>,
        job_id: JobId,
        drone_id: ipc::DroneId,
        instruction: Redirection,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        let said = Instruction::saying(&instruction.instruction)
            .ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let drone = drone_id.to_domain();
            async move {
                Fleet::redirect_one_drone(&fleet, &job_id.to_domain(), &drone, &said, by).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A fresh Drone on the worktree the last one left, and what to do
    /// differently where a person had something to say.
    ///
    /// The Job resumes at the step that stopped; every earlier step's work is
    /// on the branch and is not redone. That is what separates this from a
    /// redispatch, which starts a replacement Job at the approval gate.
    ///
    /// **No note and a blank note are different requests.** Absent is the plain
    /// restart, which is what this act has always been and stays. Present and
    /// empty is refused here rather than written down, for the reason
    /// `redirect_drone` refuses one: a Drone opened with a heading and nothing
    /// under it has been given exactly the information that was not enough.
    pub(super) async fn step_restarted(
        self: Arc<Self>,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        let said = match &note {
            Some(note) => Some(
                Instruction::saying(&note.note).ok_or_else(|| self.refusal(Adrift::Unnameable))?,
            ),
            None => None,
        };
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                // Signed as the caller the transport placed, `restart_task`'s
                // mapping: a person's, Helm's or a Commander's.
                let (signed, _) = crate::work_plan::signed(by);
                Fleet::restart_step_by(
                    &fleet,
                    &job_id.to_domain(),
                    said.as_ref(),
                    crate::resume::Ending::Unheard,
                    signed,
                )
                .await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }
}
