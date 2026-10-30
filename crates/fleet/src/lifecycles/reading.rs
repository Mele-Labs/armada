//! What each source says, in the words a lifecycle reads a state from.
//!
//! **Pure, and the same for every Job.** The tracker, the forge and the Job's record are read
//! elsewhere; here a reading becomes an [`Observed`], or nothing where the source was silent.
//! **Silence is never news**: a forge that would not answer leaves the instance where it stood.

use adapter_traits::{PullRequestPageFacts, PullRequestStanding, ReadIssue, ReviewStanding, Said};
use core_model::{FleetFact, ForgeFact, Lifecycle, Observed, StepId};

/// What a tracker calls an issue's state: the `Status` it names, else its own `Open` or `Closed`.
///
/// **The words of the tracker and not Armada's**, whatever the tracker spells them: Jira's Code
/// Review stays that, and the lifecycle that lists it maps it to In review.
pub fn tracker_word(issue: &ReadIssue) -> String {
    issue
        .fields
        .iter()
        .find(|field| field.name == "Status")
        .and_then(|field| field.values.first())
        .map(|value| value.as_written().trim().to_string())
        .filter(|word| !word.is_empty())
        .unwrap_or_else(|| String::from(if issue.closed { "Closed" } else { "Open" }))
}

/// Which fact a pull request is showing. `page` is read only for an open one, and `None` for it is
/// the forge's silence, so nothing is said of it.
///
/// **When several hold, the furthest along wins**, as an approved pull request still has its
/// reviewers on it: approved, else a reviewer has looked, else somebody is asked, else ready.
/// A reviewer is anybody but the author, and a comment from a bot counts as a look.
pub fn forge_fact(standing: PullRequestStanding, page: Option<&PullRequestPageFacts>) -> Option<ForgeFact> {
    match standing {
        PullRequestStanding::Draft => Some(ForgeFact::Draft),
        PullRequestStanding::Merged => Some(ForgeFact::Merged),
        PullRequestStanding::Closed => Some(ForgeFact::Closed),
        PullRequestStanding::Open => page.map(open_fact),
    }
}

fn open_fact(page: &PullRequestPageFacts) -> ForgeFact {
    let by_someone_else = |author: &str| page.author.as_deref() != Some(author);
    let looked = page.review == ReviewStanding::ChangesRequested
        || page.reviewers.iter().any(|reviewer| reviewer.verdict.is_some())
        || page.conversation.iter().any(|said| match said {
            Said::Comment { author, .. } | Said::Review { author, .. } => by_someone_else(author),
        });
    if page.review == ReviewStanding::Approved {
        ForgeFact::Approved
    } else if looked {
        ForgeFact::InReview
    } else if !page.reviewers.is_empty() {
        ForgeFact::ReviewersOn
    } else {
        ForgeFact::Ready
    }
}

/// What Fleet's own events say of a Job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JobSeen {
    /// The step it is in, where its workflow has one.
    pub step: Option<StepId>,
    pub opened_pull_request: bool,
    pub landed: bool,
    pub ended: bool,
}

/// The fact that stands for the Job among those `lifecycle` has a state for: landed, else ended,
/// else a pull request opened, else the step it is in, else started.
///
/// **A fact the lifecycle has no state for is not offered**, where a lifecycle that read the Job's
/// steps would otherwise be handed `ended` and find it unmapped. **A landed Job that closes
/// stays landed**, because landed outranks ended.
pub fn fleet_fact(lifecycle: &Lifecycle, seen: &JobSeen) -> Option<FleetFact> {
    let mut holding = Vec::new();
    for (holds, fact) in [
        (seen.landed, FleetFact::Landed),
        (seen.ended, FleetFact::Ended),
        (seen.opened_pull_request, FleetFact::PullRequestOpened),
    ] {
        if holds {
            holding.push(fact);
        }
    }
    holding.extend(seen.step.clone().map(FleetFact::StepEntered));
    holding.push(FleetFact::Started);
    holding
        .into_iter()
        .find(|fact| lifecycle.states().iter().any(|state| state.read().claims(&Observed::Fleet(fact.clone()))))
}
