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
    Backlog, IssueAddress, LinkLookup, LookupCall, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tool, ToolInstalled, Tracker,
    TrackerChoice, Trackers,
};

use crate::beads_tracker::{self, BeadsTracker};

use crate::jira::{self, JiraSettings};
use crate::taught_tracker::{Taught, TaughtTrackers};
use crate::jira_lookup::JiraLookup;
use crate::jira_tracker::{JiraTracker, Process};
use crate::api_forge::{ApiLinkLookup, ApiTracker};
use crate::{ApiForge, IssueLookup, IssueTracker};

/// The forge's own, written out; a block that names no provider means it too.
const FORGE_OWN: &str = "github";

pub struct TrackersByProvider;

enum Chosen {
    Forge,
    Jira(JiraSettings),
    Beads,
    /// A word this build has no tracker for, which a tracker mod may answer to.
    Taught { name: String, unknown: String },
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
        Some(beads_tracker::PROVIDER) if choice.settings().is_empty() => Chosen::Beads,
        Some(beads_tracker::PROVIDER) => Chosen::Refused(format!(
            "the {} tracker takes no keys beyond `provider`; found {}",
            beads_tracker::PROVIDER,
            choice.settings().keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        Some(other) if !choice.settings().is_empty() => Chosen::Refused(format!(
            "no tracker this build has is called `{other}`, and a tracker mod takes no keys beyond `provider`; found {}",
            choice.settings().keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        Some(other) => Chosen::Taught {
            name: other.to_string(),
            unknown: format!(
                "no tracker is called `{other}`; the ones this build has are {FORGE_OWN}, {} and {}, and no tracker mod is called that",
                jira::PROVIDER,
                beads_tracker::PROVIDER
            ),
        },
    }
}

impl Trackers for TrackersByProvider {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync> {
        tracker_over(choice, None, None)
    }

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync> {
        links_over(choice, None)
    }

    fn backlog(&self, choice: &TrackerChoice) -> Option<Arc<dyn Backlog + Send + Sync>> {
        backlog_over(choice)
    }

    fn install(&self, tool: Tool) -> ToolInstalled {
        crate::tool_install::installed(&Process, tool)
    }
}

/// What writes to the tracker `choice` names: beads alone, the one tracker Fleet writes to.
pub(crate) fn backlog_over(choice: &TrackerChoice) -> Option<Arc<dyn Backlog + Send + Sync>> {
    match chosen(choice) {
        Chosen::Beads => Some(Arc::new(BeadsTracker::over())),
        _ => None,
    }
}

/// The tracker `choice` names. The forge's own reads through `forge` where there is one, and
/// through `gh` where there is not. A word this build has no tracker for is looked up in `taught`,
/// the tracker mods, where there are any.
pub(crate) fn tracker_over(
    choice: &TrackerChoice,
    forge: Option<&ApiForge>,
    taught: Option<&Arc<dyn TaughtTrackers>>,
) -> Arc<dyn Tracker + Send + Sync> {
    match (chosen(choice), forge, taught) {
        (Chosen::Forge, Some(forge), _) => Arc::new(ApiTracker::over(forge.clone())),
        (Chosen::Forge, None, _) => Arc::new(IssueTracker),
        (Chosen::Jira(settings), _, _) => Arc::new(JiraTracker::over(settings)),
        (Chosen::Beads, _, _) => Arc::new(BeadsTracker::over()),
        (Chosen::Taught { name, unknown }, _, Some(mods)) => Arc::new(Taught { name, mods: Arc::clone(mods), unknown }),
        (Chosen::Taught { unknown, .. }, _, None) => Arc::new(Refusing { why: unknown }),
        (Chosen::Refused(why), _, _) => Arc::new(Refusing { why }),
    }
}

/// The link reader `choice` names, as [`tracker_over`] chooses a tracker.
pub(crate) fn links_over(choice: &TrackerChoice, forge: Option<&ApiForge>) -> Arc<dyn LinkLookup + Send + Sync> {
    match (chosen(choice), forge) {
        (Chosen::Forge, Some(forge)) => Arc::new(ApiLinkLookup::over(forge)),
        (Chosen::Forge, None) => Arc::new(IssueLookup),
        (Chosen::Jira(settings), _) => Arc::new(JiraLookup::over(settings)),
        // A taught tracker and beads read no link: a request naming one is dispatched as written.
        (Chosen::Beads | Chosen::Taught { .. } | Chosen::Refused(_), _) => Arc::new(Refusing { why: String::new() }),
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
