//! A Session's place in its workflow, read from what Fleet observed. `docs/concepts/session.md`,
//! *The rail*.
//!
//! **Pure.** The steps and the facts are passed in, so each rule is read and tested without a Fleet.
//! `rail_keeping` takes the facts out of the store and `rail_watching` acts on what changed; this is
//! the rules between the two.
//!
//! **The agent's word is never an input.** The step comes from the plan (approved, tasks open), the
//! build record (first edit, a push, a pull request, a merge) and the Session's own Check runs and
//! commits. It is soft: nothing here blocks, and a skipped step draws one nudge.
//!
//! **Three anchors are exact, the steps between them are read loosely.** A `records_plan` step is
//! current until a plan is approved. A `follows_plan` step is current from approval and stays until a
//! later step is entered. A `delivers` step is current from the first push until its pull request
//! merges. A step between anchors is entered on the fact that fits it (a diff step on the first edit, a
//! step of manifest Checks on a Check run, a delivery step on a commit, a review step on a pull request
//! opened), only once every anchor before it has been passed, and **where it cannot be told apart from
//! the step just before it the earlier one is kept**. A step that fits nothing observable is never
//! entered by itself.

use config::ResolvedWorkflow;
use core_model::{EvidenceType, JudgeCriterion, ResolvedCheck, ResolvedStep, StepPhase};
use ipc::{BuildPhase, BuildStopId, PlanTaskState, SessionRecord};

/// The observable fact a step between anchors fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    /// A first edit.
    Edited,
    /// A Check run the Session asked for, or a pull request's checks.
    Checked,
    /// A commit.
    Committed,
    /// A pull request opened.
    PrOpened,
}

/// What the rail needs to know of one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RailStep {
    pub label: String,
    pub about: Option<String>,
    /// What the agent is told on entering the step.
    pub instructions: Option<String>,
    pub records_plan: bool,
    pub follows_plan: bool,
    pub delivers: bool,
    /// Entering the step asks how the work is delivered.
    pub asks_delivery: bool,
    /// What a boundary judge asks after the step. Empty is a step nobody judges.
    pub criteria: Vec<JudgeCriterion>,
    pub fit: Option<Fit>,
}

impl RailStep {
    pub(crate) fn judges(&self) -> bool {
        !self.criteria.is_empty()
    }
}

fn fit_of(step: &ResolvedStep) -> Option<Fit> {
    if step.records_plan() || step.follows_plan() || step.delivers() {
        return None;
    }
    let manifest_check = step.checks().iter().any(|check| matches!(check, ResolvedCheck::ManifestCheck { .. }));
    match step.evidence_type() {
        Some(EvidenceType::Diff) => Some(Fit::Edited),
        Some(EvidenceType::TestSuiteRun | EvidenceType::FailingTest) => Some(Fit::Checked),
        _ if manifest_check => Some(Fit::Checked),
        _ if step.phase_declared() == Some(StepPhase::Delivery) => Some(Fit::Committed),
        Some(EvidenceType::Review) => Some(Fit::PrOpened),
        _ => None,
    }
}

/// The workflow's steps as the rail reads them.
pub(crate) fn steps_of(workflow: &ResolvedWorkflow) -> Vec<RailStep> {
    workflow
        .steps()
        .iter()
        .map(|step| RailStep {
            label: step.label().to_string(),
            about: step.about().map(str::to_string),
            instructions: step.instructions().map(str::to_string),
            records_plan: step.records_plan(),
            follows_plan: step.follows_plan(),
            delivers: step.delivers(),
            asks_delivery: step.asks_delivery(),
            criteria: step.judge_checks().iter().flat_map(|check| check.criteria().to_vec()).collect(),
            fit: fit_of(step),
        })
        .collect()
}

/// Facts the record does not carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extra {
    pub committed: bool,
    pub checked: bool,
}

/// What was observed, as the rules read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    pub plan_approved: bool,
    pub tasks_open: bool,
    pub edited: bool,
    pub committed: bool,
    pub checked: bool,
    pub pushed: bool,
    pub pr_opened: bool,
    pub landed: bool,
}

impl Facts {
    pub(crate) fn of(record: &SessionRecord, extra: Extra) -> Facts {
        let reached = |id: BuildStopId| {
            record.build.as_ref().is_some_and(|build| build.stops.iter().any(|stop| stop.id == id && stop.at.is_some()))
        };
        Facts {
            plan_approved: record.plan.as_ref().is_some_and(|plan| plan.approved),
            tasks_open: record.plan.as_ref().is_some_and(|plan| {
                plan.groups
                    .iter()
                    .flat_map(|group| &group.tasks)
                    .any(|task| matches!(task.state, PlanTaskState::Open | PlanTaskState::Working))
            }),
            edited: record
                .build
                .as_ref()
                .is_some_and(|build| matches!(build.phase, BuildPhase::Implement | BuildPhase::Road | BuildPhase::Landed)),
            committed: extra.committed,
            checked: extra.checked || reached(BuildStopId::Checks),
            pushed: reached(BuildStopId::Pushed),
            pr_opened: reached(BuildStopId::Pr),
            landed: record.build.as_ref().is_some_and(|build| build.phase == BuildPhase::Landed || build.landed_at.is_some()),
        }
    }

    fn has(&self, fit: Fit) -> bool {
        match fit {
            Fit::Edited => self.edited,
            Fit::Checked => self.checked,
            Fit::Committed => self.committed,
            Fit::PrOpened => self.pr_opened,
        }
    }
}

/// Whether the Session is past an anchor. A step that is no anchor is not in the way.
fn passed(step: &RailStep, facts: &Facts) -> bool {
    if step.records_plan {
        return facts.plan_approved;
    }
    if step.follows_plan {
        return facts.plan_approved && !facts.tasks_open;
    }
    if step.delivers {
        return facts.landed;
    }
    true
}

fn entered(steps: &[RailStep], at: usize, facts: &Facts) -> bool {
    let step = &steps[at];
    if step.records_plan {
        return !facts.plan_approved;
    }
    if step.follows_plan {
        return facts.plan_approved;
    }
    if step.delivers {
        return facts.pushed;
    }
    let Some(fit) = step.fit else { return false };
    facts.has(fit)
        && steps[..at].iter().all(|earlier| passed(earlier, facts))
        && !(at > 0 && steps[at - 1].fit == Some(fit))
}

/// The step the Session is on: the last one entered, else the first. **Absent** once the delivering
/// step's pull request merged and nothing follows it, and for a workflow of no steps.
pub(crate) fn current(steps: &[RailStep], facts: &Facts) -> Option<usize> {
    if steps.is_empty() {
        return None;
    }
    if let Some(delivering) = steps.iter().position(|step| step.delivers).filter(|_| facts.landed) {
        return (delivering + 1 < steps.len()).then_some(delivering + 1);
    }
    Some((0..steps.len()).rev().find(|at| entered(steps, *at, facts)).unwrap_or(0))
}

/// Which step was skipped, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Skip {
    /// Files were edited before the plan was approved.
    Plan,
    /// The work was pushed while the plan's tasks were open.
    Tasks,
}

/// The steps the observed facts show skipped. A step is named however often it is skipped; saying so
/// only once is the caller's.
pub(crate) fn skipped(steps: &[RailStep], facts: &Facts) -> Vec<(usize, Skip)> {
    let mut found = Vec::new();
    if let Some(at) = steps.iter().position(|step| step.records_plan).filter(|_| !facts.plan_approved && facts.edited) {
        found.push((at, Skip::Plan));
    }
    if let Some(at) = steps.iter().position(|step| step.follows_plan).filter(|_| facts.pushed && facts.tasks_open) {
        found.push((at, Skip::Tasks));
    }
    found
}

/// The most of a step's own words a nudge repeats.
const ABOUT_AT_MOST: usize = 200;

/// The one short message that names the step skipped and what it asks. Nothing is blocked.
pub(crate) fn nudge(workflow: &str, step: &RailStep, skip: Skip) -> String {
    let asks = match step.about.as_deref().map(str::trim).filter(|about| !about.is_empty()) {
        Some(about) => about.chars().take(ABOUT_AT_MOST).collect::<String>(),
        None => match skip {
            Skip::Plan => String::from("a plan recorded with record_plan, and approved by the person"),
            Skip::Tasks => String::from("the plan's tasks worked and finished before the work goes out"),
        },
    };
    let did = match skip {
        Skip::Plan => "edited files before",
        Skip::Tasks => "pushed while tasks were still open in",
    };
    format!(
        "Fleet: this Job follows the {workflow} workflow, and you {did} its \"{}\" step. That step asks: {asks}. \
         Nothing is blocked. Take it up now, or say why the work goes on without it.",
        step.label
    )
}

/// What the agent is told on entering a step: the step's own instructions, named by its step.
/// `None` where the step wrote none, and nothing is sent.
pub(crate) fn instructed(workflow: &str, step: &RailStep) -> Option<String> {
    let words = step.instructions.as_deref().map(str::trim).filter(|words| !words.is_empty())?;
    Some(format!("Fleet: this Job follows the {workflow} workflow, and you are on its \"{}\" step.\n\n{words}", step.label))
}

/// The step a boundary judge reads: the one the Session just left, if it asks the Judge anything and
/// has not been read. Moving back, or staying, leaves nothing behind.
pub(crate) fn left_behind(
    steps: &[RailStep],
    from: Option<usize>,
    to: Option<usize>,
    landed: bool,
    judged: &[u32],
) -> Option<usize> {
    let from = from?;
    let left = match to {
        Some(to) => to > from,
        None => landed,
    };
    (left && steps.get(from)?.judges() && !judged.contains(&(from as u32))).then_some(from)
}

/// How a step reads in the timeline's `says`: its label, `start` before the first and `done` past the
/// last.
pub(crate) fn named(steps: &[RailStep], at: Option<usize>, none: &str) -> String {
    at.and_then(|at| steps.get(at)).map_or_else(|| none.to_string(), |step| step.label.clone())
}
