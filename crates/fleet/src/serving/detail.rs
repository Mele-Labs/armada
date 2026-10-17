//! One Job in full: the read Bridge makes on every open of one.
//!
//! **Split out of [`serving`](mod@crate::serving) when that file crossed the
//! 900-line rule.** This is one quarter of it and one read — the biggest thing
//! in the crate's redaction layer — and it is still that layer: every value
//! below is put onto an `ipc` DTO by hand, `crate::serving`'s own rule.
//!
//! The trait method is next door and delegates here, because `Queries` is one
//! `impl` block and a trait implementation cannot be split across two.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobDelivery, JobDetail, JobId, JobReview, JobSpend};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::{step_facts, step_moves};

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
    /// One Job in full, folded from its log like every other read.
    ///
    /// **A read of the record and nothing else.** Jobs are no longer dispatched or worked, so
    /// everything a live Job carried in memory — a Drone's wait, a question, a Check under way,
    /// what was stuck — is absent, and the rest is what the store holds.
    pub(crate) async fn job_detail(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let (
            flagged,
            moves,
            ran_every_attempt,
            judged_every_attempt,
            resolved,
            frames,
            started_at,
            ended_at,
        ) = {
            let store = self.store().lock().await;
            // Every attempt's, for `judged_every_attempt`'s reason below: a
            // flag that refused an earlier run belongs to that run's record.
            let flagged = store
                .step_gaming_flags_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // The rows `get_job_events` serves, narrowed to the step moves.
            // **Read on every open, unlike the history**: one entry per run of
            // a step rather than a row per move, so a rail can say `Attempt 1
            // refused` without the unbounded read `history.rs` keeps off this.
            let moves =
                step_moves(&store, job.id()).map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // **Every attempt's rows, beside the latest-only `ran` above.**
            // `ran` stays latest-only because `why_stuck` below reads it as
            // that; `step_facts` wants every run's Checks and Judge answers
            // stamped with the attempt they belong to, which these two give.
            let ran_every_attempt = store
                .step_checks_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let judged_every_attempt = store
                .step_judgments_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // What each run's gate resolved its policies to. #1683.
            let resolved = store
                .resolved_policies_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // Every attempt's, for `ran_every_attempt`'s reason: a person
            // comparing the run that was handed back against the one that
            // passed needs both, and the rail stamps each with its run.
            let frames = store
                .step_frames_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // The same log, narrowed the other way: the Job's own first
            // arrival at `running`, beside the step moves above.
            let started_at = crate::wire::job_started_at(&store, job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            // And its one arrival at a terminal status, where it has ended —
            // `started_at`'s own reason, and `JobSummary::ended_at`'s.
            let ended_at = crate::wire::job_ended_at(&store, job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (
                flagged,
                moves,
                ran_every_attempt,
                judged_every_attempt,
                resolved,
                frames,
                started_at,
                ended_at,
            )
        };
        // Read for every Job, not only a finished one. Delivery is written when
        // the Job *enters* its delivering step — `crate::landing`'s module
        // doc, since #520 — so a Job holding at its handoff gate has a record
        // here well before it reaches a terminal status, and that is exactly
        // the read the gate wants: the pull request it is meant to review.
        let came_to = self
            .store()
            .lock()
            .await
            .delivery_for(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        // Absent rather than three nulls: a Job with no delivering step, or one
        // that has not reached its delivering step yet, is not a Job whose
        // branch came to nothing, and the surface says different sentences for
        // the two.
        let delivery = match came_to.is_empty() {
            true => None,
            false => {
                // **Off the record, so it outlives the merge** where the
                // reading above does not. Only beside an address: a title
                // with no pull request to click names nothing.
                let kept = match came_to.pull_request.is_some() {
                    true => self
                        .store()
                        .lock()
                        .await
                        .kept_pull_request_for(job.id())
                        .map_err(|why| self.refusal(Adrift::Reading(why)))?,
                    false => store::KeptPullRequest::default(),
                };
                // Only beside a merge: nothing else has an instant to say.
                let merged_at = match came_to.landed {
                    Some(adapter_traits::Landing::Merged { .. }) => self
                        .store()
                        .lock()
                        .await
                        .merged_at(job.id())
                        .map_err(|why| self.refusal(Adrift::Reading(why)))?,
                    _ => None,
                };
                Some(JobDelivery {
                    commit: came_to.commit,
                    pushed: came_to.pushed,
                    pull_request: came_to.pull_request,
                    pull_request_detail: None,
                    landed: came_to.landed.as_ref().and_then(crate::attention::settled),
                    unpushed: came_to.unpushed,
                    pull_request_title: kept.title,
                    pull_request_comments: kept.comments,
                    merged_at: merged_at.as_ref().map(ipc::Instant::from),
                    pull_request_opened_at: kept.opened_at.map(ipc::Instant::carried),
                })
            }
        };
        // Read for every Job, like `delivery` above and for the same reason:
        // a workflow with no delivering step still stops for a person at its
        // own gate, and the review area wants what Fleet composed there just
        // as much as a gate that opens a pull request does. `#665`.
        let composed = self
            .store()
            .lock()
            .await
            .review_for(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let review = match (
            composed.why,
            composed.outcome,
            composed.risks,
            composed.evidence,
        ) {
            (Some(why), Some(outcome), Some(risks), Some(evidence)) => Some(JobReview {
                why,
                outcome,
                risks,
                evidence,
            }),
            // **All four or none**, `store::Review::is_empty`'s own shape: a
            // row that somehow held three of the four would still be a Job
            // that has not reached a gate, for anything a surface could do
            // with it.
            _ => None,
        };
        let allowance = self.allowance_for(&job);
        let spent = self
            .spend_of(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let spend = Some(JobSpend {
            cost_micros: spent.cost_micros,
            cost_cap_micros: allowance.cost().count(),
            unpriced: spent.unpriced,
            turns: spent.turns,
            turn_cap: allowance.turns(),
            ran_ms: spent.ran_ms,
            drones: spent.drones,
            commander_cost_micros: 0,
        });
        let mut detail = JobDetail::of(
            &job,
            reason.as_ref(),
            None,
            None,
            None,
            started_at,
            ended_at,
            &step_facts(
                &job,
                ran_every_attempt,
                judged_every_attempt,
                flagged,
                resolved,
                frames,
                &moves,
            ),
            None,
            None,
            None,
            None,
            None,
            delivery,
            spend,
            review,
        );
        detail.job.bays = crate::summarising::bays_on_row(&*self.store().lock().await, &job);
        // The frozen set and every firing, joined: a Trigger no moment has
        // reached is `pending`.
        detail.triggers = {
            let store = self.store().lock().await;
            let frozen = store
                .frozen_triggers(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let firings = store
                .trigger_firings(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            crate::job_triggers(&frozen, &firings)
        };
        let phoned = self
            .store()
            .lock()
            .await
            .created_from_phone(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(why))))?;
        detail.created_via = phoned.then(|| core_model::Via::Phone.into());
        // One indexed read on every open, because a Job that was replaced is
        // otherwise a dead end — `store::lineage`. The handle is composed here
        // rather than in the store, `core_model` owning that one string.
        detail.replaced_by = self
            .store()
            .lock()
            .await
            .replaced_by(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?
            .map(|found| ipc::ReplacedBy {
                job_id: ipc::JobId::from(&found.job_id),
                handle: core_model::handle_of(found.number, &found.title),
            });
        // The same column followed forwards, so the header can name what this
        // Job replaced rather than print its id — `#1474`.
        detail.replaces = self
            .store()
            .lock()
            .await
            .replaces(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?
            .map(|found| ipc::Replaces {
                job_id: ipc::JobId::from(&found.job_id),
                handle: core_model::handle_of(found.number, &found.title),
            });
        // The way back to the Studio that dispatched it — `#1362`. Derived
        // from the `produced` edge, so a deleted Studio answers nothing here
        // and `job.origin` is what still says the Job came off one.
        detail.from_studio = self
            .store()
            .lock()
            .await
            .dispatched_from(job.id())
            .map_err(|why| self.studio_refusal(why))?
            .map(|found| ipc::FromStudio {
                studio_id: ipc::StudioId::from(&found.studio_id),
                name: found.name.map(|name| name.as_str().to_string()),
                node_id: ipc::StudioNodeId::from(&found.node_id),
            });
        Ok(detail)
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
    /// What a Job's worktree holds, where it still has one.
    ///
    /// **`None` is "there is nothing to read"** — one whose worktree has been reclaimed, or whose
    /// slot is no longer its own — and it is a different answer from a reading that found no
    /// change. A directory that will not open is an error rather than either.
    pub(crate) fn worktree_of(
        &self,
        job: &core_model::Job,
    ) -> Result<Option<adapter_traits::Worktree>, Adrift> {
        let served = self.served_by(job)?;
        let Some(spec) = self
            .job_tree(&served, job)
            .map_err(|cause| Adrift::Unworkable {
                job: job.id().clone(),
                cause,
            })?
            .here()
        else {
            return Ok(None);
        };
        if !std::path::Path::new(&spec.worktree_path()).is_dir() {
            return Ok(None);
        }
        let branch = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .unwrap_or_else(|| spec.branch());
        // **The Manifest's base rides along**, because a patch measured against anything else is a
        // smaller claim than the Job's work and nothing downstream can tell.
        Ok(Some(self.based(
            &served,
            adapter_traits::Worktree::at(spec.worktree_path(), branch),
        )))
    }
}
