//! What a person changes on a proposal, read against the Job, and the Job it
//! leaves at the press. Spike 022, slice 4: #1641's body, #1699's `edit_job`.
//!
//! **Everything a person sets at the gate is what the Job runs.** The words,
//! the workflow, each step's gate and the criteria are rewritten on the record
//! (`core_model::Job::proposal_edited`), so the gate, the brief and the Judge
//! read them where they always read; the tier map, the Drone cap, the landing
//! and the policy overrides are kept beside it and read where each is used.
//!
//! **Pure, then applied.** [`decided`] and [`edited`] read a body against a
//! Job and refuse or answer without touching anything, so the whole proposal
//! is accepted or none of it is, and the acceptance test reads the same
//! function `approve_dispatch` calls. What needs Fleet — a model `list_models`
//! offers, a branch the repository holds — is asked before anything is kept.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    AcceptanceCriterion, AdvanceGate, AutoMerge, Component, CriterionId, CriterionOrigin,
    CriterionSource, Envelope, Facts, FieldValue, FrozenWorkflow, Job, JobId, JobStatus, Landing,
    Level, ModelName, PolicyOverrides, PrMode, ProposalEdit, ReviewGate, StepSeed, TierModels,
    Title,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Why a proposal was refused, **and nothing was kept**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The Job is past its approval gate, so what it runs has frozen.
    Frozen(JobStatus),
    /// `edit_job` sent nothing to change.
    NothingToEdit,
    BlankTitle,
    /// The workflow named is not one this Job's repository holds.
    NoSuchWorkflow {
        named: String,
    },
    /// A gate names a step the workflow does not have.
    NoSuchStep {
        step: String,
    },
    /// A criterion's id is not one the Job holds.
    NoSuchCriterion {
        id: String,
    },
    /// One criterion's id twice.
    Repeated {
        id: String,
    },
    /// A held criterion moved above another, or below a new line: held lines
    /// keep their order and a new one goes at the foot.
    OutOfPlace {
        id: String,
    },
    BlankCriterion,
    /// A held criterion sent back answered another way. **How a line is
    /// answered is the workflow's**, and a Check frozen with it does not move
    /// because the words did.
    AnsweredAnotherWay {
        id: String,
        held: CriterionSource,
    },
    /// A line typed at the gate sent as a Check's. Nothing mechanical exists to
    /// run against words typed here (#1641): it is the Judge's, or a person's.
    CheckOnATypedLine,
    /// `judge` on a step that gives the Judge nothing to read, with nobody
    /// stopping it either: it would advance on the mechanical tier while
    /// reading as judged — the parse-time rule `config` holds a file to.
    JudgeReadsNothing {
        step: String,
    },
    /// `overridden` on a step that defers to no repository rule.
    NothingToOverride {
        step: String,
    },
    /// `review_gate` decides between a person and the Judge, and the override
    /// named neither.
    ReviewGateNeedsAnAnswer {
        step: String,
    },
    /// Two steps override one policy two ways.
    OverridesDisagree {
        policy: &'static str,
    },
    /// A setting Fleet does not run, refused rather than kept unread
    /// (spike 022, `landing.ts`).
    NotHonoured {
        setting: &'static str,
        value: &'static str,
    },
    /// A Drone cap of zero: a Job that may run no Drone is not a Job.
    NoCap,
    BlankModel,
}

impl fmt::Display for Refused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refused::Frozen(status) => write!(
                out,
                "the job is {}, not awaiting_approval, so what it runs has frozen",
                status.as_wire()
            ),
            Refused::NothingToEdit => write!(out, "the edit changes nothing"),
            Refused::BlankTitle => write!(out, "a job needs a title somebody can pick out"),
            Refused::NoSuchWorkflow { named } => {
                write!(
                    out,
                    "`{named}` is not a workflow this job's repository holds"
                )
            }
            Refused::NoSuchStep { step } => {
                write!(out, "the workflow has no step `{step}` to gate")
            }
            Refused::NoSuchCriterion { id } => {
                write!(out, "the job holds no criterion `{id}`")
            }
            Refused::Repeated { id } => write!(out, "criterion `{id}` is sent twice"),
            Refused::OutOfPlace { id } => write!(
                out,
                "criterion `{id}` is out of its place: the criteria keep their order, and a new \
                 one goes at the foot"
            ),
            Refused::BlankCriterion => write!(out, "a criterion says nothing"),
            Refused::AnsweredAnotherWay { id, held } => write!(
                out,
                "criterion `{id}` is answered by {} and stays so: how a line is answered is the \
                 workflow's",
                held.as_wire()
            ),
            Refused::CheckOnATypedLine => write!(
                out,
                "a line typed at the gate has no Check to run against it; send it as the Judge's \
                 or as yours"
            ),
            Refused::JudgeReadsNothing { step } => write!(
                out,
                "step `{step}` gives the Judge nothing to read, so asking it alone would advance \
                 on the Checks while reading as judged"
            ),
            Refused::NothingToOverride { step } => {
                write!(
                    out,
                    "step `{step}` defers to no repository rule to override"
                )
            }
            Refused::ReviewGateNeedsAnAnswer { step } => write!(
                out,
                "step `{step}` defers to review_gate, which decides between you and the Judge; \
                 override it to one of them"
            ),
            Refused::OverridesDisagree { policy } => {
                write!(out, "two steps override {policy} two different ways")
            }
            Refused::NotHonoured { setting, value } => write!(
                out,
                "{setting} `{value}` is not something Fleet runs yet, so it is not kept"
            ),
            Refused::NoCap => write!(out, "a drone cap of zero lets the job run nothing"),
            Refused::BlankModel => write!(out, "a tier names a blank model"),
        }
    }
}

/// What an approval decides: the record's new values, and the settings kept
/// beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decided {
    pub edit: ProposalEdit,
    /// `None` is the map as it stands.
    pub tiers: Option<TierModels>,
    /// `None` is the machine's cap holding. **Kept, not enforced**: slice 5.
    pub drone_cap: Option<u32>,
    pub landing: Landing,
    pub overrides: PolicyOverrides,
}

/// Approve's body, read against the Job. `held` is the workflow the body
/// names where it is not the Job's own, as the Job's repository holds it.
pub fn decided(
    job: &Job,
    body: &ipc::ApproveDispatch,
    held: Option<&FrozenWorkflow>,
) -> Result<Decided, Refused> {
    at_the_gate(job)?;
    let (title, facts, acceptance_criteria) = words(
        job,
        body.title.as_deref(),
        body.facts.as_deref(),
        body.criteria.as_deref(),
    )?;
    let base = match &body.workflow_id {
        Some(named) if named.as_str() != job.workflow_id().as_str() => held
            .filter(|held| held.id().as_str() == named.as_str())
            .cloned()
            .ok_or_else(|| Refused::NoSuchWorkflow {
                named: named.as_str().to_string(),
            })?,
        _ => job.workflow().clone(),
    };
    let (workflow, overrides) = gated(base, body.gates.as_deref().unwrap_or_default())?;
    let tiers = match &body.tiers {
        Some(map) => Some(tiers_named(map)?),
        None => None,
    };
    if body.drone_cap == Some(0) {
        return Err(Refused::NoCap);
    }
    Ok(Decided {
        edit: ProposalEdit {
            title,
            facts,
            steps: steps_of(&workflow),
            workflow,
            acceptance_criteria,
        },
        tiers,
        drone_cap: body.drone_cap,
        landing: landing_of(body.landing.as_ref())?,
        overrides,
    })
}

/// `edit_job`'s body, read against the Job: its words, saved without
/// releasing it (#1699). The workflow and its gates are the Job's as they are.
pub fn edited(job: &Job, body: &ipc::EditJob) -> Result<ProposalEdit, Refused> {
    at_the_gate(job)?;
    if body.title.is_none() && body.facts.is_none() && body.criteria.is_none() {
        return Err(Refused::NothingToEdit);
    }
    let (title, facts, acceptance_criteria) = words(
        job,
        body.title.as_deref(),
        body.facts.as_deref(),
        body.criteria.as_deref(),
    )?;
    let workflow = job.workflow().clone();
    Ok(ProposalEdit {
        title,
        facts,
        steps: steps_of(&workflow),
        workflow,
        acceptance_criteria,
    })
}

/// The tier map a person named. Whether each model is one this machine
/// offers is Fleet's to ask, before anything is kept.
pub fn tiers_named(map: &ipc::TierModels) -> Result<TierModels, Refused> {
    let mut tiers = TierModels::default();
    for (tier, named) in map.named() {
        tiers = tiers.with(
            tier,
            ModelName::new(named).map_err(|_| Refused::BlankModel)?,
        );
    }
    Ok(tiers)
}

fn at_the_gate(job: &Job) -> Result<(), Refused> {
    match job.status() {
        JobStatus::AwaitingApproval => Ok(()),
        status => Err(Refused::Frozen(status)),
    }
}

/// The title, the facts and the criteria, each the body's where it sent one.
fn words(
    job: &Job,
    title: Option<&str>,
    facts: Option<&str>,
    criteria: Option<&[ipc::CriterionWritten]>,
) -> Result<(Title, Facts, Vec<AcceptanceCriterion>), Refused> {
    let title = match title {
        Some(title) => Title::new(title).map_err(|_| Refused::BlankTitle)?,
        None => job.title().clone(),
    };
    let facts = match facts {
        Some(facts) => Facts::new(facts.trim()),
        None => job.facts().clone(),
    };
    let criteria = match criteria {
        Some(sent) => written(job.acceptance_criteria(), sent)?,
        None => job.acceptance_criteria().to_vec(),
    };
    Ok((title, facts, criteria))
}

/// The criteria as a person left them.
///
/// **A line with an id is the line it was**: its origin stays — reworded, the
/// words still answer to the issue they came from, and the Job says so when
/// it moves — and how it is answered stays. **A line without one is new**: the
/// person's, answered by the Judge or by a person, and given the next `c<n>`
/// no line has had. **Held lines keep their order and a new one goes at the
/// foot** (`docs/journeys/dispatch-a-job.md`, 22 Sep 2026): a line above the
/// others renumbers every place a citation names.
fn written(
    held: &[AcceptanceCriterion],
    sent: &[ipc::CriterionWritten],
) -> Result<Vec<AcceptanceCriterion>, Refused> {
    let by_id: BTreeMap<&str, (usize, &AcceptanceCriterion)> = held
        .iter()
        .enumerate()
        .map(|(at, criterion)| (criterion.criterion_id.as_str(), (at, criterion)))
        .collect();
    let mut last_held: Option<usize> = None;
    let mut added = false;
    let mut next = core_model::next_criterion_number(
        held.iter().map(|c| c.criterion_id.as_str()).chain(
            sent.iter()
                .filter_map(|c| c.criterion_id.as_ref().map(|id| id.as_str())),
        ),
    );
    let mut seen = BTreeSet::new();
    let mut criteria = Vec::with_capacity(sent.len());
    for line in sent {
        let text = line.text.trim();
        if text.is_empty() {
            return Err(Refused::BlankCriterion);
        }
        let source = line.source.domain();
        let criterion = match &line.criterion_id {
            Some(id) => {
                let id = id.as_str();
                if !seen.insert(id.to_string()) {
                    return Err(Refused::Repeated { id: id.to_string() });
                }
                let &(at, held) = by_id
                    .get(id)
                    .ok_or_else(|| Refused::NoSuchCriterion { id: id.to_string() })?;
                if added || last_held.is_some_and(|before| before > at) {
                    return Err(Refused::OutOfPlace { id: id.to_string() });
                }
                last_held = Some(at);
                if held.source != source {
                    return Err(Refused::AnsweredAnotherWay {
                        id: id.to_string(),
                        held: held.source,
                    });
                }
                AcceptanceCriterion {
                    text: text.to_string(),
                    ..held.clone()
                }
            }
            None => {
                if source == CriterionSource::Check {
                    return Err(Refused::CheckOnATypedLine);
                }
                added = true;
                let id = core_model::criterion_numbered(next);
                next += 1;
                AcceptanceCriterion {
                    criterion_id: CriterionId::new(id),
                    text: text.to_string(),
                    source,
                    origin: CriterionOrigin::Person,
                }
            }
        };
        criteria.push(criterion);
    }
    Ok(criteria)
}

/// Each step as its gate was set, and what the Job overrode.
///
/// | The step declares | Not overridden | Overridden |
/// |---|---|---|
/// | Its own gate | `you` stops for a person, else `judge` asks the Judge, else it runs on | Refused: nothing to override |
/// | `manifest_rule:auto_merge` | The repository decides | `you` is `never`, else `checks-pass` |
/// | `manifest_rule:review_gate` | The repository decides | `you` is `human_always`, else `judge` is `auto_if_judge_passes` |
///
/// **An overridden step keeps deferring on the record**, so the Record still
/// says whose rule it was and what the gate read (#1683); the override wins
/// at every gate, however the rule moves (answer 4). `checks` and `judge` take
/// declarations away on every step alike.
fn gated(
    workflow: FrozenWorkflow,
    gates: &[ipc::GateChoice],
) -> Result<(FrozenWorkflow, PolicyOverrides), Refused> {
    let mut chosen: BTreeMap<&str, &ipc::GateChoice> = BTreeMap::new();
    for gate in gates {
        if workflow.step(&gate.step_id.to_domain()).is_none() {
            return Err(Refused::NoSuchStep {
                step: gate.step_id.as_str().to_string(),
            });
        }
        chosen.insert(gate.step_id.as_str(), gate);
    }
    let mut overrides = PolicyOverrides::none();
    let mut gates_by_step = BTreeMap::new();
    for step in workflow.steps() {
        let Some(gate) = chosen.get(step.id().as_str()) else {
            continue;
        };
        let name = || step.id().as_str().to_string();
        let declared = step.advance_gate();
        let kept = match (declared, gate.overridden) {
            (AdvanceGate::ManifestRuleAutoMerge, true) => {
                let said = if gate.you {
                    AutoMerge::Never
                } else {
                    AutoMerge::ChecksPass
                };
                agree(&mut overrides.auto_merge, said, "auto_merge")?;
                declared
            }
            (AdvanceGate::ManifestRuleReviewGate, true) => {
                let said = match (gate.you, gate.judge) {
                    (true, _) => ReviewGate::HumanAlways,
                    (false, true) => ReviewGate::AutoIfJudgePasses,
                    (false, false) => {
                        return Err(Refused::ReviewGateNeedsAnAnswer { step: name() })
                    }
                };
                agree(&mut overrides.review_gate, said, "review_gate")?;
                declared
            }
            (_, true) => return Err(Refused::NothingToOverride { step: name() }),
            (AdvanceGate::ManifestRuleAutoMerge | AdvanceGate::ManifestRuleReviewGate, false) => {
                declared
            }
            (_, false) if gate.you => AdvanceGate::HumanAlways,
            (_, false) if gate.judge => {
                if !step.asks_the_judge() {
                    return Err(Refused::JudgeReadsNothing { step: name() });
                }
                AdvanceGate::AutoIfJudgePasses
            }
            (_, false) => AdvanceGate::Auto,
        };
        gates_by_step.insert(step.id().clone(), (kept, gate.checks, gate.judge));
    }
    let workflow = workflow.regated(|step| match gates_by_step.get(step.id()) {
        Some(&(gate, checks, judge)) => step.gated_by_person(gate, checks, judge),
        None => step,
    });
    Ok((workflow, overrides))
}

/// One override per policy: a second step saying the same is no news, and one
/// saying otherwise is refused.
fn agree<P: PartialEq + Copy>(
    kept: &mut Option<P>,
    said: P,
    policy: &'static str,
) -> Result<(), Refused> {
    match kept {
        Some(already) if *already != said => Err(Refused::OverridesDisagree { policy }),
        _ => {
            *kept = Some(said);
            Ok(())
        }
    }
}

/// The landing a person set. **Only what Fleet runs is kept**: one branch
/// per Job, done when delivered (`landing.ts`'s `COMPLETE_WHEN_SERVED`).
fn landing_of(choice: Option<&ipc::LandingChoice>) -> Result<Landing, Refused> {
    let Some(choice) = choice else {
        return Ok(Landing::as_ever());
    };
    if choice.branching == ipc::LandingUnit::Group {
        return Err(Refused::NotHonoured {
            setting: "branching",
            value: "group",
        });
    }
    let refused = |value| {
        Err(Refused::NotHonoured {
            setting: "complete_when",
            value,
        })
    };
    match choice.complete_when {
        ipc::CompleteWhen::Delivered => {}
        ipc::CompleteWhen::PrMerged => return refused("pr_merged"),
        ipc::CompleteWhen::PrOpened => return refused("pr_opened"),
        // A parent finishing when its members have landed is slice 6's.
        ipc::CompleteWhen::AllMembersLanded => return refused("all_members_landed"),
    }
    Ok(Landing {
        target: core_model::branch_named(choice.target.as_deref()),
        from_ref: core_model::branch_named(choice.from_ref.as_deref()),
        pr_mode: choice
            .pr_mode
            .map(|mode| mode.domain())
            .unwrap_or(PrMode::Ready),
    })
}

/// One step row per step of `workflow`, in order.
fn steps_of(workflow: &FrozenWorkflow) -> Vec<StepSeed> {
    workflow
        .steps()
        .iter()
        .enumerate()
        .map(|(ordinal, step)| StepSeed {
            step_id: step.id().clone(),
            ordinal: ordinal as u32,
        })
        .collect()
}

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
            let held = settings
                .workflow_id
                .as_ref()
                .and_then(|named| served.workflows().get(&named.to_domain()))
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
    async fn kept_as_left(
        &self,
        job_id: &JobId,
        body: &ipc::ApproveDispatch,
        said: &'static str,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let served = self.served_by(&job)?;
        let held = body
            .workflow_id
            .as_ref()
            .and_then(|named| served.workflows().get(&named.to_domain()))
            .map(|held| held.frozen().clone());
        let decided = decided(&job, body, held.as_ref()).map_err(|why| refused(job_id, why))?;
        if let Some(tiers) = &decided.tiers {
            for (_, model) in tiers.named() {
                self.offered(model.as_str())
                    .map_err(|why| why.about(job_id))?;
            }
        }
        self.branches_held(&served, &decided.landing, job_id)?;
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
    /// request cannot open against one.
    fn branches_held(
        &self,
        served: &crate::repositories::Served,
        landing: &Landing,
        job_id: &JobId,
    ) -> Result<(), Adrift> {
        let named: Vec<&str> = [&landing.target, &landing.from_ref]
            .into_iter()
            .flatten()
            .map(|branch| branch.as_str())
            .collect();
        if named.is_empty() {
            return Ok(());
        }
        let held = self
            .vcs()
            .branches(served.root(), served.manifest().base())
            .map_err(|why| Adrift::BranchesUnread {
                job: Some(job_id.clone()),
                why: why.to_string(),
            })?;
        for name in named {
            if !held.iter().any(|branch| branch.name == name) {
                return Err(Adrift::NoSuchBranch {
                    job: job_id.clone(),
                    named: name.to_string(),
                });
            }
        }
        Ok(())
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
