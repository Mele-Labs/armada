//! A step that ends on a lifecycle state, read off what a Job holds. **Pure**: the instances and the
//! moves kept for them are passed in, so each rule is tested without a Fleet. `docs/concepts/
//! session/the-rail.md`, *A step that ends on a lifecycle state*.
//!
//! **An instance has reached a state once it has stood in it**, and stays reached: the rail is
//! derived each time the row is drawn, so a fact that could be undone (a reviewer taking back an
//! approval) would drag a Job back onto a step it left. **One that is over has reached any state of
//! its lifecycle**, since nothing more will happen to it and the step would otherwise wait for good.
//! **A step waits on instances that exist**: a Job holding none of the lifecycle is not on the step.

use core_model::{Lifecycle, LifecycleEnd, StateId, Standing, Waits};
use store::{KeptInstance, KeptMove};

use crate::rail::RailStep;

/// One bit a step index, so the facts a step is read against stay `Copy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepBits(u64);

impl StepBits {
    pub(crate) fn of(steps: impl IntoIterator<Item = usize>) -> StepBits {
        StepBits(steps.into_iter().filter(|at| *at < crate::rail::SKIPPABLE).fold(0, |bits, at| bits | 1 << at))
    }

    pub(crate) fn has(self, at: usize) -> bool {
        at < crate::rail::SKIPPABLE && self.0 & (1 << at) != 0
    }
}

/// What the store keeps of a Job's instances.
#[derive(Clone, Debug, Default)]
pub(crate) struct Held {
    pub instances: Vec<KeptInstance>,
    pub moves: Vec<KeptMove>,
}

/// Where the instances of one lifecycle stand against an end.
pub(crate) struct Reading<'a> {
    pub reached: Vec<&'a KeptInstance>,
    pub pending: Vec<&'a KeptInstance>,
}

impl Reading<'_> {
    /// Whether the end holds: every one the Job holds, or any one. **None held is not met.**
    pub(crate) fn met(&self, waits: Waits) -> bool {
        match waits {
            Waits::Every => !self.reached.is_empty() && self.pending.is_empty(),
            Waits::Any => !self.reached.is_empty(),
        }
    }

    pub(crate) fn held(&self) -> bool {
        !self.reached.is_empty() || !self.pending.is_empty()
    }
}

/// The Job's instances of the end's lifecycle, each as reached or pending. **One it handed to another
/// Job is neither**: nothing more will be read of it here, so it would be pending for good, and it is
/// the other Job's to wait on.
pub(crate) fn read<'a>(end: &LifecycleEnd, lifecycle: &Lifecycle, held: &'a Held) -> Reading<'a> {
    let (reached, pending) = held
        .instances
        .iter()
        .filter(|one| one.lifecycle == end.lifecycle().as_str() && one.handed_to.is_empty())
        .partition(|one| has_reached(end.state(), lifecycle, one, &held.moves));
    Reading { reached, pending }
}

fn has_reached(state: &StateId, lifecycle: &Lifecycle, one: &KeptInstance, moves: &[KeptMove]) -> bool {
    moves.iter().filter(|kept| kept.lifecycle == one.lifecycle && kept.key == one.key).any(|kept| {
        matches!(Standing::from_wire(&kept.to), Some(Standing::In(entered)) if &entered == state || lifecycle.ends().contains(&entered))
    })
}

/// The steps that hold on a lifecycle (an instance of one is held) and the steps whose end has come.
/// **A step lists several ends and any one ends it.**
pub(crate) fn bits(steps: &[RailStep], lifecycles: &[Lifecycle], held: &Held) -> (StepBits, StepBits) {
    let (mut holding, mut met) = (Vec::new(), Vec::new());
    for (at, step) in steps.iter().enumerate() {
        for end in step.lifecycle_ends() {
            let Some(lifecycle) = lifecycles.iter().find(|one| one.id() == end.lifecycle()) else { continue };
            let reading = read(end, lifecycle, held);
            if reading.held() {
                holding.push(at);
            }
            if reading.met(end.waits()) {
                met.push(at);
            }
        }
    }
    (StepBits::of(holding), StepBits::of(met))
}

/// What an instance is called: a pull request by its number, a ticket by its key, a Job by its
/// title where it has one. `title` is looked up by the caller, since only the store knows it.
pub(crate) fn named(one: &KeptInstance, title: Option<&str>) -> String {
    match one.lifecycle.as_str() {
        "pr" => format!("PR #{}", one.key),
        "ticket" => one.key.clone(),
        "job" => match title.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => format!("Job \"{title}\""),
            None => format!("Job {}", one.key.chars().take(8).collect::<String>()),
        },
        other => format!("{other} {}", one.key),
    }
}

fn label(lifecycle: &Lifecycle, state: &StateId) -> String {
    lifecycle.state(state).map_or_else(|| state.to_string(), |one| one.label().as_str().to_string())
}

/// What the rail says of a step that waits: `Waiting on PR #812: Approved`, served as
/// `SessionRecord::workflow_waiting_on`. **Those not there yet**, and `any of` where one is enough; with
/// none held yet, the lifecycle by its name.
pub(crate) fn waiting_on(end: &LifecycleEnd, lifecycle: &Lifecycle, reading: &Reading<'_>, name: &dyn Fn(&KeptInstance) -> String) -> String {
    let state = label(lifecycle, end.state());
    let pending: Vec<String> = match end.waits() {
        Waits::Every => reading.pending.iter().map(|one| name(one)).collect(),
        Waits::Any => reading.pending.iter().chain(&reading.reached).map(|one| name(one)).collect(),
    };
    match (pending.as_slice(), end.waits()) {
        ([], _) => format!("Waiting on a {}: {state}", lifecycle.name().as_str().to_lowercase()),
        ([one], _) => format!("Waiting on {one}: {state}"),
        (several, Waits::Any) => format!("Waiting on any of {}: {state}", several.join(", ")),
        (several, Waits::Every) => format!("Waiting on {}: {state}", several.join(", ")),
    }
}

/// What a step that waits on a lifecycle says while it waits: one `waiting_on` sentence for each of its
/// ends, in the order the step lists them. **Empty once any end has come**, since any one ends the step,
/// and empty for a step that lists no lifecycle end or only ones the Job's lifecycles do not have.
pub(crate) fn waiting(
    step: &RailStep,
    lifecycles: &[Lifecycle],
    held: &Held,
    name: &dyn Fn(&KeptInstance) -> String,
) -> Vec<String> {
    let mut said = Vec::new();
    for end in step.lifecycle_ends() {
        let Some(lifecycle) = lifecycles.iter().find(|one| one.id() == end.lifecycle()) else { continue };
        let reading = read(end, lifecycle, held);
        if reading.met(end.waits()) {
            return Vec::new();
        }
        said.push(waiting_on(end, lifecycle, &reading, name));
    }
    said
}

/// The one line that wakes the agent: what got there, and which step that ended, or, for a delivering
/// step (`lifts`), which wait it lifted. **The merge ends a delivering step**, so the state only gates it.
pub(crate) fn woke(
    step: &str,
    lifts: bool,
    workflow: &str,
    end: &LifecycleEnd,
    lifecycle: &Lifecycle,
    reading: &Reading<'_>,
    name: &dyn Fn(&KeptInstance) -> String,
) -> String {
    let state = label(lifecycle, end.state());
    let got: Vec<String> = reading.reached.iter().map(|one| name(one)).collect();
    let what = match got.as_slice() {
        [] => format!("the {} reached {state}", lifecycle.name().as_str().to_lowercase()),
        [one] => format!("{one} is {state}"),
        [rest @ .., last] => format!("{} and {last} are {state}", rest.join(", ")),
    };
    match lifts {
        true => format!("Fleet: {what}. That lifts the wait on the \"{step}\" step of this Job's {workflow} workflow: land it now."),
        false => format!("Fleet: {what}. That ends the \"{step}\" step of this Job's {workflow} workflow."),
    }
}

/// The line that wakes the agent on leaving `step`, where one of the step's ends has come. **`None` where
/// nothing got there**: a step the person skipped or wrote on first, or one that waits on no lifecycle.
pub(crate) fn woken(
    step: &RailStep,
    workflow: &str,
    lifecycles: &[Lifecycle],
    held: &Held,
    name: &dyn Fn(&KeptInstance) -> String,
) -> Option<String> {
    step.lifecycle_ends().find_map(|end| {
        let lifecycle = lifecycles.iter().find(|one| one.id() == end.lifecycle())?;
        let reading = read(end, lifecycle, held);
        reading.met(end.waits()).then(|| woke(&step.label, step.delivers, workflow, end, lifecycle, &reading, name))
    })
}
