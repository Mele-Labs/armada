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

    /// Send the Job back, with the split it belongs to, read it again once and
    /// answer with every Job the group became.
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
        // **The whole split goes back together, or none of it does.** Every
        // Job of it is read before any moves, so a sibling that is past its
        // gate is named and nothing is half-applied.
        let group = self.group_of(&job).await?;
        // What the person sent is read against the proposal they are looking
        // at, before it is given up.
        self.tuning_honourable(&job, body).map_err(refused)?;
        let head = &group[0];
        let served = self.served_by(head)?;
        let proposing = self.proposing().map_err(Adrift::NotProposable)?;
        let Some(proposal_id) = head.proposal_id().cloned() else {
            return Err(Adrift::NothingToPropose);
        };
        let origin = head
            .origin()
            .top_level()
            .unwrap_or(core_model::TopLevelOrigin::Manual);

        let mut sent_back: Vec<Job> = Vec::with_capacity(group.len());
        for member in &group {
            match self.sent_back(member).await {
                Ok(back) => sent_back.push(back),
                Err(cause) => {
                    self.restored(&sent_back, &group).await;
                    return Err(cause);
                }
            }
        }

        let titles: Vec<String> = group
            .iter()
            .map(|member| format!("\"{}\"", member.title().as_str()))
            .collect();
        let request = format!(
            "{}\n\nA person read the proposal for this ({}) and sent it back with this note:\n\n{note}",
            head.facts().as_str(),
            titles.join(", "),
        );
        let (read, _settled) = proposed(
            &request,
            served.workflows(),
            &proposing,
            self.making(Actor::Human).for_job(&proposal_id, head.id()),
            None,
            served.records_root(),
        )
        .await;
        let cause = match read {
            Ok((_, Proposal::Resolved(plan))) if !plan.is_empty() => {
                return self
                    .group_answered(&group, sent_back, plan, body, &served, origin, proposal_id)
                    .await;
            }
            Ok((_, Proposal::Unresolved(Unresolved::ModelNotHeld { named, held }))) => {
                Adrift::ModelNotHeld {
                    request,
                    named,
                    held,
                }
            }
            Ok((_, Proposal::Unresolved(why))) => Adrift::NoWorkflowFits { request, why },
            Ok(_) => Adrift::NotProposed {
                request,
                cause: NotProposed::NamesNoWorkflow,
            },
            Err(cause) => Adrift::NotProposed { request, cause },
        };
        Err(self.put_back(&sent_back, &group, cause).await)
    }

    /// One Job sent back, written and published.
    async fn sent_back(&self, job: &Job) -> Result<Job, Adrift> {
        let back = job
            .sent_back_to_the_proposer(Actor::Human, self.now())
            .map_err(Adrift::IllegalMove)?;
        self.store()
            .lock()
            .await
            .record_transition(&back)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobStateChanged((&back.event).into()));
        Ok(back.job)
    }

    /// The proposer's one answer, laid over the group. **The answer replaces
    /// the group**: its Jobs, in order, are the head and then the old extras,
    /// each kept as the same Job and rewritten whole; an answer with more Jobs
    /// than the group had makes the rest as new extras; an answer with fewer
    /// ends the old extras it has no place for, killed and said in their logs
    /// as replaced. Every rewrite is read before any is written.
    #[allow(clippy::too_many_arguments)]
    async fn group_answered(
        &self,
        group: &[Job],
        sent_back: Vec<Job>,
        plan: Vec<crate::proposing::ProposedJob>,
        body: &ipc::ToProposer,
        served: &crate::repositories::Served,
        origin: core_model::TopLevelOrigin,
        proposal_id: core_model::ProposalId,
    ) -> Result<Vec<Job>, Adrift> {
        let reused = plan.len().min(group.len());
        // Ids are known for the reused Jobs, so an edge can name them before
        // they are written; the new extras are made after and named by order.
        let mut answers = Vec::with_capacity(reused);
        for (at, entry) in plan.iter().enumerate().take(reused) {
            let waits_on = entry
                .after
                .iter()
                .filter_map(|&earlier| group.get(earlier - 1))
                .map(|peer| ipc::DependencyEdge {
                    direction: ipc::DependencyDirection::from(
                        core_model::DependencyDirection::DependsOn,
                    ),
                    peer: ipc::JobId::from(peer.id()),
                })
                .collect();
            let drafted = self.drafted(
                self.as_proposal(served, entry, waits_on, Vec::new(), origin),
                stated_by(entry, false),
                &self.now(),
                Some(proposal_id.clone()),
                group[at].number(),
            );
            match drafted {
                Ok((new, _)) => answers.push(new.into_answer()),
                Err(cause) => return Err(self.put_back(&sent_back, group, cause).await),
            }
        }
        let mut made: Vec<Job> = Vec::with_capacity(plan.len());
        for (at, answer) in answers.into_iter().enumerate() {
            match self.head_answered(&sent_back[at], answer, served).await {
                Ok(job) => made.push(job),
                Err(cause) => {
                    // What was already rewritten stays; what was not goes back.
                    self.restored(&sent_back[at..], &group[at..]).await;
                    return Err(cause);
                }
            }
        }
        for extra in &plan[reused..] {
            let waits_on = extra
                .after
                .iter()
                .filter_map(|&earlier| made.get(earlier - 1))
                .map(|peer| ipc::DependencyEdge {
                    direction: ipc::DependencyDirection::from(
                        core_model::DependencyDirection::DependsOn,
                    ),
                    peer: ipc::JobId::from(peer.id()),
                })
                .collect();
            let minted = self
                .proposed_split(
                    self.as_proposal(served, extra, waits_on, Vec::new(), origin),
                    stated_by(extra, false),
                    proposal_id.clone(),
                    Actor::Human,
                    made[0].id(),
                )
                .await?;
            made.push(minted);
        }
        for surplus in &sent_back[reused..] {
            self.replaced(surplus).await;
        }
        let mut carried = Vec::with_capacity(made.len());
        for rewritten in made {
            carried.push(self.carried_over(rewritten, body).await);
        }
        Ok(carried)
    }

    /// An old extra the answer had no place for, ended as replaced.
    async fn replaced(&self, job: &Job) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the proposer's answer to a note on this split has fewer Jobs, so this one was \
             replaced by the revised proposal and ended",
        )
        .in_job(job.id().as_ulid().clone());
        self.noted_in_the_log(job.id(), &envelope);
        let _ = self
            .move_job(job, core_model::Target::Killed, Actor::Human)
            .await;
    }

    /// Every Job of the split `job` belongs to, the head first and the extras
    /// by number, **each at its gate**: or the one that is not, and why.
    ///
    /// A Job that was dispatched by an Epic's plan step is a wave member, not
    /// a split's, and the Epic's plan owns it. A Job whose `dispatched_by` has
    /// no step is an extra of a split, and its head is the Job it names.
    async fn group_of(&self, job: &Job) -> Result<Vec<Job>, Adrift> {
        let refused = |why| Adrift::ProposalRefused {
            job: job.id().clone(),
            why,
        };
        let head_id = match job.dispatched_by() {
            Some(origin) if origin.step_id.is_some() => return Err(refused(Refused::WaveMember)),
            Some(origin) => origin.job_id.clone(),
            None => job.id().clone(),
        };
        let (loaded, _) = self.every_job().await?;
        let mut extras: Vec<&Job> = loaded
            .jobs
            .iter()
            .filter(|other| {
                other
                    .dispatched_by()
                    .is_some_and(|origin| origin.job_id == head_id)
            })
            .collect();
        if extras.iter().any(|other| {
            other
                .dispatched_by()
                .is_some_and(|origin| origin.step_id.is_some())
        }) {
            return Err(refused(Refused::WaveMember));
        }
        extras.sort_by_key(|other| other.number());
        let head = loaded
            .jobs
            .iter()
            .find(|other| *other.id() == head_id)
            .ok_or_else(|| refused(Refused::WaveMember))?;
        let group: Vec<Job> = std::iter::once(head).chain(extras).cloned().collect();
        match group
            .iter()
            .find(|member| member.status() != JobStatus::AwaitingApproval)
        {
            Some(member) => Err(refused(Refused::SiblingPastItsGate {
                sibling: member.id().as_str().to_string(),
                title: member.title().as_str().to_string(),
                status: member.status(),
            })),
            None => Ok(group),
        }
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

    /// Put the proposals back as they were, after a call that could not be
    /// read, and answer with the proposer's own refusal.
    async fn put_back(&self, sent_back: &[Job], was: &[Job], cause: Adrift) -> Adrift {
        let put = self.restored(sent_back, was).await;
        let (said, level) = match put {
            true => (
                "the proposer could not read the note, so the proposal is as it was",
                Level::Warn,
            ),
            false => (
                "the proposer could not read the note and the proposal could not be put back",
                Level::Error,
            ),
        };
        for job in was {
            let envelope = Envelope::new(
                self.now(),
                level,
                Component::Fleet,
                self.run().clone(),
                said,
            )
            .in_job(job.id().as_ulid().clone())
            .with_field("cause", FieldValue::Str(cause.to_string()));
            self.noted_in_the_log(job.id(), &envelope);
        }
        cause
    }

    /// Each Job sent back, answered with what it held. Whether all were.
    async fn restored(&self, sent_back: &[Job], was: &[Job]) -> bool {
        let mut all = true;
        for (back, old) in sent_back.iter().zip(was) {
            let restored = back.answered(old.as_answered(), Actor::Fleet, self.now());
            let written = match restored {
                Ok(restored) => {
                    let written = self.store().lock().await.record_answered(&restored);
                    if written.is_ok() {
                        self.publish(ipc::Event::JobStateChanged((&restored.event).into()));
                    }
                    written.is_ok()
                }
                Err(_) => false,
            };
            all &= written;
        }
        all
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
