//! The Job a dispatched request is, from the press to wherever the proposer's
//! call leaves it. #1714, #1716.
//!
//! **Created before the call goes out**, at `proposing` with the request as
//! its title, as the owner decided on 30 Sep 2026.
//! The call's ending then moves it: an answer freezes the head of the plan into
//! it, a person's stop kills it, and every other ending escalates it as
//! `no_workflow_fits` or `proposer_failed`, with the Job's log saying which.
//!
//! A restart folds into `proposer_failed` too: the call lives in this process
//! and nothing makes it again, so a Job left at `proposing` is moved at boot.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, Facts, FieldValue, Job, JobId, JobStatus, Level,
    NewProposal, ProposalId, Target, Title, TopLevelOrigin, WorkflowId,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::judging::CallFailed;
use crate::proposing::NotProposed;

/// What a call ended as, as far as the Job it was reading for is concerned.
enum Ended {
    /// A person stopped it: `proposing -> killed`, the act they took.
    Stopped,
    /// `proposing -> escalated`, and the sentence its log carries.
    Escalated {
        trigger: EscalationTrigger,
        said: &'static str,
        fields: Vec<(&'static str, String)>,
    },
}

/// The move and the log line one refusal of the proposal path means.
///
/// **`model_not_held` folds into `proposer_failed`**, the owner's answer of 1
/// Oct 2026: on a Job that already exists the acts are a fault's — dispatch
/// again, or change the machine — and the route still answers its own code.
fn ended(cause: &Adrift) -> Ended {
    let escalated = |trigger, said, fields| Ended::Escalated {
        trigger,
        said,
        fields,
    };
    match cause {
        Adrift::NotProposed {
            cause: NotProposed::Call(CallFailed::Stopped),
            ..
        } => Ended::Stopped,
        Adrift::NoWorkflowFits { why, .. } => escalated(
            EscalationTrigger::NoWorkflowFits,
            "the proposer read the request and no workflow this repository holds fits it",
            vec![("why", why.to_string())],
        ),
        Adrift::ModelNotHeld { named, held, .. } => escalated(
            EscalationTrigger::ProposerFailed,
            "the proposer named a model this machine does not run",
            vec![("model", named.clone()), ("models", held.join(", "))],
        ),
        Adrift::NotProposed {
            cause: NotProposed::Unreadable { kept_at, .. },
            ..
        } => escalated(
            EscalationTrigger::ProposerFailed,
            "the proposer answered twice and neither answer could be read",
            kept_at
                .iter()
                .map(|path| ("kept_at", path.clone()))
                .collect(),
        ),
        Adrift::NotProposed { cause, .. } => escalated(
            EscalationTrigger::ProposerFailed,
            "the proposer's call could not be made",
            vec![("cause", cause.to_string())],
        ),
        // Reachable only after the reading — a workflow edited away while the
        // call was out, or the store refusing the answer. The answer could not
        // be turned into a Job, which is this trigger's meaning.
        other => escalated(
            EscalationTrigger::ProposerFailed,
            "the proposer's answer could not be turned into a Job",
            vec![("cause", other.to_string())],
        ),
    }
}

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
    /// The Job a request is the moment it is dispatched: at `proposing`, the
    /// request as its title and its brief, the attachments already kept.
    ///
    /// Number and insert under one store lock, for `proposed_job`'s reason.
    pub(crate) async fn proposing_job(
        &self,
        request: &str,
        attachments: Vec<ipc::AttachmentRef>,
        served: &crate::repositories::Served,
        dispatched_as: TopLevelOrigin,
        proposal: &ProposalId,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let at = self.now();
        let title = Title::new(request).map_err(|_| Adrift::NothingToPropose)?;
        let model = self.the_model_named(None)?;
        let id = JobId::carried(self.mint().ulid());
        let attachments = self.promoted(&id, attachments)?;
        let mut store = self.store().lock().await;
        let number = store
            .next_job_number(served.manifest().id())
            .map_err(Adrift::Reading)?;
        let new = NewProposal {
            id,
            title,
            owner_manifest_id: served.manifest().id().clone(),
            model,
            proposal_id: proposal.clone(),
            number,
            facts: Facts::new(request),
            attachments,
        };
        let job = Job::create_proposing(new, dispatched_as, at.clone());
        store.insert_job(&job, &at).map_err(Adrift::Writing)?;
        self.learn_the_name(&job);
        self.manifest_snapshotted(&mut store, &job).await;
        drop(store);
        self.publish(ipc::Event::JobCreated(ipc::JobCreated {
            job: ipc::JobSummary::from(&job),
            actor: by.into(),
            at: (&at).into(),
        }));
        Ok(job)
    }

    /// Freeze the head of the plan into the Job that was being proposed, and
    /// cross it to `awaiting_approval`. Its identity — id, number, attachments,
    /// origin — is the dispatch's and stays.
    ///
    /// **The title changes, so the handle does**, and the Job's log written
    /// under the old one is carried across. Nothing else is named by it yet:
    /// no worktree, no branch and no transcript exist before the gate.
    pub(crate) async fn head_answered(
        &self,
        head: &Job,
        answer: core_model::Answered,
        served: &crate::repositories::Served,
    ) -> Result<Job, Adrift> {
        let moved = head
            .answered(answer, Actor::Fleet, self.now())
            .map_err(Adrift::IllegalMove)?;
        self.store()
            .lock()
            .await
            .record_answered(&moved)
            .map_err(Adrift::Writing)?;
        let (was, is) = (head.handle(), moved.job.handle());
        if was != is {
            let root = served.records_root();
            let _ = std::fs::rename(
                crate::transcript::log_of(root, &was),
                crate::transcript::log_of(root, &is),
            );
        }
        self.learn_the_name(&moved.job);
        self.publish(ipc::Event::JobStateChanged((&moved.event).into()));
        Ok(moved.job)
    }

    /// Move the Job a call was reading for to where its ending leaves it, and
    /// answer with the refusal unchanged — the route still says what it said.
    ///
    /// **Best-effort on the move.** A store that refused the answer may refuse
    /// this too; the Job then stays at `proposing` and the next boot moves it.
    pub(crate) async fn proposal_ended(
        &self,
        head: &Job,
        settled: Option<WorkflowId>,
        cause: Adrift,
        by: Actor,
    ) -> Adrift {
        let head = match settled {
            Some(workflow) => {
                let kept = head.workflow_settled(workflow);
                let written = self.store().lock().await.record_workflow_settled(&kept);
                match written {
                    Ok(()) => kept,
                    Err(_) => head.clone(),
                }
            }
            None => head.clone(),
        };
        let _ = match ended(&cause) {
            Ended::Stopped => self.move_job(&head, Target::Killed, by).await,
            Ended::Escalated {
                trigger,
                said,
                fields,
            } => {
                self.noted_proposal_ended(head.id(), said, fields);
                self.move_job(&head, Target::Escalated(trigger), Actor::Fleet)
                    .await
            }
        };
        cause
    }

    /// Every Job a dead Fleet left at `proposing`, escalated as
    /// `proposer_failed`. Its call lived in that process and nothing makes it
    /// again, so a row left reading `proposing` is a wait on nothing.
    pub(crate) async fn proposals_orphaned(&self, jobs: &mut [Job]) -> Result<Vec<JobId>, Adrift> {
        let mut orphaned = Vec::new();
        for slot in jobs.iter_mut() {
            if slot.status() != JobStatus::Proposing {
                continue;
            }
            self.noted_proposal_ended(
                slot.id(),
                "Fleet restarted while the proposer's call was out, and the call did not survive it",
                Vec::new(),
            );
            let job = self
                .move_job(
                    slot,
                    Target::Escalated(EscalationTrigger::ProposerFailed),
                    Actor::Fleet,
                )
                .await?;
            orphaned.push(job.id().clone());
            *slot = job;
        }
        Ok(orphaned)
    }

    fn noted_proposal_ended(
        &self,
        job: &JobId,
        said: &'static str,
        fields: Vec<(&'static str, String)>,
    ) {
        let envelope = fields.into_iter().fold(
            Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                said,
            )
            .in_job(job.as_ulid().clone()),
            |envelope, (name, value)| envelope.with_field(name, FieldValue::Str(value)),
        );
        self.noted_in_the_log(job, &envelope);
    }
}
