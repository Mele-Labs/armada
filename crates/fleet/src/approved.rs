//! What Fleet does with a proposal a person left: keeps it whole, releases it,
//! lands it where they chose, and serves it back. Spike 022, slice 4.
//!
//! **`crate::approving` decides and this applies.** Everything it asks of the
//! machine — a model `list_models` offers, a branch the repository holds — is
//! asked before anything is written, so a refused approval keeps nothing.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    AdvanceGate, Component, Envelope, FieldValue, Job, JobId, JobStatus, Landing, Level,
    ProposalEdit,
};

use crate::adrift::Adrift;
use crate::approving::{decided, edited, Decided, Refused};
use crate::daemon::Fleet;

/// Which branch each Job's worktree was cut from, keyed by the Job's branch,
/// where a person chose one other than the base.
///
/// **In memory, learned at dispatch and again at boot**, for `crate::naming`'s
/// reason: `crate::basing`'s `based` puts the base on a worktree from a
/// served repository and the worktree alone, at a dozen sites with no Job in
/// hand, and a work product measured from the Manifest's base when the work
/// started somewhere else would claim every commit between the two as this
/// Job's. The store's `job_landing` row is the authority; this is its index.
#[derive(Debug, Default)]
pub struct CutFrom(std::sync::Mutex<BTreeMap<String, String>>);

impl CutFrom {
    pub(crate) fn learn(&self, branch: &str, from: &str) {
        self.held().insert(branch.to_string(), from.to_string());
    }

    /// The branch `branch`'s worktree was cut from, or `None` where it was
    /// cut from the base.
    pub(crate) fn of(&self, branch: &str) -> Option<String> {
        self.held().get(branch).cloned()
    }

    fn held(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        // A panic while the map was held leaves a map, not a broken one.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    /// Approve a Job as a person left it: its proposal rewritten and its
    /// settings kept, **then** the press. Every refusal is answered before
    /// anything is written, so a refused approval leaves the proposal as it
    /// was and the Job at its gate.
    pub async fn approve_as_left(
        &self,
        job_id: &JobId,
        body: &ipc::ApproveDispatch,
    ) -> Result<Job, Adrift> {
        // Before the proposal is kept, so a refused press keeps nothing.
        self.not_alone(&self.load(job_id).await?)?;
        let job = self.kept_as_left(job_id, body, APPROVED).await?;
        self.approve(job.id()).await
    }

    /// Refused where a dispatch setting is one nothing could keep: a workflow
    /// the repository does not hold, a model this machine does not offer, a
    /// cap of zero. **Asked before the request becomes a Job.**
    pub(crate) fn settable(
        &self,
        served: &crate::repositories::Served,
        settings: &ipc::DispatchSettings,
    ) -> Result<(), Adrift> {
        if let Some(named) = &settings.workflow_id {
            if served.workflows().get(&named.to_domain()).is_none() {
                return Err(Adrift::NoSuchWorkflow {
                    named: named.as_str().to_string(),
                    held: served
                        .workflows()
                        .keys()
                        .map(|id| id.as_str().to_string())
                        .collect(),
                });
            }
        }
        if let Some(tiers) = &settings.tiers {
            for (_, named) in tiers.named() {
                if self.offered(named).is_err() {
                    return Err(Adrift::ModelNotHeld {
                        request: String::new(),
                        named: named.to_string(),
                        held: self.models().models.clone(),
                    });
                }
            }
        }
        if settings.drone_cap == Some(0) {
            return Err(Adrift::Unnameable);
        }
        Ok(())
    }

    /// What a person set while typing the request, laid on each Job it became
    /// once the proposer has answered. **The approval's own path without the
    /// press**: the Jobs are at their gate, and stay there.
    pub(crate) async fn dispatched_as_set(
        &self,
        made: &[Job],
        settings: &ipc::DispatchSettings,
    ) -> Result<(), Adrift> {
        for job in made {
            let served = self.served_by(job)?;
            let workflows = served.workflows();
            let held = settings
                .workflow_id
                .as_ref()
                .and_then(|named| workflows.get(&named.to_domain()))
                .map(|held| held.frozen().clone());
            let workflow = held.as_ref().unwrap_or(job.workflow());
            let gates = match (settings.lands, workflow.delivering_step()) {
                (Some(lands), Some(step)) => vec![ipc::GateChoice {
                    step_id: step.id().into(),
                    checks: true,
                    judge: step.asks_the_judge(),
                    you: lands == ipc::LandsWhen::YouAtReview,
                    overridden: matches!(
                        step.advance_gate(),
                        AdvanceGate::ManifestRuleAutoMerge | AdvanceGate::ManifestRuleReviewGate
                    ),
                }],
                _ => Vec::new(),
            };
            let body = ipc::ApproveDispatch {
                workflow_id: settings.workflow_id.clone(),
                gates: Some(gates),
                tiers: settings.tiers.clone(),
                drone_cap: settings.drone_cap,
                ..ipc::ApproveDispatch::default()
            };
            self.kept_as_left(job.id(), &body, SET_AT_DISPATCH).await?;
        }
        Ok(())
    }

    /// [`approve_as_left`](Fleet::approve_as_left) up to the press: read,
    /// refused or kept whole, and the Job left at its gate.
    pub(crate) async fn kept_as_left(
        &self,
        job_id: &JobId,
        body: &ipc::ApproveDispatch,
        said: &'static str,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let served = self.served_by(&job)?;
        let workflows = served.workflows();
        let held = body
            .workflow_id
            .as_ref()
            .and_then(|named| workflows.get(&named.to_domain()))
            .map(|held| held.frozen().clone());
        let decided = decided(&job, body, held.as_ref()).map_err(|why| refused(job_id, why))?;
        if let Some(tiers) = &decided.tiers {
            for (_, model) in tiers.named() {
                self.offered(model.as_str())
                    .map_err(|why| why.about(job_id))?;
            }
        }
        crate::tuned::harnesses_held(
            body.tuning.as_deref().unwrap_or_default(),
            &self.models().harnesses,
        )
        .map_err(|why| refused(job_id, Refused::Untuned(why)))?;
        for model in body
            .tuning
            .iter()
            .flatten()
            .filter_map(|step| step.model.as_deref())
        {
            self.offered(model.trim())
                .map_err(|why| why.about(job_id))?;
        }
        let start_point = body
            .landing
            .as_ref()
            .and_then(|landing| landing.start_point.as_deref())
            .map(str::trim)
            .filter(|start| !start.is_empty());
        let cut = self.branches_held(&served, &decided.landing, start_point, job_id)?;
        if let Some((named, from)) = cut {
            self.vcs()
                .create_branch(served.root(), named, from)
                .map_err(|why| Adrift::BranchNotCut {
                    job: job_id.clone(),
                    named: named.to_string(),
                    from: from.to_string(),
                    why: why.to_string(),
                })?;
        }
        let job = self.proposal_kept(&job, decided.edit.clone()).await?;
        {
            let mut store = self.store().lock().await;
            if let Some(tiers) = &decided.tiers {
                store
                    .set_tier_models(job_id, tiers)
                    .map_err(Adrift::Writing)?;
            }
            store
                .set_drone_cap(job_id, decided.drone_cap)
                .map_err(Adrift::Writing)?;
            store
                .set_landing(job_id, &decided.landing)
                .map_err(Adrift::Writing)?;
            store
                .set_policy_overrides(job_id, &decided.overrides)
                .map_err(Adrift::Writing)?;
        }
        self.noted_as_left(&job, &decided, said);
        Ok(job)
    }

    /// Save a proposal's words without releasing it (#1699's route).
    pub async fn edit_proposal(&self, job_id: &JobId, body: &ipc::EditJob) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let edit = edited(&job, body).map_err(|why| refused(job_id, why))?;
        self.proposal_kept(&job, edit).await
    }

    /// Rewrite the record, and move the Job's log where a new title moves its
    /// handle — the rename `crate::dispatched` makes when the proposer's title
    /// lands. Nothing else is on disk before the gate.
    async fn proposal_kept(&self, job: &Job, edit: ProposalEdit) -> Result<Job, Adrift> {
        let edited = job
            .proposal_edited(edit, &self.now())
            .map_err(|frozen| refused(job.id(), Refused::Frozen(frozen.status)))?;
        self.store()
            .lock()
            .await
            .record_proposal_edit(&edited)
            .map_err(|why| match why {
                store::WriteError::NotAtApproval { .. } => {
                    refused(job.id(), Refused::Frozen(JobStatus::Queued))
                }
                why => Adrift::Writing(why),
            })?;
        let (was, is) = (job.handle(), edited.handle());
        if was != is {
            if let Ok(served) = self.served_by(job) {
                let root = served.records_root();
                let _ = std::fs::rename(
                    crate::transcript::log_of(root, &was),
                    crate::transcript::log_of(root, &is),
                );
            }
        }
        self.learn_the_name(&edited);
        Ok(edited)
    }

    /// Refused where either branch is one the repository does not hold: a
    /// worktree cannot be cut from a branch that is not there, and a pull
    /// request cannot open against one. **A `from_ref` it lacks is answered
    /// with the branch to make**, where `start_point` names one it holds, and
    /// `target` may name that branch too. Since 23.21.
    fn branches_held<'a>(
        &self,
        served: &crate::repositories::Served,
        landing: &'a Landing,
        start_point: Option<&'a str>,
        job_id: &JobId,
    ) -> Result<Option<(&'a str, &'a str)>, Adrift> {
        let named: Vec<&str> = [&landing.target, &landing.from_ref]
            .into_iter()
            .flatten()
            .map(|branch| branch.as_str())
            .collect();
        if named.is_empty() {
            return Ok(None);
        }
        let held = self
            .vcs()
            .branches(served.root(), served.manifest().base())
            .map_err(|why| Adrift::BranchesUnread {
                job: Some(job_id.clone()),
                why: why.to_string(),
            })?;
        let holds = |name: &str| held.iter().any(|branch| branch.name == name);
        let refused = |name: &str| Adrift::NoSuchBranch {
            job: job_id.clone(),
            named: name.to_string(),
        };
        let from_ref = landing.from_ref.as_ref().map(|branch| branch.as_str());
        let cut = match (from_ref, start_point) {
            (Some(from), Some(start)) if !holds(from) => match holds(start) {
                true => Some((from, start)),
                false => return Err(refused(start)),
            },
            _ => None,
        };
        let made = cut.map(|(from, _)| from);
        match named
            .into_iter()
            .find(|name| !holds(name) && made != Some(*name))
        {
            Some(name) => Err(refused(name)),
            None => Ok(cut),
        }
    }

    /// How this Job lands, as approved. **A Job with no row lands as every Job
    /// did before 23.8**: cut from the base, landing in it, ready.
    pub(crate) async fn landing_of(&self, job_id: &JobId) -> Landing {
        self.store()
            .lock()
            .await
            .landing(job_id)
            .ok()
            .flatten()
            .unwrap_or_else(Landing::as_ever)
    }

    /// The branch this Job's work lands in: the one a person chose, or the
    /// Manifest's base, which the adapter infers where it names none.
    pub(crate) async fn target_of(
        &self,
        served: &crate::repositories::Served,
        job_id: &JobId,
    ) -> Option<String> {
        match self.landing_of(job_id).await.target {
            Some(target) => Some(target.as_str().to_string()),
            None => served.manifest().base().map(str::to_string),
        }
    }

    /// Learn which branch each Job in flight was cut from, at boot, so a
    /// worktree measured after a restart is measured from where it started.
    pub(crate) async fn cut_from_learned(&self, jobs: &[Job]) {
        for job in jobs.iter().filter(|job| !job.status().is_terminal()) {
            let Some(branch) = job.branch() else {
                continue;
            };
            if let Some(from) = self.landing_of(job.id()).await.from_ref {
                self.cut_from().learn(branch.as_str(), from.as_str());
            }
        }
    }

    /// What `get_job` serves of this slice: the four settings kept beside the
    /// Job, when it was approved, and its criteria with the issue they came
    /// from. **A store that will not say leaves each absent**, which reads as
    /// the Job doing what every Job did before 23.8.
    pub(crate) async fn approval_served(&self, job: &Job, detail: &mut ipc::JobDetail) {
        let events = {
            let store = self.store().lock().await;
            detail.landing = store
                .landing(job.id())
                .ok()
                .flatten()
                .map(|landing| ipc::LandingRule::from(&landing));
            detail.drone_cap = store.drone_cap(job.id()).ok().flatten();
            detail.policy_overrides = store
                .policy_overrides(job.id())
                .ok()
                .filter(|overrides| !overrides.is_none())
                .map(|overrides| ipc::PolicyOverrides::from(&overrides));
            let issue = store.issue_source(job.id()).ok().flatten();
            detail.acceptance_criteria = job
                .acceptance_criteria()
                .iter()
                .map(|criterion| ipc::Criterion::of(criterion, issue.as_ref()))
                .collect();
            store.events_for(job.id()).unwrap_or_default()
        };
        // The press: a person's move off the approval gate to `queued`. The
        // last one, since a Job sent back to the gate is approved again.
        detail.approved_at = events
            .iter()
            .rev()
            .find(|event| {
                event.under() == JobStatus::AwaitingApproval
                    && matches!(
                        event.moved(),
                        store::Moved::Job {
                            to: JobStatus::Queued,
                            ..
                        }
                    )
            })
            .map(|event| ipc::Instant::from(event.at()));
    }

    fn noted_as_left(&self, job: &Job, decided: &Decided, said: &'static str) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field(
            "workflow",
            FieldValue::Str(job.workflow_id().as_str().to_string()),
        )
        .with_field(
            "criteria",
            FieldValue::Str(job.acceptance_criteria().len().to_string()),
        );
        if let Some(cap) = decided.drone_cap {
            envelope = envelope.with_field("drone_cap", FieldValue::Str(cap.to_string()));
        }
        if let Some(target) = &decided.landing.target {
            envelope = envelope.with_field("target", FieldValue::Str(target.as_str().to_string()));
        }
        if let Some(from) = &decided.landing.from_ref {
            envelope = envelope.with_field("from_ref", FieldValue::Str(from.as_str().to_string()));
        }
        if let Some(said) = decided.overrides.auto_merge {
            envelope = envelope.with_field(
                "auto_merge_overridden",
                FieldValue::Str(said.as_written().to_string()),
            );
        }
        if let Some(said) = decided.overrides.review_gate {
            envelope = envelope.with_field(
                "review_gate_overridden",
                FieldValue::Str(said.as_written().to_string()),
            );
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}

/// The Job's log line at the press.
const APPROVED: &str = "a person approved the proposal as they left it";
/// The Job's log line where the dispatch form's settings were laid on it.
const SET_AT_DISPATCH: &str = "the settings a person chose at dispatch were laid on the proposal";

fn refused(job: &JobId, why: Refused) -> Adrift {
    Adrift::ProposalRefused {
        job: job.clone(),
        why,
    }
}

/// A proposal's `continue_from`, kept as where its worktree is cut from. The
/// approval shows it as the landing's `from_ref` and is where a branch the
/// repository does not hold is refused, so nothing is checked here. Blank is
/// none, and the landing is otherwise every Job's before slice 4.
pub(crate) fn continuing_from(
    store: &mut store::Store,
    job: &JobId,
    from: Option<&str>,
) -> Result<(), Adrift> {
    let Some(from_ref) = core_model::branch_named(from) else {
        return Ok(());
    };
    let landing = Landing {
        from_ref: Some(from_ref),
        ..Landing::as_ever()
    };
    store.set_landing(job, &landing).map_err(Adrift::Writing)
}
