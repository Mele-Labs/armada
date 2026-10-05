//! A proposal sent back to the proposer with a note, from the approval gate.
//! The owner, 4 Oct 2026. Since 23.25.
//!
//! **The Job goes `awaiting_approval -> proposing`, is read again, and comes
//! back.** The proposer is given the request and the note, and its answer
//! replaces the words, the workflow and the criteria whole, as the first
//! answer did. It may split: each Job it makes comes to the gate on its own.
//!
//! **What the person set carries over** where it can: their tuning per step
//! wherever the new workflow still has a step by that id, and their landing as
//! it stands. What cannot carry is dropped and said in the Job's log, never
//! refused, since the person asked for a different proposal and a refusal now
//! would throw it away.
//!
//! **A call that fails leaves the proposal as it was**: the Job goes back to
//! the gate holding what it held, and the refusal is the proposer's own. This
//! differs from a first dispatch, whose failure has no earlier proposal to
//! return to. A Fleet that restarts mid-call escalates the Job as
//! `proposer_failed`, as it does a first dispatch.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Actor, Component, Envelope, FieldValue, Job, JobId, JobStatus, Level};

use crate::adrift::Adrift;
use crate::approving::Refused;
use crate::daemon::Fleet;
use crate::proposal::{proposed, stated_by};
use crate::proposing::{NotProposed, Proposal, Unresolved};

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
    /// `to_proposer`, from Bridge. **On a task of its own**, `propose_from_request`'s
    /// reason: a caller that stops waiting drops this future, and the call and
    /// the move back to the gate have to go on without it.
    pub(crate) async fn to_proposer_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        body: ipc::ToProposer,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        let fleet = Arc::clone(&self);
        let id = job_id.to_domain();
        let made = tokio::spawn(async move { fleet.reproposed(&id, &body).await })
            .await
            .unwrap_or(Err(Adrift::ProposalAbandoned))
            .map_err(|why| self.refusal(why))?;
        let mut jobs = Vec::with_capacity(made.len());
        for job in &made {
            jobs.push(self.summarised(job).await?);
        }
        Ok(ipc::ProposedPlan { jobs })
    }

    /// Send the Job back, read it again and answer with every Job it became.
    pub(crate) async fn reproposed(
        &self,
        job_id: &JobId,
        body: &ipc::ToProposer,
    ) -> Result<Vec<Job>, Adrift> {
        let refused = |why| Adrift::ProposalRefused {
            job: job_id.clone(),
            why,
        };
        let note = body.note.trim();
        if note.is_empty() {
            return Err(refused(Refused::NoNote));
        }
        let job = self.load(job_id).await?;
        if job.status() != JobStatus::AwaitingApproval {
            return Err(refused(Refused::Frozen(job.status())));
        }
        if job.dispatched_by().is_some() || self.was_split(&job).await? {
            return Err(refused(Refused::SplitAlready));
        }
        // What the person sent is read against the proposal they are looking
        // at, before it is given up.
        self.tuning_honourable(&job, body).map_err(refused)?;
        let served = self.served_by(&job)?;
        let proposing = self.proposing().map_err(Adrift::NotProposable)?;
        let Some(proposal_id) = job.proposal_id().cloned() else {
            return Err(Adrift::NothingToPropose);
        };

        let origin = job
            .origin()
            .top_level()
            .unwrap_or(core_model::TopLevelOrigin::Manual);
        let back = job
            .sent_back_to_the_proposer(Actor::Human, self.now())
            .map_err(Adrift::IllegalMove)?;
        self.store()
            .lock()
            .await
            .record_transition(&back)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobStateChanged((&back.event).into()));

        let request = format!(
            "{}\n\nA person read the proposal \"{}\" for this and sent it back with this note:\n\n{note}",
            job.facts().as_str(),
            job.title().as_str(),
        );
        let (read, _settled) = proposed(
            &request,
            served.workflows(),
            &proposing,
            self.making(Actor::Human).for_job(&proposal_id, job.id()),
            None,
            served.records_root(),
        )
        .await;
        let plan = match read {
            Ok((_, Proposal::Resolved(plan))) if !plan.is_empty() => plan,
            Ok((_, Proposal::Unresolved(Unresolved::ModelNotHeld { named, held }))) => {
                let cause = Adrift::ModelNotHeld {
                    request,
                    named,
                    held,
                };
                return Err(self.put_back(&back.job, &job, cause).await);
            }
            Ok((_, Proposal::Unresolved(why))) => {
                let cause = Adrift::NoWorkflowFits { request, why };
                return Err(self.put_back(&back.job, &job, cause).await);
            }
            Ok(_) => {
                let cause = Adrift::NotProposed {
                    request,
                    cause: NotProposed::NamesNoWorkflow,
                };
                return Err(self.put_back(&back.job, &job, cause).await);
            }
            Err(cause) => {
                let cause = Adrift::NotProposed { request, cause };
                return Err(self.put_back(&back.job, &job, cause).await);
            }
        };
        let Some((first, extras)) = plan.split_first() else {
            unreachable!("a plan with no Job in it was refused above");
        };
        // The rewrite, whole. **Its facts are the proposer's**: the request was
        // already the brief, and the note is what moved it.
        let answered = self
            .drafted(
                self.as_proposal(&served, first, Vec::new(), Vec::new(), origin),
                stated_by(first, false),
                &self.now(),
                Some(proposal_id.clone()),
                job.number(),
            )
            .map(|(new, _)| new.into_answer());
        let head = match answered {
            Ok(answer) => self.head_answered(&back.job, answer, &served).await,
            Err(cause) => Err(cause),
        };
        let head = match head {
            Ok(head) => head,
            Err(cause) => return Err(self.put_back(&back.job, &job, cause).await),
        };
        let mut made: Vec<Job> = Vec::with_capacity(plan.len());
        made.push(head);
        for extra in extras {
            let waits_on = extra
                .after
                .iter()
                .map(|&at| ipc::DependencyEdge {
                    direction: ipc::DependencyDirection::from(
                        core_model::DependencyDirection::DependsOn,
                    ),
                    peer: ipc::JobId::from(made[at - 1].id()),
                })
                .collect();
            let minted = self
                .proposed_split(
                    self.as_proposal(&served, extra, waits_on, Vec::new(), origin),
                    stated_by(extra, false),
                    proposal_id.clone(),
                    Actor::Human,
                    made[0].id(),
                )
                .await?;
            made.push(minted);
        }
        let mut carried = Vec::with_capacity(made.len());
        for rewritten in made {
            carried.push(self.carried_over(rewritten, body).await);
        }
        Ok(carried)
    }

    /// Whether other Jobs name this one as the head of a split or an Epic.
    async fn was_split(&self, job: &Job) -> Result<bool, Adrift> {
        let (loaded, _) = self.every_job().await?;
        Ok(loaded.jobs.iter().any(|other| {
            other
                .dispatched_by()
                .is_some_and(|origin| origin.job_id == *job.id())
        }))
    }

    /// The tuning and landing a person sent, read against the Job as it
    /// stands, so a body nothing could honour is refused before the Job moves.
    fn tuning_honourable(&self, job: &Job, body: &ipc::ToProposer) -> Result<(), Refused> {
        let tuning = body.tuning.as_deref().unwrap_or_default();
        crate::tuned::harnesses_held(tuning, &self.models().harnesses).map_err(Refused::Untuned)?;
        crate::tuned::tuned(job.workflow().clone(), tuning).map_err(Refused::Untuned)?;
        for named in tuning.iter().filter_map(|step| step.model.as_deref()) {
            if self.offered(named.trim()).is_err() {
                return Err(Refused::BlankModel);
            }
        }
        crate::approving::landing_of(body.landing.as_ref()).map(|_| ())
    }

    /// Put the proposal back as it was, after a call that could not be read,
    /// and answer with the proposer's own refusal.
    async fn put_back(&self, sent_back: &Job, was: &Job, cause: Adrift) -> Adrift {
        let restored = sent_back.answered(was.as_answered(), Actor::Fleet, self.now());
        let written = match restored {
            Ok(restored) => {
                let written = self.store().lock().await.record_answered(&restored);
                if written.is_ok() {
                    self.publish(ipc::Event::JobStateChanged((&restored.event).into()));
                }
                written.map(|_| ()).map_err(Adrift::Writing)
            }
            Err(illegal) => Err(Adrift::IllegalMove(illegal)),
        };
        let (said, level) = match written {
            Ok(()) => (
                "the proposer could not read the note, so the proposal is as it was",
                Level::Warn,
            ),
            Err(_) => (
                "the proposer could not read the note and the proposal could not be put back",
                Level::Error,
            ),
        };
        let envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(was.id().as_ulid().clone())
        .with_field("cause", FieldValue::Str(cause.to_string()));
        self.noted_in_the_log(was.id(), &envelope);
        cause
    }

    /// Lay what the person set on a rewritten Job: their tuning for each step
    /// the rewrite still has, and their landing. **What cannot carry is said in
    /// the Job's log and dropped.** Answers with the Job as it ended up.
    async fn carried_over(&self, rewritten: Job, body: &ipc::ToProposer) -> Job {
        let sent = body.tuning.as_deref().unwrap_or_default();
        let (kept, dropped): (Vec<_>, Vec<_>) = sent.iter().cloned().partition(|step| {
            crate::tuned::tuned(rewritten.workflow().clone(), std::slice::from_ref(step)).is_ok()
        });
        let mut landing_dropped = false;
        let mut job = rewritten;
        if !kept.is_empty() || body.landing.is_some() {
            let with_landing = ipc::ApproveDispatch {
                tuning: Some(kept.clone()),
                landing: body.landing.clone(),
                ..ipc::ApproveDispatch::default()
            };
            match self.kept_as_left(job.id(), &with_landing, CARRIED).await {
                Ok(laid) => job = laid,
                // The landing named a branch this repository lacks, or the like:
                // the tuning still carries.
                Err(_) if body.landing.is_some() => {
                    landing_dropped = true;
                    let without = ipc::ApproveDispatch {
                        landing: None,
                        ..with_landing
                    };
                    if let Ok(laid) = self.kept_as_left(job.id(), &without, CARRIED).await {
                        job = laid;
                    }
                }
                Err(_) => {}
            }
        }
        if !dropped.is_empty() || landing_dropped {
            let mut envelope = Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                "part of what the person set did not carry over to the rewritten proposal",
            )
            .in_job(job.id().as_ulid().clone());
            if !dropped.is_empty() {
                let steps: Vec<&str> = dropped.iter().map(|step| step.step_id.as_str()).collect();
                envelope = envelope.with_field("tuning_dropped", FieldValue::Str(steps.join(", ")));
            }
            if landing_dropped {
                envelope = envelope.with_field("landing", FieldValue::Str("dropped".to_string()));
            }
            self.noted_in_the_log(job.id(), &envelope);
        }
        job
    }
}

/// The Job's log line where the person's settings were laid on a rewrite.
const CARRIED: &str = "what a person set was carried over to the rewritten proposal";
