//! The forge's own tracker and link reader over GitHub's API, with `gh` standing behind each.
//!
//! **Per repository, as the rest of [`ApiForge`](super::ApiForge)**: a checkout whose `origin` is
//! not on `github.com`, or a call `gh` holds no token for, goes to the `gh` implementation whole.
//! Any other failure is said in the tracker's own words, as `gh`'s stderr was.

use std::sync::Arc;

use adapter_traits::{
    IssueAddress, LinkLookup, LookupCall, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker, TrackerChoice,
    Trackers,
};

use super::{issues, ApiForge};
use crate::forge_address::ForgeAddress;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::issue_lookup::issue_reference;
use crate::taught_tracker::TaughtTrackers;
use crate::{IssueLookup, IssueTracker};

/// The trackers a Manifest can choose, the forge's own read over the forge's API.
#[derive(Clone)]
pub struct ApiTrackers {
    forge: ApiForge,
    taught: Option<Arc<dyn TaughtTrackers>>,
}

impl ApiForge {
    /// The trackers that read through this forge's client, so Fleet holds one.
    pub fn trackers(&self) -> ApiTrackers {
        ApiTrackers { forge: self.clone(), taught: None }
    }
}

impl ApiTrackers {
    /// The same, with a provider no tracker here answers to looked up among `mods`.
    pub fn taught_by(self, mods: Arc<dyn TaughtTrackers>) -> ApiTrackers {
        ApiTrackers { taught: Some(mods), ..self }
    }
}

impl Trackers for ApiTrackers {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync> {
        crate::tracker_providers::tracker_over(choice, Some(&self.forge), self.taught.as_ref())
    }

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync> {
        crate::tracker_providers::links_over(choice, Some(&self.forge))
    }

    fn backlog(&self, choice: &TrackerChoice) -> Option<Arc<dyn adapter_traits::Backlog + Send + Sync>> {
        crate::tracker_providers::backlog_over(choice)
    }
}

/// [`IssueTracker`], over the API.
pub(crate) struct ApiTracker {
    forge: ApiForge,
}

impl ApiTracker {
    pub(crate) fn over(forge: ApiForge) -> ApiTracker {
        ApiTracker { forge }
    }
}

impl Tracker for ApiTracker {
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let Some(repository) = self.forge.on_github(in_repo) else { return IssueTracker.open_issues(in_repo) };
        match issues::open_issues(&self.forge.api, &repository) {
            Err(ApiFailure::NoToken(_)) => IssueTracker.open_issues(in_repo),
            read => read.map_err(|why| NotTracked::of("listing the open issues", why.to_string())),
        }
    }

    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        // `#12` or `12`; text that is not a number names no issue here, and never reaches the forge.
        let Ok(number) = reference.trim_start_matches('#').parse::<u64>() else { return Ok(None) };
        let Some(repository) = self.forge.on_github(in_repo) else { return IssueTracker.read_issue(in_repo, reference) };
        match issues::read_issue(&self.forge.api, &repository, number) {
            Err(ApiFailure::NoToken(_)) => IssueTracker.read_issue(in_repo, reference),
            Err(why) if issues::is_no_such_issue(&why) => Ok(None),
            read => read.map_err(|why| NotTracked::of("reading the issue", why.to_string())),
        }
    }

    fn search_issues(&self, in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let Some(repository) = self.forge.on_github(in_repo) else { return IssueTracker.search_issues(in_repo, query) };
        match issues::search_issues(&self.forge.api, &repository, query) {
            Err(ApiFailure::NoToken(_)) => IssueTracker.search_issues(in_repo, query),
            found => found.map_err(|why| NotTracked::of("searching the issues", why.to_string())),
        }
    }

    fn query_guide(&self) -> QueryGuide {
        crate::issue_tracker::forge_guide()
    }

    fn issues_linked_in(&self, in_repo: &str, text: &str) -> Vec<String> {
        crate::issue_tracker::forge_issues_linked_in(in_repo, text)
    }
}

/// [`IssueLookup`], with the issue read over the API instead of by a call Fleet runs.
pub(crate) struct ApiLinkLookup {
    api: GitHubApi,
}

impl ApiLinkLookup {
    pub(crate) fn over(forge: &ApiForge) -> ApiLinkLookup {
        ApiLinkLookup { api: forge.api.clone() }
    }
}

impl LinkLookup for ApiLinkLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall> {
        IssueLookup.resolve(request)
    }

    /// The title and body of the issue `request` links. A link is always into `github.com`, so no
    /// checkout decides where it goes; only a token `gh` does not hold sends it to `resolve`'s call.
    fn read_link(&self, request: &str) -> Option<Result<String, String>> {
        let (owner, repo, number) = issue_reference(request)?;
        let address = ForgeAddress::of_remote(&format!("https://github.com/{owner}/{repo}"))?;
        let repository = GitHubRepository::of(address).ok()?;
        let number: u64 = number.parse().ok()?;
        match issues::text(&self.api, &repository, number) {
            Err(ApiFailure::NoToken(_)) => None,
            Err(why) => Some(Err(why.to_string())),
            Ok(text) if text.is_empty() => Some(Err("the lookup said nothing".to_string())),
            Ok(text) => Some(Ok(text)),
        }
    }

    fn issue(&self, request: &str) -> Option<IssueAddress> {
        IssueLookup.issue(request)
    }

    fn edited(&self, issue: &IssueAddress) -> Option<LookupCall> {
        IssueLookup.edited(issue)
    }
}
