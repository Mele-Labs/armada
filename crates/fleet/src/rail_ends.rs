//! What ends a step, as a short fact, read off the step's anchor. **Pure**, and beside `rail`, whose
//! rules it restates for a person and for the agent; nothing here is a second way to place a step.

use std::borrow::Cow;

use core_model::{LifecycleEnd, StepEnd, Waits};
use ipc::WaitingSource;

use crate::rail::{Fit, RailStep};

/// The fact that ends a step. One reading, said two ways: to the person on the workflow's page and to
/// the agent standing on the step.
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
    PlanRecorded,
    Pushed,
    WalkShown,
    Edited,
    Checked,
    Committed,
    PrOpened,
    /// A delivering step the Job has not delivered on yet: no pull request is open. Only `rail::stalled`
    /// reads it, as `end_of` names a delivering step's end as its merge.
    Undelivered,
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
            End::WalkApproved => "you approve the walk".into(),
            End::Replied => "you reply".into(),
            End::AnyWalkApproved => "you approve a walk".into(),
            End::SketchApproved => "you approve a sketch".into(),
            End::DraftApproved => "you approve a draft".into(),
            End::IdeaPicked => "you pick one of the ideas shown".into(),
            End::PlanRecorded => "a plan is recorded".into(),
            End::Pushed => "it is pushed".into(),
            End::WalkShown => "a walk is shown".into(),
            End::Edited => "a file is edited".into(),
            End::Checked => "a Check is run".into(),
            End::Committed => "a commit is made".into(),
            End::PrOpened => "a pull request is opened".into(),
            End::Undelivered => "it is delivered".into(),
        }
    }

    /// As the agent on the step is told it, naming the tool where one does it.
    pub(crate) fn for_agent(&self) -> Cow<'static, str> {
        match self {
            End::Lifecycle(_) => format!("{}, which Fleet watches for", self.for_person()).into(),
            End::Merged => "its pull request merges".into(),
            End::PlanApproved => "the person approves the plan".into(),
            End::WalkApproved => "the person approves the walk".into(),
            End::Replied => "the person replies".into(),
            End::AnyWalkApproved => "the person approves a walk you showed with show_window".into(),
            End::SketchApproved => "the person approves a sketch you showed with show_sketch".into(),
            End::DraftApproved => "the person approves a draft you proposed with propose_draft".into(),
            End::IdeaPicked => "the person picks an option you gave a preview in AskUserQuestion".into(),
            End::PlanRecorded => "you record a plan with record_plan".into(),
            End::Pushed => "you push the branch".into(),
            End::WalkShown => "you show a walk with show_window".into(),
            End::Edited => "you edit a file".into(),
            End::Checked => "you run a Check with armada check".into(),
            End::Committed => "you commit".into(),
            End::PrOpened => "you open a pull request".into(),
            End::Undelivered => "you deliver the way the person chose".into(),
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
            End::PlanRecorded
            | End::Pushed
            | End::WalkShown
            | End::Edited
            | End::Checked
            | End::Committed
            | End::PrOpened
            | End::Undelivered => Waiter::TheJob,
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
            StepEnd::Lifecycle(end) => End::Lifecycle(end.clone()),
        }
    }
}

/// What a **delivering** step that lists a lifecycle end tells the agent. The end does not end the step,
/// which the merge does: it gates the merge. `None` for any other step.
pub(crate) fn gate_said(step: &RailStep) -> Option<String> {
    let gates: Vec<_> = step.lifecycle_ends().map(|end| End::Lifecycle(end.clone()).for_person()).collect();
    if !step.delivers || gates.is_empty() {
        return None;
    }
    Some(format!(
        "Do not merge until {}. Open the pull request if you have not, then end your turn: Fleet watches for it and \
         wakes you with a message, and then you land it.",
        gates.join(", or ")
    ))
}

/// Everything that ends the step at `at`, any one of them, or none where nothing observable does. A
/// step before the recording step that lists its ends is ended by those; the plan recorded is said
/// only where it lists none.
pub(crate) fn ends_of(steps: &[RailStep], at: usize) -> Vec<End> {
    let Some(step) = steps.get(at) else { return Vec::new() };
    if step.delivers {
        return vec![End::Merged];
    }
    if step.records_plan {
        return vec![End::PlanApproved];
    }
    if step.walks {
        return vec![End::WalkApproved];
    }
    if step.has_lifecycle_end() {
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

/// Everything that ends the step at `at`, to the person, any one of them. For the workflow's page,
/// which serves it once `StepReading` carries `ends_any`.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn ends_any(steps: &[RailStep], at: usize) -> Vec<String> {
    ends_of(steps, at).iter().map(|end| end.for_person().into_owned()).collect()
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
