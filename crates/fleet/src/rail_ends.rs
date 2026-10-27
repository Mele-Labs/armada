//! What ends a step, as a short fact, read off the step's anchor. **Pure**, and beside `rail`, whose
//! rules it restates for a person; nothing here is a second way to place a step.

use crate::rail::{Fit, RailStep};

/// What ends the step at `at`, or `None` where nothing observable does. Served by the workflow's page,
/// which another route adds.
#[allow(dead_code)]
pub(crate) fn ends_when(steps: &[RailStep], at: usize) -> Option<&'static str> {
    let step = steps.get(at)?;
    if step.delivers {
        return Some("it is merged");
    }
    if step.records_plan {
        return Some("you approve the plan");
    }
    if step.walks {
        return Some("you approve the walk");
    }
    if steps.iter().position(|one| one.records_plan).is_some_and(|recording| at < recording) {
        return Some(if step.awaits_reply { "you reply" } else { "a plan is recorded" });
    }
    steps.get(at + 1).and_then(entry)
}

/// What a step is entered on, which is what ends the one before it.
fn entry(step: &RailStep) -> Option<&'static str> {
    if step.delivers {
        return Some("it is pushed");
    }
    if step.walks {
        return Some("a walk is shown");
    }
    step.fit.map(|fit| match fit {
        Fit::Edited => "a file is edited",
        Fit::Checked => "a Check is run",
        Fit::Committed => "a commit is made",
        Fit::PrOpened => "a pull request is opened",
    })
}

/// When a step is passed over instead of walked, for a step that can be.
#[allow(dead_code)]
pub(crate) fn passed_over_when(step: &RailStep) -> Option<&'static str> {
    step.walks.then_some("nothing would be seen")
}
