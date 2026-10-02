//! Which model a step's Drone is run as.
//!
//! Its own file rather than a section of [`super::record`], for the reason that
//! one is about what a transition writes down: this is a read, it has one
//! rule — the step's, or the Job's — and the rule is the whole subject.
//!
//! **It could not be asked before a step was its own process.** One session
//! spanned a whole Job and a session cannot change model partway, so the Job's
//! model was the answer at every step by construction and there was no fallback
//! to spell.

use super::*;

/// The fixture workflow with `fix` naming a model of its own and `repro`
/// naming none, which is the shape every shipped workflow has: the annotation
/// is on the step that wants something other than the Job's.
fn workflow_with_a_modelled_step() -> FrozenWorkflow {
    let steps: Vec<ResolvedStep> = workflow()
        .steps()
        .iter()
        .map(|step| {
            let model = match step.id().as_str() {
                "fix" => Some(ModelName::new("the-steps-own-model").expect("a model name")),
                _ => None,
            };
            ResolvedStep::frozen(
                step.id().clone(),
                step.label().to_string(),
                step.evidence_type(),
                step.checks().to_vec(),
                step.advance_gate(),
                step.judge_checks().to_vec(),
                step.evidence_scope().cloned(),
                step.retry_limit(),
                model,
            )
        })
        .collect();
    FrozenWorkflow::frozen(
        workflow().id().clone(),
        workflow().name().to_string(),
        workflow().version(),
        steps,
    )
}

fn job_whose_fix_step_names_a_model() -> Job {
    let mut draft = draft();
    draft.workflow = workflow_with_a_modelled_step();
    Job::create_top_level(
        draft,
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

/// **A step that names none is run as the Job was proposed.** This is what
/// every step did while one process spanned a whole Job, and removing it would
/// make every workflow have to state a model on every step.
#[test]
fn a_step_naming_no_model_falls_back_to_the_jobs() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("repro")).as_str(),
        job.model().as_str()
    );
}

/// **A step that names one gets it**, which is the whole of what the dial does.
#[test]
fn a_step_naming_a_model_is_run_as_that_one() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("fix")).as_str(),
        "the-steps-own-model"
    );
    assert_ne!(job.model().as_str(), "the-steps-own-model");
}

/// **A person's choice beats both**: the step's own model and the Job's.
#[test]
fn a_chosen_model_wins_over_the_steps_own_and_the_jobs() {
    let job = job_whose_fix_step_names_a_model();
    let chosen = ModelName::new("a-persons-choice").expect("a model name");
    for step in ["fix", "repro"] {
        assert_eq!(
            job.model_spawned_at(&StepId::new(step), Some(&chosen))
                .as_str(),
            "a-persons-choice",
            "{step}"
        );
    }
}

/// With nothing chosen, the answer is `model_at`'s, fallback and all.
#[test]
fn with_nothing_chosen_each_step_runs_as_model_at_says() {
    let job = job_whose_fix_step_names_a_model();
    for step in ["fix", "repro"] {
        let step = StepId::new(step);
        assert_eq!(job.model_spawned_at(&step, None), job.model_at(&step));
    }
}

/// A step id the workflow does not declare answers with the Job's rather than
/// panicking or inventing one. **Not a case to rely on**: a Drone is only ever
/// put on a step the frozen workflow names, and a caller that reached here has
/// a larger problem than which model it got.
#[test]
fn a_step_this_workflow_does_not_declare_answers_with_the_jobs() {
    let job = job_whose_fix_step_names_a_model();
    assert_eq!(
        job.model_at(&StepId::new("no-such-step")).as_str(),
        job.model().as_str()
    );
}

/// A plan of three tasks: one the planner called difficult, one it left to
/// Armada, and one a person then picked a model for.
fn tiered_plan() -> crate::WorkPlan {
    use crate::{
        Approach, PlanAuthor, PlanChange, PlanEntry, TaskEdit, TaskId, TaskTier, WorkPlan,
    };
    let when = at("2026-10-02T10:00:00.000Z");
    let recorded = PlanEntry {
        change: PlanChange::Recorded {
            approach: Approach::new("Bound the reader").expect("an approach"),
            tasks: vec![
                crate::NewTask::new("hard", "", &[], "")
                    .expect("a title")
                    .at_tier(Some(TaskTier::Difficult)),
                crate::NewTask::new("left to Armada", "", &[], "").expect("a title"),
                crate::NewTask::new("picked", "", &[], "")
                    .expect("a title")
                    .at_tier(Some(TaskTier::Difficult)),
            ],
        },
        by: PlanAuthor::Person,
        at: when.clone(),
    };
    let picked = PlanEntry {
        change: PlanChange::Edited {
            task: TaskId::read("T3").expect("a task id"),
            edit: TaskEdit::new(
                None,
                None,
                None,
                None,
                Some(ModelName::new("a-persons-pick").expect("a model name")),
            )
            .expect("an edit"),
        },
        by: PlanAuthor::Person,
        at: when,
    };
    WorkPlan::fold(&[recorded, picked])
        .expect("the history replays")
        .expect("a plan")
}

/// **A person's pick, then the map for the tier, then `model_spawned_at`**
/// (spike 022, *which model a Drone runs*). A tier the map leaves out is
/// Armada picking, which is the step's model and then the Job's (answer 8).
#[test]
fn a_tasks_model_is_the_persons_pick_then_the_map_then_the_step_and_the_job() {
    use crate::{TaskId, TaskTier, TierModels};
    let job = job_whose_fix_step_names_a_model();
    let plan = tiered_plan();
    let task = |id: &str| plan.task(TaskId::read(id).expect("a task id"));
    let tiers = TierModels::default().with(
        TaskTier::Difficult,
        ModelName::new("strong").expect("a name"),
    );
    let fix = StepId::new("fix");
    let ran = |id: &str| {
        job.model_spawned_for(&fix, None, task(id), &tiers)
            .as_str()
            .to_string()
    };
    assert_eq!(ran("T1"), "strong", "the map, for the task's tier");
    assert_eq!(ran("T2"), "the-steps-own-model", "no tier: the step's own");
    assert_eq!(ran("T3"), "a-persons-pick", "a person's pick beats the map");
    assert_eq!(
        job.model_spawned_for(&fix, None, task("T1"), &TierModels::default())
            .as_str(),
        "the-steps-own-model",
        "a tier the map leaves out falls through"
    );
    assert_eq!(
        job.model_spawned_for(&fix, None, None, &tiers),
        job.model_spawned_at(&fix, None),
        "a Drone on no task runs as before"
    );
}
