//! Which tracker a Manifest's `tracker:` block names. #91,
//! `docs/concepts/tracker.md`, *Choosing a tracker*.
//!
//! **Here and not in `armada`** because the provider words are vendors', and
//! `armada` may not spell one. The composition root still names this and no
//! other implementation, which is what `docs/contracts/adapters.md` asks of it.
//!
//! **Total.** A provider this build does not have, or a block its provider
//! refuses, answers with a tracker that says why on every call, so one
//! repository's mistake never stops Fleet or touches another repository.

use std::sync::Arc;

use adapter_traits::{
    IssueAddress, LinkLookup, LookupCall, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker,
    TrackerChoice, Trackers,
};

use crate::jira::{self, JiraSettings};
use crate::jira_lookup::JiraLookup;
use crate::jira_tracker::JiraTracker;
use crate::api_forge::{ApiLinkLookup, ApiTracker};
use crate::{ApiForge, IssueLookup, IssueTracker};

/// The forge's own, written out; a block that names no provider means it too.
const FORGE_OWN: &str = "github";

pub struct TrackersByProvider;

enum Chosen {
    Forge,
    Jira(JiraSettings),
    Refused(String),
}

fn chosen(choice: &TrackerChoice) -> Chosen {
    let provider = choice.provider().map(str::to_ascii_lowercase);
    match provider.as_deref() {
        None | Some(FORGE_OWN) if choice.settings().is_empty() => Chosen::Forge,
        None => Chosen::Refused("`tracker` has keys and no `provider`".to_string()),
        Some(FORGE_OWN) => Chosen::Refused(format!(
            "the {FORGE_OWN} tracker takes no keys beyond `provider`; found {}",
            choice.settings().keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        Some(jira::PROVIDER) => match JiraSettings::read(choice.settings()) {
            Ok(settings) => Chosen::Jira(settings),
            Err(why) => Chosen::Refused(why),
        },
        Some(other) => Chosen::Refused(format!(
            "no tracker is called `{other}`; the ones this build has are {FORGE_OWN} and {}",
            jira::PROVIDER
        )),
    }
}

impl Trackers for TrackersByProvider {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync> {
        tracker_over(choice, None)
    }

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync> {
        links_over(choice, None)
    }
}

/// The tracker `choice` names. The forge's own reads through `forge` where there is one, and
/// through `gh` where there is not.
pub(crate) fn tracker_over(choice: &TrackerChoice, forge: Option<&ApiForge>) -> Arc<dyn Tracker + Send + Sync> {
    match (chosen(choice), forge) {
        (Chosen::Forge, Some(forge)) => Arc::new(ApiTracker::over(forge.clone())),
        (Chosen::Forge, None) => Arc::new(IssueTracker),
        (Chosen::Jira(settings), _) => Arc::new(JiraTracker::over(settings)),
        (Chosen::Refused(why), _) => Arc::new(Refusing { why }),
    }
}

/// The link reader `choice` names, as [`tracker_over`] chooses a tracker.
pub(crate) fn links_over(choice: &TrackerChoice, forge: Option<&ApiForge>) -> Arc<dyn LinkLookup + Send + Sync> {
    match (chosen(choice), forge) {
        (Chosen::Forge, Some(forge)) => Arc::new(ApiLinkLookup::over(forge)),
        (Chosen::Forge, None) => Arc::new(IssueLookup),
        (Chosen::Jira(settings), _) => Arc::new(JiraLookup::over(settings)),
        (Chosen::Refused(_), _) => Arc::new(Refusing { why: String::new() }),
    }
}

/// A tracker that could not be built, standing where it would have been.
pub(crate) struct Refusing {
    why: String,
}

impl Refusing {
    fn no(&self, doing: &'static str) -> NotTracked {
        NotTracked::of(doing, self.why.clone())
    }
}

impl Tracker for Refusing {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        Err(self.no("listing the open issues"))
    }

    fn read_issue(&self, _in_repo: &str, _reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        Err(self.no("reading the issue"))
    }

    fn search_issues(&self, _in_repo: &str, _query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        Err(self.no("searching the issues"))
    }

    /// No language to teach: the guide is why every call is refused.
    fn query_guide(&self) -> QueryGuide {
        QueryGuide { guide: self.why.clone(), ..QueryGuide::default() }
    }
}

/// With no tracker there is no link to read: a request is dispatched as written.
impl LinkLookup for Refusing {
    fn resolve(&self, _request: &str) -> Option<LookupCall> {
        None
    }

    fn issue(&self, _request: &str) -> Option<IssueAddress> {
        None
    }

    fn edited(&self, _issue: &IssueAddress) -> Option<LookupCall> {
        None
    }
}
