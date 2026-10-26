//! Saved issue queries and the query writer. `docs/concepts/tracker.md`, *Saved queries*.
//!
//! **A query is in the tracker's own language**, so nothing here reads one: it is text Fleet
//! passes to the repository's tracker, and the fields a row shows are that tracker's own names.

use serde::{Deserialize, Serialize};

use crate::{ManifestId, TrackedIssue};

/// `search_issues`: a query in the repository's tracker's own language, run once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIssues {
    pub query: String,
}

/// `install_tool`: the tool a refusal's `missing_tool` or `needs_setup` named. Installs it where it is absent, then
/// sets it up in the repository's main checkout. Idempotent. Only the tools Fleet knows how to
/// install are taken; any other is refused, so no command crosses the wire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallTool {
    pub tool: String,
}

/// `install_tool`'s answer: whether the tool is now installed, and why not where it is not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledTool {
    pub tool: String,
    pub done: bool,
    /// What happened, in words: Homebrew's own where it failed, `Homebrew is not available` where
    /// there is none.
    pub said: String,
}

/// What a person needs of a tracker's query language to write or choose a query: its name, the two
/// built-in queries, and the fields a row can show. The prose a model reads is not sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueQueryGuide {
    /// As a person would say it: `JQL`.
    pub language: String,
    /// The open issues, in the tracker's own language.
    pub open: String,
    /// The open issues assigned to whoever the tracker's credentials are.
    pub assigned_to_me: String,
    /// The tracker's field names a row can show.
    pub fields: Vec<String>,
}

/// One query a person saved, on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedIssueQuery {
    pub id: String,
    pub name: String,
    /// What the person said, which the query was written from. Empty for one typed by hand.
    pub words: String,
    /// In the tracker's own language: shown to the person, who may edit it.
    pub query: String,
    /// The tracker's field names a row shows, in order.
    pub row_fields: Vec<String>,
}

/// `list_issue_queries`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueQueries {
    pub manifest_id: ManifestId,
    /// Absent where the tracker cannot be read, and then `refused` says why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide: Option<IssueQueryGuide>,
    /// Why the tracker cannot be read, in its own words: no tool, nobody signed in, a Manifest block
    /// it refuses. Bridge offers to teach Armada the tracker from this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// Oldest first. Listed whether or not the tracker can be read, so a person can still forget one.
    pub saved: Vec<SavedIssueQuery>,
}

/// `write_issue_query`: what the person wants listed, in their own words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteIssueQuery {
    pub words: String,
}

/// `write_issue_query`' answer: the query, already run. **Nothing is kept** until `save_issue_query`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrittenIssueQuery {
    pub query: String,
    pub row_fields: Vec<String>,
    /// What the tracker answered the query, in `list_issues`' shape.
    pub issues: Vec<TrackedIssue>,
    /// What the one model call cost, in millionths of a dollar. Absent where the call ended without saying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `save_issue_query`: a query to keep. **With an `id` it edits that one**, which is how a hand
/// edit of the text lands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveIssueQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub words: String,
    pub query: String,
    pub row_fields: Vec<String>,
}
