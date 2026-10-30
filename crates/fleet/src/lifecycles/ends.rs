//! A Job's place on a workflow whose step ends on a lifecycle state, as a function over values, so
//! the claim *a step ends on `pr: approved`, the agent is stopped while it waits and Fleet wakes it
//! on the transition* is asserted without a Fleet. **The rail draws the same rules** from what the
//! store keeps; this hands them what the readers' own types hold (`holding`).

use core_model::{HeldInstances, Lifecycle};
use config::ResolvedWorkflow;
use store::{KeptInstance, KeptMove};

use super::holding::Transition;
use crate::rail::{self, Facts};
use crate::rail_ends;
use crate::rail_lifecycle::{self, Held};

/// Where a Job stands, and what the step it is on does to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The step by its index; absent once the last step's end has come.
    pub step: Option<usize>,
    /// What the agent is told on entering the step.
    pub told: Option<String>,
    /// Whether ending the step is something the person does. **A lifecycle state is not.**
    pub waits_on_the_person: bool,
}

fn kept(held: &HeldInstances, moves: &[Transition]) -> Held {
    let instances = held.all().iter().map(|one| KeptInstance {
        session_id: String::new(),
        lifecycle: one.subject().lifecycle().as_str().to_string(),
        key: one.subject().key().to_string(),
        links: Vec::new(),
        seen_source: String::new(),
        seen: String::new(),
        watch_id: String::new(),
        taken_on_at: String::new(),
    });
    let moves = moves.iter().enumerate().map(|(n, one)| KeptMove {
        id: n as i64,
        session_id: String::new(),
        lifecycle: one.instance.lifecycle().as_str().to_string(),
        key: one.instance.key().to_string(),
        from: one.moved.from.as_wire(),
        to: one.moved.to.as_wire(),
        seen_source: one.source.as_wire().to_string(),
        seen: one.seen.clone(),
        at: one.at.clone(),
    });
    Held { instances: instances.collect(), moves: moves.collect() }
}

/// Where a Job whose plan is approved stands on `workflow`, holding `held` and having moved as
/// `moves` say.
pub fn place(workflow: &ResolvedWorkflow, lifecycles: &[Lifecycle], held: &HeldInstances, moves: &[Transition]) -> Place {
    let steps = rail::steps_of(workflow);
    let mut facts = Facts { plan_approved: true, ..Facts::default() };
    (facts.lifecycle_held, facts.lifecycle_met) = rail_lifecycle::bits(&steps, lifecycles, &kept(held, moves));
    let step = rail::current(&steps, &facts);
    Place {
        step,
        told: step.and_then(|at| rail::instructed(workflow.name(), &steps, at)),
        waits_on_the_person: step
            .is_some_and(|at| rail_ends::ends_of(&steps, at).iter().any(|end| !end.by_the_world() && !end.by_the_job())),
    }
}

/// The line that wakes the agent on leaving `step`, where the moves satisfy one of its ends.
pub fn woken(
    workflow: &ResolvedWorkflow,
    lifecycles: &[Lifecycle],
    held: &HeldInstances,
    moves: &[Transition],
    step: usize,
) -> Option<String> {
    let steps = rail::steps_of(workflow);
    let name = |one: &KeptInstance| rail_lifecycle::named(one, None);
    rail_lifecycle::woken(steps.get(step)?, workflow.name(), lifecycles, &kept(held, moves), &name)
}
