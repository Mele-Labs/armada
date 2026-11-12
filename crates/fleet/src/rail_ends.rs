//! What ends a step, as a short fact, read off the step's anchor. **Pure**, and beside `rail`, whose
//! rules it restates for a person; nothing here is a second way to place a step.

use std::borrow::Cow;

use core_model::{LifecycleEnd, StepEnd, Waits};
use ipc::WaitingSource;

use crate::rail::{Fit, RailStep};

/// The fact that ends a step, said to the person on the workflow's page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum End {
    Merged,
    PlanApproved,
    WalkApproved,
    Replied,
    /// A walk approved, as a step's `ends_on` lists it: any window, whenever approved.
    AnyWalkApproved,
    SketchApproved,
    DraftApproved,
    IdeaPicked,
    DocumentApproved,
    YouApproved,
    PlanRecorded,
    Pushed,
    WalkShown,
    Edited,
    Checked,
    Committed,
    PrOpened,
    /// Instances of a lifecycle the Job holds reach a state. The world's to bring about, and Fleet
    /// reads it: neither the Job's act nor the person's.
    Lifecycle(LifecycleEnd),
}

/// `in_review` as `in review`, the way a state reads in a sentence.
fn spoken(id: impl std::fmt::Display) -> String {
    id.to_string().replace('_', " ")
}

/// Who a step's end waits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Waiter {
    /// The Job's own act ends it.
    TheJob,
    /// Its pull request merging, which is nobody's to press.
    TheMerge,
    /// A lifecycle reaching a state, which Fleet reads off the world and moves the Job on: neither the
    /// Job's act nor the person's, so no item waits on him.
    TheWorld,
    /// The person, through the item of this source.
    ThePerson(WaitingSource),
}

impl End {
    /// As the workflow's page says it, to the person.
    pub(crate) fn for_person(&self) -> Cow<'static, str> {
        match self {
            End::Lifecycle(end) => {
                let which = if end.waits() == Waits::Any { "any" } else { "every" };
                format!("{which} {} is {}", spoken(end.lifecycle()), spoken(end.state())).into()
            }
            End::Merged => "it is merged".into(),
            End::PlanApproved => "you approve the plan".into(),
            End::WalkApproved => "you approve every walk".into(),
            End::Replied => "you reply".into(),
            End::AnyWalkApproved => "you approve a walk".into(),
            End::SketchApproved => "you approve a sketch".into(),
            End::DraftApproved => "you approve a draft".into(),
            End::IdeaPicked => "you pick one of the ideas shown".into(),
            End::DocumentApproved => "you approve a document".into(),
            End::YouApproved => "you approve".into(),
            End::PlanRecorded => "a plan is recorded".into(),
            End::Pushed => "it is pushed".into(),
            End::WalkShown => "a walk is shown".into(),
            End::Edited => "a file is edited".into(),
            End::Checked => "a Check is run".into(),
            End::Committed => "a commit is made".into(),
            End::PrOpened => "a pull request is opened".into(),
        }
    }

    /// Who ends the step, and for the person the item that waits on him while it stands. **Every end
    /// is named here**, so an end added with no wait behind it does not build. The item is raised off
    /// what the Job showed or said, never off the agent's `waiting_for`.
    pub(crate) fn waits_on(&self) -> Waiter {
        match self {
            End::Lifecycle(_) => Waiter::TheWorld,
            // His Approve on the pull request is a forge's; the merge itself is the queue's, never his.
            End::Merged => Waiter::TheMerge,
            End::PlanApproved => Waiter::ThePerson(WaitingSource::Plan),
            End::WalkApproved | End::AnyWalkApproved => Waiter::ThePerson(WaitingSource::Walk),
            End::Replied => Waiter::ThePerson(WaitingSource::Replied),
            End::SketchApproved => Waiter::ThePerson(WaitingSource::Sketch),
            End::DraftApproved => Waiter::ThePerson(WaitingSource::Draft),
            End::IdeaPicked => Waiter::ThePerson(WaitingSource::AskCard),
            End::DocumentApproved => Waiter::ThePerson(WaitingSource::Document),
            End::YouApproved => Waiter::ThePerson(WaitingSource::Replied),
            End::PlanRecorded
            | End::Pushed
            | End::WalkShown
            | End::Edited
            | End::Checked
            | End::Committed
            | End::PrOpened => Waiter::TheJob,
        }
    }

    /// The Job's own act ends the step, so nobody else will: Fleet does not move it, and the person
    /// has nothing to answer.
    pub(crate) fn by_the_job(&self) -> bool {
        self.waits_on() == Waiter::TheJob
    }

    /// Fleet reads it off the world and moves the Job, so it raises no wait for the person.
    pub(crate) fn by_the_world(&self) -> bool {
        self.waits_on() == Waiter::TheWorld
    }

    fn listed(end: &StepEnd) -> End {
        match end {
            StepEnd::Replied => End::Replied,
            StepEnd::WalkApproved => End::AnyWalkApproved,
            StepEnd::SketchApproved => End::SketchApproved,
            StepEnd::DraftApproved => End::DraftApproved,
            StepEnd::IdeaPicked => End::IdeaPicked,
            StepEnd::DocumentApproved => End::DocumentApproved,
            StepEnd::YouApproved => End::YouApproved,
            StepEnd::Lifecycle(end) => End::Lifecycle(end.clone()),
        }
    }
}

/// What a delivering step's lifecycle ends gate the merge on, to the person: `every pr is approved`, and
/// `, or ` between several. `None` for a step that does not deliver or lists none.
fn gate_for_person(step: &RailStep) -> Option<String> {
    let gates: Vec<_> = step.lifecycle_ends().map(|end| End::Lifecycle(end.clone()).for_person()).collect();
    (step.delivers && !gates.is_empty()).then(|| gates.join(", or "))
}

/// Everything that ends the step at `at`, any one of them, or none where nothing observable does. A
/// step before the recording step that lists its ends is ended by those; the plan recorded is said
/// only where it lists none.
pub(crate) fn ends_of(steps: &[RailStep], at: usize) -> Vec<End> {
    let Some(step) = steps.get(at) else { return Vec::new() };
    if step.delivers {
        return vec![End::Merged];
    }
    if step.records_plan && !step.plan_unasked {
        return std::iter::once(End::PlanApproved).chain(step.ends_on.iter().map(End::listed)).collect();
    }
    if step.records_plan && !step.follows_plan {
        return vec![End::PlanRecorded];
    }
    if step.walks {
        return vec![End::WalkApproved];
    }
    if step.has_lifecycle_end() || step.ends_on.contains(&StepEnd::YouApproved) {
        return step.ends_on.iter().map(End::listed).collect();
    }
    if steps.iter().position(|one| one.records_plan).is_some_and(|recording| at < recording) {
        return match step.ends_on.is_empty() {
            true => vec![End::PlanRecorded],
            false => step.ends_on.iter().map(End::listed).collect(),
        };
    }
    steps.get(at + 1).and_then(entry).into_iter().collect()
}

/// What ends the step at `at` where one thing does, or `None` where several or nothing does.
pub(crate) fn end_of(steps: &[RailStep], at: usize) -> Option<End> {
    match ends_of(steps, at).as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// What ends the step at `at`, to the person, where one thing does. Served by the workflow's page.
pub(crate) fn ends_when(steps: &[RailStep], at: usize) -> Option<String> {
    end_of(steps, at).map(|end| end.for_person().into_owned())
}

/// Everything that ends the step at `at`, to the person, any one of them. Served by the workflow's page
/// as `StepReading::ends_any`. **A delivering step gated by a lifecycle says the gate and then the
/// merge**, `every pr is approved, then it is merged`: the state does not end the step, so `ends_when`
/// keeps saying only the merge.
pub(crate) fn ends_any(steps: &[RailStep], at: usize) -> Vec<String> {
    let said = ends_of(steps, at).into_iter().map(|end| end.for_person().into_owned());
    match steps.get(at).and_then(gate_for_person) {
        Some(gate) => said.map(|merged| format!("{gate}, then {merged}")).collect(),
        None => said.collect(),
    }
}

/// What a step is entered on, which is what ends the one before it.
fn entry(step: &RailStep) -> Option<End> {
    if step.delivers {
        return Some(End::Pushed);
    }
    if step.walks {
        return Some(End::WalkShown);
    }
    step.fit.map(|fit| match fit {
        Fit::Edited => End::Edited,
        Fit::Checked => End::Checked,
        Fit::Committed => End::Committed,
        Fit::PrOpened => End::PrOpened,
    })
}

/// When a step is passed over instead of walked, for a step that can be.
pub(crate) fn passed_over_when(step: &RailStep) -> Option<&'static str> {
    step.walks.then_some("nothing would be seen")
}
