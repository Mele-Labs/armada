//! What ends a step, as a short fact, read off the step's anchor. **Pure**, and beside `rail`, whose
//! rules it restates for a person and for the agent; nothing here is a second way to place a step.

use core_model::StepEnd;

use crate::rail::{Fit, RailStep};

/// The fact that ends a step. One reading, said two ways: to the person on the workflow's page and to
/// the agent standing on the step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl End {
    /// As the workflow's page says it, to the person.
    pub(crate) fn for_person(self) -> &'static str {
        match self {
            End::Merged => "it is merged",
            End::PlanApproved => "you approve the plan",
            End::WalkApproved => "you approve the walk",
            End::Replied => "you reply",
            End::AnyWalkApproved => "you approve a walk",
            End::SketchApproved => "you approve a sketch",
            End::DraftApproved => "you approve a draft",
            End::IdeaPicked => "you pick one of the ideas shown",
            End::PlanRecorded => "a plan is recorded",
            End::Pushed => "it is pushed",
            End::WalkShown => "a walk is shown",
            End::Edited => "a file is edited",
            End::Checked => "a Check is run",
            End::Committed => "a commit is made",
            End::PrOpened => "a pull request is opened",
            End::Undelivered => "it is delivered",
        }
    }

    /// As the agent on the step is told it, naming the tool where one does it.
    pub(crate) fn for_agent(self) -> &'static str {
        match self {
            End::Merged => "its pull request merges",
            End::PlanApproved => "the person approves the plan",
            End::WalkApproved => "the person approves the walk",
            End::Replied => "the person replies",
            End::AnyWalkApproved => "the person approves a walk you showed with show_window",
            End::SketchApproved => "the person approves a sketch you showed with show_sketch",
            End::DraftApproved => "the person approves a draft you proposed with propose_draft",
            End::IdeaPicked => "the person picks an option you gave a preview in AskUserQuestion",
            End::PlanRecorded => "you record a plan with record_plan",
            End::Pushed => "you push the branch",
            End::WalkShown => "you show a walk with show_window",
            End::Edited => "you edit a file",
            End::Checked => "you run a Check with armada check",
            End::Committed => "you commit",
            End::PrOpened => "you open a pull request",
            End::Undelivered => "you deliver the way the person chose",
        }
    }

    /// The Job's own act ends the step, so nobody else will: Fleet does not move it, and the person
    /// has nothing to answer.
    pub(crate) fn by_the_job(self) -> bool {
        !matches!(
            self,
            End::Merged
                | End::PlanApproved
                | End::WalkApproved
                | End::Replied
                | End::AnyWalkApproved
                | End::SketchApproved
                | End::DraftApproved
                | End::IdeaPicked
        )
    }

    fn listed(end: StepEnd) -> End {
        match end {
            StepEnd::Replied => End::Replied,
            StepEnd::WalkApproved => End::AnyWalkApproved,
            StepEnd::SketchApproved => End::SketchApproved,
            StepEnd::DraftApproved => End::DraftApproved,
            StepEnd::IdeaPicked => End::IdeaPicked,
        }
    }
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
    if steps.iter().position(|one| one.records_plan).is_some_and(|recording| at < recording) {
        return match step.ends_on.is_empty() {
            true => vec![End::PlanRecorded],
            false => step.ends_on.iter().copied().map(End::listed).collect(),
        };
    }
    steps.get(at + 1).and_then(entry).into_iter().collect()
}

/// What ends the step at `at` where one thing does, or `None` where several or nothing does.
pub(crate) fn end_of(steps: &[RailStep], at: usize) -> Option<End> {
    match ends_of(steps, at).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// What ends the step at `at`, to the person, where one thing does. Served by the workflow's page.
pub(crate) fn ends_when(steps: &[RailStep], at: usize) -> Option<&'static str> {
    end_of(steps, at).map(End::for_person)
}

/// Everything that ends the step at `at`, to the person, any one of them. For the workflow's page,
/// which serves it once `StepReading` carries `ends_any`.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn ends_any(steps: &[RailStep], at: usize) -> Vec<&'static str> {
    ends_of(steps, at).into_iter().map(End::for_person).collect()
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
