//! Where each Job's offered build stands against the running preview, as the Jobs list reads it: the same offers the
//! title bar and the In this build sheet read, matched to a Job by its id. A Job that offered nothing has no entry, so
//! a Job of a repository without a preview reads as it always has.

use std::collections::HashMap;

use ipc::{HeldOpenBy, PreviewOffers, PreviewState, PullRequestWait, SessionRecord};

/// One Job's newest offer against the running preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Built {
    /// The running preview holds it at its tip: he can try it now.
    In,
    /// Checked and ready, but the running preview does not hold it: Refresh preview brings it in.
    NotIn,
    /// Fleet is still merging it onto main and compiling it.
    Checking,
    /// Fleet's check failed it, with the first error or conflict where Fleet said one.
    Off(Option<String>),
}

impl Built {
    /// The row's second line while the Job stands Ready to try on this build.
    pub(super) fn said(&self) -> String {
        match self {
            Built::In => "In your preview: waits on your Looks good".into(),
            Built::NotIn => "Not in your preview yet: Refresh preview brings it in".into(),
            Built::Checking => "Checking that the build will work".into(),
            Built::Off(Some(why)) => format!("Off the preview list: {why}"),
            Built::Off(None) => "Off the preview list: Fleet's check failed it".into(),
        }
    }
}

/// Each Job's newest offer, by the Job's id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Builds(HashMap<String, Built>);

impl Builds {
    /// The offers Fleet last listed, oldest first, so a Job's newer offer stands over its older one.
    pub(super) fn of(offers: Option<&PreviewOffers>) -> Builds {
        let mut by_job = HashMap::new();
        for one in offers.map(|offers| offers.offers.as_slice()).unwrap_or_default() {
            let built = match one.state {
                _ if one.in_build => Built::In,
                PreviewState::Checking => Built::Checking,
                PreviewState::Failed => Built::Off(one.why.clone()),
                PreviewState::Ready => Built::NotIn,
            };
            by_job.insert(one.session_id.as_str().to_string(), built);
        }
        Builds(by_job)
    }

    pub(super) fn get(&self, job: &str) -> Option<&Built> {
        self.0.get(job)
    }
}

/// What keeps a Job open that is not him, said as the row's second line: its watch, a Job it started, its pull request.
/// None where nothing does. **The words the lead's board says** for the same fact (`leads::holds`).
pub(crate) fn held_said(job: &SessionRecord) -> Option<String> {
    Some(match job.held_open_by.first()? {
        HeldOpenBy::Watch { name } => format!("Waits on its watch {name}"),
        HeldOpenBy::Job { name } => format!("Waits on its Job {name}"),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::Queue } => format!("Pull request #{number} in the merge queue"),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::AutoMerge } => format!("Pull request #{number} merges once its Checks pass"),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::Checks { .. } } => format!("Pull request #{number} waits on its Checks"),
    })
}
