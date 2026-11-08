//! What happened to a Job between two looks at it: a pull request opened or landed, the Job ending.
//! **Found by comparing**, because Fleet is told the Job's row changed and not what changed in it.
//!
//! **Steps are not found here.** The step the old rail derived slid backwards after a merge and moved
//! a merged Job's ticket back, so a step starting or ending, and a gate closing, are heard from the
//! gate rows instead ([`super::ruling`], `rules_gates_closed`). A look still has a `step` field for
//! rows kept before that, and nothing compares it.
//!
//! A look is what [`store::KeptSeen`] holds, so the comparison survives a restart: a Job whose pull
//! request was open before Fleet stopped has none opening after it comes back.

use store::KeptSeen;

/// One thing that happened to the Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Happened {
    PullRequestOpened,
    Landed,
    Ended,
}

/// What happened going from `before` to `now`, in the order it happened to the Job: the pull request,
/// then the end. **A thing that is no longer so is not an event**: a pull request given back opens
/// nothing.
pub fn happened_between(before: &KeptSeen, now: &KeptSeen) -> Vec<Happened> {
    let mut found = Vec::new();
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
    (a.pull_request_opened, a.landed, a.ended) == (b.pull_request_opened, b.landed, b.ended)
}
