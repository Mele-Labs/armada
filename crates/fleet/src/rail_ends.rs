//! What ends a step, as a short fact, read off the step's anchor. **Pure**, and beside `rail`, whose
//! rules it restates for a person and for the agent; nothing here is a second way to place a step.

use crate::rail::{Fit, RailStep};

/// The fact that ends a step. One reading, said two ways: to the person on the workflow's page and to
/// the agent standing on the step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    Merged,
    PlanApproved,
    WalkApproved,
    Replied,
    PlanRecorded,
    Pushed,
    WalkShown,
    Edited,
    Checked,
    Committed,
    PrOpened,
}

impl End {
    /// As the workflow's page says it, to the person.
    pub(crate) fn for_person(self) -> &'static str {
        match self {
            End::Merged => "it is merged",
            End::PlanApproved => "you approve the plan",
            End::WalkApproved => "you approve the walk",
            End::Replied => "you reply",
            End::PlanRecorded => "a plan is recorded",
            End::Pushed => "it is pushed",
            End::WalkShown => "a walk is shown",
            End::Edited => "a file is edited",
            End::Checked => "a Check is run",
            End::Committed => "a commit is made",
            End::PrOpened => "a pull request is opened",
        }
    }

    /// As the agent on the step is told it, naming the tool where one does it.
    pub(crate) fn for_agent(self) -> &'static str {
        match self {
            End::Merged => "its pull request merges",
            End::PlanApproved => "the person approves the plan",
            End::WalkApproved => "the person approves the walk",
            End::Replied => "the person replies",
            End::PlanRecorded => "you record a plan with record_plan",
            End::Pushed => "you push the branch",
            End::WalkShown => "you show a walk with show_window",
            End::Edited => "you edit a file",
            End::Checked => "you run a Check with armada check",
            End::Committed => "you commit",
            End::PrOpened => "you open a pull request",
        }
    }

    /// The Job's own act ends the step, so nobody else will: Fleet does not move it, and the person
    /// has nothing to answer.
    pub(crate) fn by_the_job(self) -> bool {
        !matches!(self, End::Merged | End::PlanApproved | End::WalkApproved | End::Replied)
    }
}

/// What ends the step at `at`, or `None` where nothing observable does.
pub(crate) fn end_of(steps: &[RailStep], at: usize) -> Option<End> {
    let step = steps.get(at)?;
    if step.delivers {
        return Some(End::Merged);
    }
    if step.records_plan {
        return Some(End::PlanApproved);
    }
    if step.walks {
        return Some(End::WalkApproved);
    }
    if steps.iter().position(|one| one.records_plan).is_some_and(|recording| at < recording) {
        return Some(if step.awaits_reply { End::Replied } else { End::PlanRecorded });
    }
    steps.get(at + 1).and_then(entry)
}

/// What ends the step at `at`, to the person. Served by the workflow's page.
pub(crate) fn ends_when(steps: &[RailStep], at: usize) -> Option<&'static str> {
    end_of(steps, at).map(End::for_person)
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
