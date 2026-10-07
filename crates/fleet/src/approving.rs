//! What a person changes on a proposal, read against the Job, and the Job it
//! leaves at the press. Spike 022, slice 4: #1641's body, #1699's `edit_job`.
//!
//! **Everything a person sets at the gate is what the Job runs.** The words,
//! the workflow, each step's gate and the criteria are rewritten on the record
//! (`core_model::Job::proposal_edited`), so the gate, the brief and the Judge
//! read them where they always read; the tier map, the Drone cap, the landing
//! and the policy overrides are kept beside it and read where each is used.
//!
//! **Pure.** [`decided`] and [`edited`] refuse or answer without touching
//! anything, so the whole proposal is kept or none of it is, and the
//! acceptance test reads what `approve_dispatch` calls. `crate::approved`
//! asks the machine and applies.
//!
//! **Over 500 by [`Refused`]**, one press's list; a step's tuning is `crate::tuned`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use core_model::{
    AcceptanceCriterion, AdvanceGate, AutoMerge, CriterionId, CriterionOrigin, CriterionSource,
    Facts, FrozenWorkflow, Job, JobStatus, Landing, ModelName, PolicyOverrides, PrMode,
    PrModeTiers, ProposalEdit, ReviewGate, StepSeed, TierModels, Title,
};

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
    /// A held criterion answered another way: how a line is answered is the
    /// workflow's, and does not move because the words did.
    AnsweredAnotherWay {
        id: String,
        held: CriterionSource,
    },
    /// A line typed at the gate sent as a Check's: nothing mechanical runs on
    /// words typed here (#1641).
    CheckOnATypedLine,
    /// `judge` alone on a step giving the Judge nothing to read: it would
    /// advance on the Checks while reading as judged, `config`'s parse rule.
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
    /// A setting Fleet does not run, refused rather than kept unread.
    NotHonoured {
        setting: &'static str,
        value: &'static str,
    },
    /// A Drone cap of zero.
    NoCap,
    BlankModel,
    /// A note for the proposer that says nothing.
    NoNote,
    /// The Job was dispatched by an Epic's plan step, whose plan owns it.
    WaveMember,
    /// A Job of the same split is not at its gate, so the split cannot go back
    /// to the proposer together, and none of it does.
    SiblingPastItsGate {
        sibling: String,
        title: String,
        status: JobStatus,
    },
    /// `local` with `auto_merge`: nothing opens a pull request to merge.
    NothingToAutoMerge,
    /// A step's tuning nothing could honour.
    Untuned(crate::tuned::Untunable),
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
            Refused::NoNote => write!(out, "a note for the proposer says nothing"),
            Refused::WaveMember => write!(
                out,
                "this job belongs to an Epic's wave, which the Epic's plan owns, so it is not \
                 sent back on its own"
            ),
            Refused::SiblingPastItsGate {
                sibling,
                title,
                status,
            } => write!(
                out,
                "`{title}` ({sibling}), which was proposed together with this one, is {} and not \
                 awaiting_approval, so the split cannot go back to the proposer together",
                status.as_wire()
            ),
            Refused::NothingToAutoMerge => write!(
                out,
                "local keeps the work on its branch and opens no pull request, so there is \
                 nothing to auto-merge"
            ),
            Refused::Untuned(why) => write!(out, "{why}"),
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
    decided_under(job, body, held, None, None)
}

/// The pull request mode a Job approved with nothing said opens as: the
/// workflow's delivering step, then the repository, then this machine, then
/// ready. **What approval serves Bridge to start on**, and what [`decided_under`]
/// freezes where the body names none, so the two cannot disagree.
pub fn pr_mode_default(
    workflow: &FrozenWorkflow,
    repository: Option<PrMode>,
    machine: Option<PrMode>,
) -> PrMode {
    PrModeTiers {
        job: None,
        step: workflow.delivering_step().and_then(|step| step.draft_pr()),
        repository,
        machine,
    }
    .resolve()
}

/// [`decided`], with the repository's and this machine's pull request mode
/// beneath whatever the workflow's delivering step and the body say. **The
/// most specific wins**: the body's own, the step's `draft_pr`, the
/// repository's `pr_mode`, the machine's, then ready.
pub fn decided_under(
    job: &Job,
    body: &ipc::ApproveDispatch,
    held: Option<&FrozenWorkflow>,
    repository: Option<PrMode>,
    machine: Option<PrMode>,
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
    let workflow = crate::tuned::tuned(workflow, body.tuning.as_deref().unwrap_or_default())
        .map_err(Refused::Untuned)?;
    let tiers = match &body.tiers {
        Some(map) => Some(tiers_named(map)?),
        None => None,
    };
    if body.drone_cap == Some(0) {
        return Err(Refused::NoCap);
    }
    let pr_mode = PrModeTiers {
        job: body
            .landing
            .as_ref()
            .and_then(|landing| landing.pr_mode)
            .map(|mode| mode.domain()),
        step: workflow.delivering_step().and_then(|step| step.draft_pr()),
        repository,
        machine,
    }
    .resolve();
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
        landing: landing_of(body.landing.as_ref(), pr_mode)?,
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

/// The landing a person set. **Only what Fleet runs is kept** (`landing.ts`'s
/// `COMPLETE_WHEN_SERVED`): one branch per Job, done delivered or landed.
/// `pr_mode` is the answer the tiers came to, which the choice's own already
/// went into.
pub(crate) fn landing_of(
    choice: Option<&ipc::LandingChoice>,
    pr_mode: PrMode,
) -> Result<Landing, Refused> {
    let Some(choice) = choice else {
        return Ok(Landing {
            pr_mode,
            ..Landing::as_ever()
        });
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
    let complete_when = match choice.complete_when {
        ipc::CompleteWhen::Delivered => core_model::CompleteWhen::Delivered,
        ipc::CompleteWhen::AllMembersLanded => core_model::CompleteWhen::AllMembersLanded,
        ipc::CompleteWhen::PrMerged => return refused("pr_merged"),
        ipc::CompleteWhen::PrOpened => return refused("pr_opened"),
    };
    if choice.local && choice.auto_merge {
        return Err(Refused::NothingToAutoMerge);
    }
    Ok(Landing {
        local: choice.local,
        auto_merge: choice.auto_merge,
        complete_when,
        target: core_model::branch_named(choice.target.as_deref()),
        from_ref: core_model::branch_named(choice.from_ref.as_deref()),
        // Ignored while `local` holds: one answer, not two.
        pr_mode: match choice.local {
            true => PrMode::Ready,
            false => pr_mode,
        },
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
