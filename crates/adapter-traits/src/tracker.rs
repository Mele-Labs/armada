//! The issue tracker a repository keeps: what is open there, and one issue by
//! its reference. **Read only**: Fleet never comments on, labels or edits an
//! issue on the owner's behalf. The one tracker Fleet writes to is a [`Backlog`](crate::Backlog)
//! as well, and the writes are that trait's.
//!
//! **Its own trait, not more of [`Delivery`](crate::Delivery) or
//! [`LinkLookup`](crate::LinkLookup).** Delivery publishes a Job's branch and is
//! held as Fleet's version-control type; a tracker need not be the forge the
//! branch goes to. LinkLookup renders calls and runs none, which a listing
//! cannot keep to. `docs/concepts/tracker.md`.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::{FromOutside, LinkLookup};

/// One issue, as the tracker listed it: open where [`Tracker::open_issues`] listed it, either
/// where [`Tracker::search_issues`] did.
///
/// **Reference, title, address and state are always said.** The rest is said where the tracker
/// has it, and is empty where it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenIssue {
    /// How the tracker names it, as a person writes it: the forge's `#123`, Jira's `ABC-123`.
    /// **The identity of the issue in this crate**: nothing looks one up by a bare number.
    pub reference: String,
    pub title: FromOutside,
    /// The tracker's own address for it: what a dispatch names, and what
    /// [`IssueAddress::url`] holds once a Job came from it.
    pub url: FromOutside,
    pub labels: Vec<FromOutside>,
    /// Who it is assigned to, by login.
    pub assignees: Vec<FromOutside>,
    /// When it was last edited, RFC 3339, as the tracker wrote it.
    pub updated_at: String,
    /// The start of its body, whitespace folded. Never the whole body: the
    /// list is for choosing, and the proposer reads the issue itself.
    pub excerpt: FromOutside,
    /// The tracker says a pull request already closes it.
    pub linked_pull_request: bool,
    /// Who opened it. Empty where the tracker did not say, or the account is gone.
    pub author: FromOutside,
    /// The tracker no longer holds it open.
    pub closed: bool,
    /// Everything else the tracker said of it that a query may show, by the tracker's own names.
    pub fields: Vec<IssueField>,
}

/// One of an issue's own fields that Armada has no slot for, under the tracker's name for it.
/// Each value is a display string; a field with none is not listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueField {
    pub name: String,
    pub values: Vec<FromOutside>,
}

/// What a model needs to write a query in one tracker's own language, and the two queries
/// every list starts from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QueryGuide {
    /// The language's name, as a person would say it.
    pub language: String,
    /// Prose for a model: the syntax, its operators, examples, and what it cannot ask.
    pub guide: String,
    /// The open issues.
    pub open: String,
    /// The open issues assigned to whoever the tracker's credentials are, in the tracker's own
    /// word for them, so no identity has to be looked up.
    pub assigned_to_me: String,
    /// The [`IssueField::name`]s this tracker fills, so a query can choose its row's fields.
    pub fields: Vec<String>,
}

impl QueryGuide {
    /// Why the tracker cannot be queried, where it cannot: a tracker that refuses every call teaches
    /// no language and puts its reason where the guide would be.
    pub fn refusal(&self) -> Option<&str> {
        self.language.is_empty().then_some(self.guide.as_str())
    }
}

/// One issue read by its reference, open or closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadIssue {
    /// As [`OpenIssue::reference`].
    pub reference: String,
    pub title: FromOutside,
    pub url: FromOutside,
    /// The tracker no longer holds it open.
    pub closed: bool,
    /// The whole body, where [`OpenIssue::excerpt`] is its start.
    pub body: FromOutside,
    /// By login. Empty where the account is gone.
    pub author: FromOutside,
    /// RFC 3339, as the tracker wrote it.
    pub opened_at: String,
    pub labels: Vec<FromOutside>,
    pub assignees: Vec<FromOutside>,
    /// Oldest first.
    pub comments: Vec<IssueComment>,
    /// The pull requests that close it or name it, each once.
    pub linked_pull_requests: Vec<LinkedPullRequest>,
    /// As [`OpenIssue::fields`].
    pub fields: Vec<IssueField>,
}

impl ReadIssue {
    /// An issue as a tracker that reads only its title and state has it: nothing else said.
    pub fn bare(reference: String, title: FromOutside, url: FromOutside, closed: bool) -> ReadIssue {
        ReadIssue {
            reference,
            title,
            url,
            closed,
            body: FromOutside::verbatim(""),
            author: FromOutside::verbatim(""),
            opened_at: String::new(),
            labels: Vec::new(),
            assignees: Vec::new(),
            comments: Vec::new(),
            linked_pull_requests: Vec::new(),
            fields: Vec::new(),
        }
    }
}

/// One comment on an issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueComment {
    /// By login. Empty where the account is gone.
    pub author: FromOutside,
    /// RFC 3339, as the tracker wrote it.
    pub at: String,
    pub body: FromOutside,
}

/// A pull request that closes an issue or names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedPullRequest {
    pub number: u64,
    pub title: FromOutside,
    pub url: FromOutside,
    pub state: LinkedPullRequestState,
}

/// Where a linked pull request stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkedPullRequestState {
    Open,
    Merged,
    Closed,
}

/// Why the tracker did not do what was asked, in its own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotTracked {
    /// What was being attempted, as a noun: `listing the open issues`.
    pub doing: &'static str,
    pub said: String,
    /// The program that is not on this machine, where that is why: `bd`. Bridge offers to install
    /// it, so it is a name and not a sentence to be read back out of `said`.
    pub missing: Option<String>,
}

impl NotTracked {
    pub fn of(doing: &'static str, said: impl Into<String>) -> NotTracked {
        NotTracked {
            doing,
            said: said.into(),
            missing: None,
        }
    }

    /// `tool` is not installed, or not on the PATH Fleet runs with.
    pub fn lacking(doing: &'static str, tool: &str) -> NotTracked {
        NotTracked {
            doing,
            said: format!("`{tool}` is not installed, or not on the PATH Fleet runs with"),
            missing: Some(tool.to_string()),
        }
    }
}

/// A program Fleet installs when a person asks, and the only ones. **A closed set**: what the wire
/// names is looked up here and nothing else reaches a shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// The beads command line.
    Bd,
}

impl Tool {
    /// The tool a wire name means, `None` for any other.
    pub fn named(name: &str) -> Option<Tool> {
        match name {
            "bd" => Some(Tool::Bd),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tool::Bd => "bd",
        }
    }
}

/// How installing a [`Tool`] went, in words a person reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInstalled {
    pub done: bool,
    pub said: String,
}

/// Reading a repository's issues.
///
/// **No method writes.** Nothing comments on an issue, moves its labels, edits
/// its text or puts a closing line on a pull request; `the_tracker_trait_has_no_write_method`
/// in `adapters` holds the trait to it.
///
/// Every method is the operator's credentials, as with `Delivery`.
pub trait Tracker {
    /// Every open issue in the repository checked out at `in_repo`, newest
    /// edit first. Pull requests are not issues here.
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked>;

    /// The issue `reference` names, as [`OpenIssue::reference`] writes it, closed ones included.
    /// **A forge also takes the bare number**, since that is what a person types after `#`.
    /// `None` where there is none or the text names none, **and where it is a pull request's
    /// number**: a pull request is not an issue here, as it is not in
    /// [`open_issues`](Tracker::open_issues).
    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked>;

    /// The issues `query` matches, open or closed as it says, newest edit first and at most a
    /// hundred. **`query` is in the tracker's own language**, as [`query_guide`](Tracker::query_guide)
    /// teaches it, and runs as one call.
    fn search_issues(&self, in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked>;

    /// How to write a query [`search_issues`](Tracker::search_issues) takes.
    fn query_guide(&self) -> QueryGuide;

    /// The address an issue named like `example` opens at, with `{match}` where its reference goes:
    /// what turns a taught pattern into a link for this repository without asking anyone. `None`
    /// where this tracker does not know one, or `example` is not how it names an issue. **Built
    /// from the Manifest's own settings**, so it runs no process and reads no network.
    fn address_template(&self, example: &str) -> Option<String> {
        let _ = example;
        None
    }

    /// The issues of this tracker that `text` links, each as the ledger holds an issue (`7`, or a
    /// key such as `ABC-12`), once each and in the order first written. `in_repo` is the checkout
    /// the tracker is asked for. **None by default**: a tracker that reads no links names no issue,
    /// and a dispatch is still read.
    fn issues_linked_in(&self, in_repo: &str, text: &str) -> Vec<String> {
        let _ = (in_repo, text);
        Vec::new()
    }

    /// The issues a pull request names by key in its `title` or `branch`, title first, once each
    /// and in the order written: **how the tracker itself links a pull request to an issue**. Only
    /// the Manifest's own projects, so `UTF-8` names none. None by default.
    fn issues_named_by_pull_request(&self, title: &str, branch: &str) -> Vec<String> {
        let _ = (title, branch);
        Vec::new()
    }
}

/// Which tracker a repository's Manifest chose: the provider where it named
/// one, and every other key of the block as written.
///
/// **The provider is the only key this crate knows.** What the rest mean is the
/// implementation's to read and refuse, so a new tracker adds no field here.
/// `docs/contracts/adapters.md`, *Where adapter config lives*.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrackerChoice {
    provider: Option<String>,
    settings: BTreeMap<String, String>,
}

impl TrackerChoice {
    /// A Manifest that names no tracker: the forge the repository is on.
    pub fn of_the_forge() -> TrackerChoice {
        TrackerChoice::default()
    }

    pub fn named(provider: Option<&str>, settings: &BTreeMap<String, String>) -> TrackerChoice {
        TrackerChoice {
            provider: provider.map(String::from),
            settings: settings.clone(),
        }
    }

    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    pub fn settings(&self) -> &BTreeMap<String, String> {
        &self.settings
    }
}

/// The tracker and the link reader for a repository's choice. **Fleet holds one
/// of these and asks it per repository**, so no code above the composition root
/// learns which tracker is behind a Manifest.
///
/// **Total.** A choice that cannot be honoured, an unknown provider or a block
/// missing a key it needs, still answers: with a tracker that refuses every
/// call and says why. A bad block in one repository's file must not stop Fleet
/// starting or hide the others.
pub trait Trackers {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync>;

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync>;

    /// What writes to and syncs the tracker `choice` names, **where it is one Fleet writes to**.
    /// `None` for every other: a forge and Jira are read only.
    fn backlog(&self, choice: &TrackerChoice) -> Option<Arc<dyn crate::Backlog + Send + Sync>> {
        let _ = choice;
        None
    }

    /// Install `tool`. **Nothing here runs until a person asks**: a missing tool is reported, not
    /// repaired. Nothing is installed by default.
    fn install(&self, tool: Tool) -> ToolInstalled {
        ToolInstalled {
            done: false,
            said: format!("nothing here installs `{}`", tool.name()),
        }
    }
}
