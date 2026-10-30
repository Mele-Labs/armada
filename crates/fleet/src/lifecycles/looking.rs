//! What happened to a Job between two looks at it: a step ending and the next starting, a pull
//! request opened or landed, the Job ending. **Found by comparing**, because Fleet is told the
//! Job's row changed and not what changed in it.
//!
//! A look is what [`store::KeptSeen`] holds, so the comparison survives a restart: a Job at the same
//! step after Fleet comes back has no step starting.

use store::KeptSeen;

/// One thing that happened to the Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Happened {
    StepEnded(String),
    StepStarted(String),
    PullRequestOpened,
    Landed,
    Ended,
}

/// What happened going from `before` to `now`, in the order it happened to the Job: the step it
/// left, the step it entered, then the pull request, then the end. **A thing that is no longer so
/// is not an event**: a pull request given back opens nothing.
pub fn happened_between(before: &KeptSeen, now: &KeptSeen) -> Vec<Happened> {
    let mut found = Vec::new();
    if before.step != now.step {
        found.extend(before.step.clone().map(Happened::StepEnded));
        found.extend(now.step.clone().map(Happened::StepStarted));
    }
    if now.pull_request_opened && !before.pull_request_opened {
        found.push(Happened::PullRequestOpened);
    }
    if now.landed && !before.landed {
        found.push(Happened::Landed);
    }
    if now.ended && !before.ended {
        found.push(Happened::Ended);
    }
    found
}

/// Whether the two looks show the same Job, whatever number of events came between.
pub fn same_look(a: &KeptSeen, b: &KeptSeen) -> bool {
    (&a.step, a.pull_request_opened, a.landed, a.ended) == (&b.step, b.pull_request_opened, b.landed, b.ended)
}
