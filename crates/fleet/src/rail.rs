//! A Session's place in its workflow, read from what Fleet observed. `docs/concepts/session.md`,
//! *The rail*.
//!
//! **Pure.** The steps and the facts are passed in, so each rule is read and tested without a Fleet.
//! `rail_keeping` takes the facts out of the store and `rail_following` acts on what changed; this is
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
//!
//! **Steps before the plan, and the walk.** A step before the recording step is current until a plan
//! is recorded, or until any end its `ends_on` lists holds (`awaits_reply` is `replied`); the
//! recording step is then entered when a plan is recorded. A `walks` step is current from the plan's
//! approval until he approves a window the Job showed, at or after that approval, and is passed over
//! when no walk is expected and its own work is done: its tasks finished where it follows the plan, else
//! at once. The step after it is entered once it is passed, whatever fact would have entered it.
//!
//! **A step the person skipped is passed and never entered**, so the Job moves on past it. A skipped
//! recording step reads as a plan approved once the Job has edited, since no plan is coming.

use config::ResolvedWorkflow;
use core_model::{EvidenceType, JudgeCriterion, LifecycleEnd, ResolvedCheck, ResolvedStep, StepEnd, StepPhase};
use ipc::{BuildPhase, BuildStopId, PlanTaskState, SessionRecord, SessionStanding, TouchAct};

use crate::rail_ends;
use crate::rail_lifecycle::StepBits;

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
    pub id: String,
    pub label: String,
    pub about: Option<String>,
    /// What the agent is told on entering the step.
    pub instructions: Option<String>,
    pub records_plan: bool,
    pub follows_plan: bool,
    pub delivers: bool,
    /// Entering the step asks how the work is delivered.
    pub asks_delivery: bool,
    /// The step where the person is shown the work and approves it before tests are written.
    pub walks: bool,
    /// What the person does that ends the step, any one of them. Empty for a step that lists none.
    pub ends_on: Vec<StepEnd>,
    /// What a boundary judge asks after the step. Empty is a step nobody judges.
    pub criteria: Vec<JudgeCriterion>,
    pub fit: Option<Fit>,
}

impl RailStep {
    pub(crate) fn judges(&self) -> bool {
        !self.criteria.is_empty()
    }

    /// The lifecycle states that end the step. A step that lists one waits on the world, and is
    /// placed by what it holds rather than by a fact that fits it.
    pub(crate) fn lifecycle_ends(&self) -> impl Iterator<Item = &LifecycleEnd> {
        self.ends_on.iter().filter_map(StepEnd::lifecycle)
    }

    pub(crate) fn has_lifecycle_end(&self) -> bool {
        self.lifecycle_ends().next().is_some()
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
            id: step.id().as_str().to_string(),
            label: step.label().to_string(),
            about: step.about().map(str::to_string),
            instructions: step.instructions().map(str::to_string),
            records_plan: step.records_plan(),
            follows_plan: step.follows_plan(),
            delivers: step.delivers(),
            asks_delivery: step.asks_delivery(),
            walks: step.walks(),
            ends_on: step.ends_on().to_vec(),
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
    /// The person wrote after the agent's first turn.
    pub replied: bool,
    /// The Job showed a window.
    pub walk_shown: bool,
    /// The person approved a window the Job showed at or after the plan's approval: what passes a
    /// `walks` step.
    pub walk_approved: bool,
    /// The person approved a window the Job showed, whenever: what a `walk_approved` end reads.
    pub any_walk_approved: bool,
    /// The person approved a sketch the Job showed.
    pub sketch_approved: bool,
    /// The person approved the newest revision of a document the Job wrote, read off the kept page.
    pub document_approved: bool,
    /// The person approved a draft the Job proposed, as written or with his own text.
    pub draft_approved: bool,
    /// The person picked an option the Job's question gave a preview.
    pub idea_picked: bool,
    /// The recorded plan names a walk or says the person would see the change.
    pub plan_expects_walk: bool,
    /// The steps the person wrote on after they began: a message passes a step that waits on a lifecycle.
    pub messaged: StepBits,
}

/// The most steps the person can skip: the bits of [`Skipped`].
pub(crate) const SKIPPABLE: usize = 64;

/// The steps the person skipped, one bit an index, so the facts stay `Copy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Skipped(u64);

impl Skipped {
    pub(crate) fn of(steps: &[u32]) -> Skipped {
        Skipped(steps.iter().filter(|at| (**at as usize) < SKIPPABLE).fold(0, |bits, at| bits | 1 << at))
    }

    pub(crate) fn has(self, at: usize) -> bool {
        at < SKIPPABLE && self.0 & (1 << at) != 0
    }

    fn last(self) -> Option<usize> {
        (self.0 != 0).then(|| SKIPPABLE - 1 - self.0.leading_zeros() as usize)
    }
}

/// What was observed, as the rules read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    pub replied: bool,
    pub plan_recorded: bool,
    pub plan_approved: bool,
    pub tasks_open: bool,
    pub edited: bool,
    pub committed: bool,
    pub checked: bool,
    pub pushed: bool,
    pub pr_opened: bool,
    pub landed: bool,
    pub walk_shown: bool,
    /// Approved at or after the plan's approval. [`Extra::walk_approved`].
    pub walk_approved: bool,
    pub any_walk_approved: bool,
    pub sketch_approved: bool,
    pub document_approved: bool,
    pub draft_approved: bool,
    pub idea_picked: bool,
    pub plan_expects_walk: bool,
    /// A test file was written or edited.
    pub test_edited: bool,
    /// The steps the person skipped.
    pub skipped: Skipped,
    /// The steps he wrote on after they began. [`Extra::messaged`].
    pub messaged: StepBits,
    /// The steps that list a lifecycle end of which the Job holds an instance. Set where the row is
    /// drawn, from the instances and their moves (`rail_lifecycle::bits`).
    pub lifecycle_held: StepBits,
    /// The steps whose lifecycle end has come.
    pub lifecycle_met: StepBits,
}

impl Facts {
    pub(crate) fn of(record: &SessionRecord, extra: Extra) -> Facts {
        let reached = |id: BuildStopId| {
            record.build.as_ref().is_some_and(|build| build.stops.iter().any(|stop| stop.id == id && stop.at.is_some()))
        };
        Facts {
            replied: extra.replied,
            plan_recorded: record.plan.is_some(),
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
            walk_shown: extra.walk_shown,
            walk_approved: extra.walk_approved,
            any_walk_approved: extra.any_walk_approved,
            sketch_approved: extra.sketch_approved,
            document_approved: extra.document_approved,
            draft_approved: extra.draft_approved,
            idea_picked: extra.idea_picked,
            plan_expects_walk: extra.plan_expects_walk,
            test_edited: record
                .touches
                .iter()
                .any(|touch| touch.act != TouchAct::Remove && crate::test_paths::is_test_path(&touch.path)),
            skipped: Skipped::of(&record.skipped_steps),
            messaged: extra.messaged,
            ..Facts::default()
        }
    }

    /// The facts with the person's skips counted: a skipped recording step reads as a plan recorded and
    /// approved once the Job has edited, or every step that waits on the plan would wait for good.
    fn waived(self, steps: &[RailStep]) -> Facts {
        match recorder(steps) {
            Some(at) if self.skipped.has(at) && self.edited && !self.plan_approved => {
                Facts { plan_recorded: true, plan_approved: true, ..self }
            }
            _ => self,
        }
    }

    /// A walk is expected: the plan said the person would see the change or named a walk, or a
    /// window was shown.
    pub(crate) fn walk_expected(&self) -> bool {
        self.plan_expects_walk || self.walk_shown
    }

    /// The walk was approved since the plan was, or nothing would be seen: the plan was approved
    /// with no walk expected and the step's own work is done, which for a `walk` that also follows the
    /// plan is its tasks finished. A walk approved before the plan was ends a step before the plan and
    /// never this one. **Not the push**: the push ends the step after, so a Job waiting to be moved
    /// on from a walk it had nothing to show for waited for good (30 Oct 2026).
    pub(crate) fn walk_passed(&self, walk: &RailStep) -> bool {
        self.walk_approved
            || (self.plan_approved && !self.walk_expected() && (!walk.follows_plan || !self.tasks_open || self.pushed))
    }

    /// Whether `end`, listed by the step at `at`, holds. **A lifecycle end holds when it has come, or
    /// when the person wrote on the step**: a Job parked on one he cannot see the end of is his to move.
    pub(crate) fn ended(&self, at: usize, end: &StepEnd) -> bool {
        match end {
            StepEnd::Lifecycle(_) => self.lifecycle_met.has(at) || self.messaged.has(at),
            StepEnd::Replied => self.replied,
            StepEnd::WalkApproved => self.any_walk_approved,
            StepEnd::SketchApproved => self.sketch_approved,
            StepEnd::DraftApproved => self.draft_approved,
            StepEnd::IdeaPicked => self.idea_picked,
            StepEnd::DocumentApproved => self.document_approved,
        }
    }

    fn any_ended(&self, at: usize, ends: &[StepEnd]) -> bool {
        ends.iter().any(|end| self.ended(at, end))
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

/// The step that records the plan.
fn recorder(steps: &[RailStep]) -> Option<usize> {
    steps.iter().position(|step| step.records_plan)
}

/// Whether the Session is past an anchor. A step that is no anchor is not in the way.
fn passed(steps: &[RailStep], at: usize, facts: &Facts) -> bool {
    if facts.skipped.has(at) {
        return true;
    }
    let step = &steps[at];
    if step.records_plan {
        return facts.plan_approved;
    }
    if step.walks {
        return facts.walk_passed(step);
    }
    if step.follows_plan {
        return facts.plan_approved && !facts.tasks_open;
    }
    if step.delivers {
        return facts.landed;
    }
    if recorder(steps).is_some_and(|recording| at < recording) {
        return facts.plan_recorded || facts.any_ended(at, &step.ends_on);
    }
    // A step that lists a lifecycle end holds the Job until it comes.
    !step.has_lifecycle_end() || facts.any_ended(at, &step.ends_on)
}

fn entered(steps: &[RailStep], at: usize, facts: &Facts) -> bool {
    if facts.skipped.has(at) {
        return false;
    }
    let step = &steps[at];
    if step.records_plan {
        return match at {
            0 => !facts.plan_approved,
            _ => facts.plan_recorded && !facts.plan_approved,
        };
    }
    if step.walks {
        return facts.plan_approved && !facts.walk_passed(step);
    }
    if step.follows_plan {
        return facts.plan_approved
            && steps[..at].iter().enumerate().all(|(earlier, one)| !one.walks || passed(steps, earlier, facts));
    }
    if step.delivers {
        // A step that waits on a lifecycle holds the Job past the push, as a walk holds a refine step.
        return facts.pushed
            && steps[..at].iter().enumerate().all(|(earlier, one)| !one.has_lifecycle_end() || passed(steps, earlier, facts));
    }
    let before = |earlier: usize| (0..earlier).all(|one| passed(steps, one, facts));
    if recorder(steps).is_some_and(|recording| at < recording) {
        return !facts.any_ended(at, &step.ends_on) && !facts.plan_recorded && before(at);
    }
    // **A step that waits on a lifecycle is entered once the Job holds one**, and left when its end
    // comes. The step after it is entered at that moment, whatever fact would have entered it: the
    // Job has moved on, and showing the step before it would say it had not.
    if step.has_lifecycle_end() {
        return facts.lifecycle_held.has(at) && !facts.any_ended(at, &step.ends_on) && before(at);
    }
    let after_a_wait = at > 0 && steps[at - 1].has_lifecycle_end() && before(at);
    // **The step after a walk is entered once the walk is passed**, as after a wait: else the Job falls
    // back behind the walk he approved, is told to show it again, and the new window voids his approval.
    let after_a_walk = at > 0 && steps[at - 1].walks && passed(steps, at - 1, facts);
    let Some(fit) = step.fit else {
        return after_a_wait || after_a_walk;
    };
    after_a_wait || after_a_walk || (facts.has(fit) && before(at) && !(at > 0 && steps[at - 1].fit == Some(fit)))
}

/// The step the Session is on: the last one entered, else the first standing. **Absent** once the
/// delivering step's pull request merged and nothing follows it, and for a workflow of no steps.
pub(crate) fn current(steps: &[RailStep], facts: &Facts) -> Option<usize> {
    if steps.is_empty() {
        return None;
    }
    let facts = &facts.waived(steps);
    if let Some(delivering) = steps.iter().position(|step| step.delivers).filter(|_| facts.landed) {
        // A step after it that waits on a lifecycle is left when its end has come.
        let over = |at: usize| facts.skipped.has(at) || (steps[at].has_lifecycle_end() && facts.any_ended(at, &steps[at].ends_on));
        return (delivering + 1..steps.len()).find(|at| !over(*at));
    }
    // The last step waited on a lifecycle and its end has come: nothing follows it.
    let last = steps.len() - 1;
    if steps[last].has_lifecycle_end() && (0..=last).all(|at| passed(steps, at, facts)) && facts.any_ended(last, &steps[last].ends_on) {
        return None;
    }
    Some((0..steps.len()).rev().find(|at| entered(steps, *at, facts)).unwrap_or_else(|| standing(steps, facts)))
}

/// Where no step is entered: the first step, or, past the person's skips, the first step neither
/// skipped nor passed before the last of them. With nothing skipped this is the first step.
fn standing(steps: &[RailStep], facts: &Facts) -> usize {
    let last = facts.skipped.last();
    (0..steps.len())
        .find(|at| !facts.skipped.has(*at) && !last.is_some_and(|last| *at < last && passed(steps, *at, facts)))
        .unwrap_or(0)
}

/// Which step was skipped, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Skip {
    /// Files were edited before the plan was approved.
    Plan,
    /// The work was pushed while the plan's tasks were open.
    Tasks,
    /// A test file was edited before the walk was approved.
    Walk,
    /// In a workflow with a walk step: a test file was edited before the plan was approved.
    PlanTested,
    /// In a workflow with a walk step: the work was pushed before the plan was approved.
    PlanPushed,
}

impl Skip {
    /// What happened, as the workflow's page lists it under the step.
    pub(crate) fn fact(self) -> &'static str {
        match self {
            Skip::Plan => "edited before the plan was approved",
            Skip::Tasks => "pushed with the plan's tasks open",
            Skip::Walk => "a test before the walk was approved",
            Skip::PlanTested => "a test before the plan was approved",
            Skip::PlanPushed => "pushed before the plan was approved",
        }
    }
}

/// The steps the observed facts show skipped. A step is named however often it is skipped; saying so
/// only once is the caller's.
pub(crate) fn skipped(steps: &[RailStep], facts: &Facts) -> Vec<(usize, Skip)> {
    let facts = &facts.waived(steps);
    let mut found = Vec::new();
    if let Some(at) = steps.iter().position(|step| step.records_plan).filter(|_| !facts.plan_approved) {
        // Where the workflow walks, a rough mock may be edited while the plan is discussed.
        let skip = if !steps.iter().any(|step| step.walks) {
            facts.edited.then_some(Skip::Plan)
        } else if facts.test_edited {
            Some(Skip::PlanTested)
        } else {
            facts.pushed.then_some(Skip::PlanPushed)
        };
        found.extend(skip.map(|skip| (at, skip)));
    }
    if let Some(at) = steps.iter().position(|step| step.follows_plan).filter(|_| facts.pushed && facts.tasks_open) {
        found.push((at, Skip::Tasks));
    }
    if let Some(at) = steps.iter().position(|step| step.walks).filter(|at| {
        facts.test_edited && facts.plan_approved && facts.walk_expected() && !facts.walk_passed(&steps[*at])
    }) {
        found.push((at, Skip::Walk));
    }
    // A step the person skipped is his call, not the agent's drift.
    found.retain(|(at, _)| !facts.skipped.has(*at));
    found
}

/// The one message to a Job that has run when the person skips a step of its workflow, or undoes it.
pub(crate) fn skip_told(workflow: &str, step: &RailStep, skipped: bool) -> String {
    if skipped {
        format!(
            "Fleet: the person skipped the \"{}\" step of this Job's {workflow} workflow. Do not do that step, \
             and do not ask about it. Fleet counts it passed.",
            step.label
        )
    } else {
        format!(
            "Fleet: the person undid the skip of the \"{}\" step of this Job's {workflow} workflow, so that step \
             is wanted again. Fleet places the Job on it when it comes.",
            step.label
        )
    }
}

/// The steps the person skipped, said with a Job's first turn: a message before it would start the
/// Job ahead of the turn it waits for. `None` where none is.
pub(crate) fn skips_said(workflow: &str, steps: &[RailStep], skipped: &[u32]) -> Option<String> {
    let labels: Vec<String> =
        skipped.iter().filter_map(|at| steps.get(*at as usize)).map(|step| format!("\"{}\"", step.label)).collect();
    match labels.as_slice() {
        [] => None,
        [one] => Some(format!(
            "Fleet: the person skipped the {one} step of this Job's {workflow} workflow. Do not do it, and do not ask about it."
        )),
        _ => Some(format!(
            "Fleet: the person skipped these steps of this Job's {workflow} workflow: {}. Do not do them, and do not ask about them.",
            labels.join(", ")
        )),
    }
}

/// The most of a step's own words a nudge repeats.
const ABOUT_AT_MOST: usize = 200;

/// The one short message that names the step skipped and what it asks. Nothing is blocked.
pub(crate) fn nudge(workflow: &str, step: &RailStep, skip: Skip) -> String {
    let asks = match step.about.as_deref().map(str::trim).filter(|about| !about.is_empty()) {
        Some(about) => about.chars().take(ABOUT_AT_MOST).collect::<String>(),
        None => match skip {
            Skip::Plan | Skip::PlanTested | Skip::PlanPushed => {
                String::from("a plan recorded with record_plan, and approved by the person")
            }
            Skip::Tasks => String::from("the plan's tasks worked and finished before the work goes out"),
            Skip::Walk => String::from("the work shown as a walk and approved by the person before any test is written"),
        },
    };
    let did = match skip {
        Skip::Plan => "edited files before",
        Skip::Tasks => "pushed while tasks were still open in",
        Skip::Walk => "edited a test before the walk was approved in",
        Skip::PlanTested => "edited a test before the plan was approved in",
        Skip::PlanPushed => "pushed before the plan was approved in",
    };
    format!(
        "Fleet: this Job follows the {workflow} workflow, and you {did} its \"{}\" step. That step asks: {asks}. \
         Nothing is blocked. Take it up now, or say why the work goes on without it.",
        step.label
    )
}

/// The step the Job stopped on, where only its own act ends it: the turn ended on a reply that asks
/// nothing, nothing waits on the person, and what ends the step is something the Job does. Fleet never
/// moves such a step, so a Job waiting for the move waits for good. Read from the row, never the words.
/// **A delivering step with no pull request opened is the Job's too**: the merge that ends it is not, but
/// nothing reaches the merge until the Job delivers. **So is a walk that follows the plan with nothing to
/// show**, whose tasks are what end it; `extra` says whether a walk is expected.
pub(crate) fn stalled(steps: &[RailStep], record: &SessionRecord, extra: Extra) -> Option<(usize, rail_ends::End)> {
    if record.standing != Some(SessionStanding::Replied) || !record.waiting_for.is_empty() {
        return None;
    }
    let at = record.workflow_step? as usize;
    let step = steps.get(at)?;
    if step.delivers {
        return (!Facts::of(record, Extra::default()).pr_opened).then_some((at, rail_ends::End::Undelivered));
    }
    if step.walks && step.follows_plan && !Facts::of(record, extra).walk_expected() {
        return Some((at, rail_ends::End::TasksDone));
    }
    let end = rail_ends::end_of(steps, at).filter(|end| end.by_the_job())?;
    Some((at, end))
}

/// The one message to a Job stopped on a step only it can end.
pub(crate) fn stall_nudge(workflow: &str, step: &RailStep, end: rail_ends::End) -> String {
    if end == rail_ends::End::Undelivered {
        return format!(
            "Fleet: this Job follows the {workflow} workflow, and your turn ended on its \"{}\" step with no pull \
             request opened and nothing asked or shown. Deliver now, the way the person chose: with Auto land or \
             Open PR only, open the pull request; with Keep local, call finish_up and then all_done. If you are \
             waiting on work you started, or the person said to wait, carry on.",
            step.label
        );
    }
    format!(
        "Fleet: this Job follows the {workflow} workflow, and your turn ended on its \"{}\" step with nothing \
         asked or shown. This step ends when {}, and Fleet does not move it for you. Do that now, or ask the \
         person what is still open. If you are waiting on work you started, or the person said to wait, carry on.",
        step.label,
        end.for_agent()
    )
}

/// What the agent is told on entering step `at`: the step's own instructions, named by its step, and
/// what ends it. **The end is said because Fleet does not move a step the Job's own act ends**: a Job
/// left to wait for the move waits for good. `None` where the step wrote none, and nothing is sent.
pub(crate) fn instructed(workflow: &str, steps: &[RailStep], at: usize) -> Option<String> {
    let step = steps.get(at)?;
    let words = step.instructions.as_deref().map(str::trim).filter(|words| !words.is_empty());
    let gate = rail_ends::gate_said(step);
    if words.is_none() && gate.is_none() {
        return None;
    }
    let mut said = format!("Fleet: this Job follows the {workflow} workflow, and you are on its \"{}\" step.", step.label);
    if let Some(words) = words {
        said.push_str(&format!("\n\n{words}"));
    }
    let ends = rail_ends::ends_of(steps, at);
    if !ends.is_empty() {
        let each: Vec<_> = ends.iter().map(|end| end.for_agent()).collect();
        said.push_str(&format!("\n\nThis step ends when {}.", each.join(", or ")));
        // A Job with nothing to show read the walk's end as already met, and waited to be moved on.
        if step.walks && step.follows_plan {
            said.push_str(&format!(" Where nothing would be seen, it ends when {}.", rail_ends::End::TasksDone.for_agent()));
        }
        // Fleet moves the Job on by itself, so the agent has nothing to do but stop.
        if ends.iter().any(rail_ends::End::by_the_world) {
            said.push_str(" End your turn now: Fleet wakes you with a message when it comes, so do not poll for it.");
        }
        // A Job that pushed and stopped, waiting to be told to deliver, sat idle (28 Oct 2026).
        if let Some(next) = steps.get(at + 1).filter(|next| next.delivers && ends.iter().all(|end| end.by_the_job())) {
            said.push_str(&format!(
                " Then carry straight on with its \"{}\" step in the same turn, the way the person chose to deliver: \
                 do not end your turn to wait for Fleet's message.",
                next.label
            ));
        }
    }
    // A delivering step gated by a lifecycle: it is entered as always, and the agent holds the merge.
    if let Some(gate) = gate {
        said.push_str(&format!(" {gate}"));
    }
    Some(said)
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
