//! What an agent says about one pull request, and the review tools it is asked with.
//! `docs/concepts/pull-review.md`, *Agent runs*.
//!
//! **A run is read-only and costs tokens, so it starts only from a press** and never because a
//! pull request was listed. It answers at once with a [`ReviewRun`] that is still going, and
//! each change of state arrives as `review_run.changed` carrying the run whole. Nothing streams:
//! a run is `queued`, `running`, then `done` or `failed`, so the event stream sees three small
//! events a run and no partial text.
//!
//! **A result is kept per commit.** It is keyed by repository, pull request, head commit, kind
//! and name, so asking for a kept one runs nothing, and a new head leaves the old result served
//! with the commit it ran on, which is how a reader marks it out of date.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::pull_reviews::DiffSide;
use crate::review_walks::{ReviewMatch, ReviewRequest, WalkPlace};
use crate::walk_steps::WalkStep;

/// Where a review tool was read from. Kit's replaces the repository's of the same id, and either
/// replaces Armada's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewToolSource {
    /// Carried in the binary.
    Armada,
    /// `~/.armada/review-tools/`, every repository on the machine.
    Kit,
    /// `.armada/review-tools/`, everyone on the repository.
    Repository,
}

/// Where a save writes. Armada's own are carried in the binary and are not a place to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewToolScope {
    Kit,
    Repository,
}

/// What a definition is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewToolKind {
    /// Finds things, each pinned to a place in the change.
    Tool,
    /// Tells a person one thing about the pull request.
    Explanation,
}

/// One review tool or explanation, resolved: the most specific place that defines its id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewTool {
    /// The name lower cased with its words joined by a dash, which is also the file's stem.
    pub id: String,
    pub kind: ReviewToolKind,
    pub name: String,
    /// What it checks, or for an explanation what it explains. The shape of the answer is
    /// Fleet's: a tool answers findings, and the four explanations Armada carries answer the
    /// shape their id has, and any other explanation answers text.
    pub prompt: String,
    pub source: ReviewToolSource,
    /// The place a definition of the same id was replaced from, where this one replaced one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<ReviewToolSource>,
    /// `repository` where this is Kit's and hides a repository definition of the same id that says
    /// something else, so Bridge can mark it. Absent where the two are the same, or nothing is
    /// hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hides: Option<ReviewToolSource>,
}

/// A file Fleet read and left out, and why. **Never fatal**: the others stand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewToolLeftOut {
    /// Absolute.
    pub file: String,
    pub source: ReviewToolSource,
    pub said: String,
}

/// `list_review_tools`: the definitions a repository has, resolved Kit over the repository over
/// Armada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewTools {
    /// Armada's first, in the order it carries them, then the others as they were added.
    pub tools: Vec<ReviewTool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_out: Vec<ReviewToolLeftOut>,
}

/// `save_review_tool`'s body. `POST /review_tools/:repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveReviewTool {
    pub scope: ReviewToolScope,
    pub kind: ReviewToolKind,
    pub name: String,
    pub prompt: String,
    /// Replace the file already holding this id in `scope`. Without it that is a 422.
    #[serde(default)]
    pub overwrite: bool,
}

/// What a run is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRunKind {
    /// A review tool: answers findings.
    Tool,
    /// An explanation.
    Explanation,
    /// A question in the person's own words: answers text.
    Ask,
    /// Natural Search: the person's words, put to every open pull request of the repositories
    /// Fleet serves. Answers matches. Kept under [`crate::SEARCH_RUN_SCOPE`], not under a pull request.
    Search,
}

/// How far a run has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRunState {
    /// Waiting for a bay to read the pull request in.
    Queued,
    Running,
    Done,
    /// It ended without a result that held up, and `failed` says why. **Never a result.**
    Failed,
}

/// `start_review_run`'s body. `POST /review_runs/:repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartReviewRun {
    pub number: u64,
    pub kind: ReviewRunKind,
    /// The id of the tool or explanation, or for [`ReviewRunKind::Ask`] the question's words.
    /// Words are kept under their lower-cased, single-spaced form.
    pub name: String,
    /// Run it again, whatever is kept. Without it a kept result for this pull request is
    /// answered as it stands, and only a pull request nothing was kept for is run.
    #[serde(default)]
    pub again: bool,
    /// Where the change runs, for the tool `walk` and for nothing else. **Required for it**: a walk is
    /// written against a place, and Fleet does not guess one (a 422 `fleet.review_walk_needs_a_place`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<WalkPlace>,
}

/// A place in the change: a range of lines of a file, on one side of the diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPlace {
    /// A file the pull request changes.
    pub path: String,
    /// `right` is the head's numbering, `left` the base's, as the forge names a line.
    pub side: DiffSide,
    pub start: u64,
    /// At least `start`.
    pub end: u64,
}

/// One thing done, in the order it was done, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewChange {
    pub title: String,
    pub why: String,
    /// The hunk that is the evidence for it.
    pub at: ReviewPlace,
}

/// A set of changes that make one thing happen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewTheme {
    pub title: String,
    pub purpose: String,
    pub changes: Vec<ReviewChange>,
}

/// One thing a person sees or can do differently.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewImpact {
    pub what: String,
    pub before: String,
    pub after: String,
}

/// Where a step of How to Test came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case")]
pub enum ReviewStepSource {
    /// The author's own directions in the description, which come first.
    Description,
    /// A README or an `AGENTS.md` agent file found up the tree from a
    /// changed path. Fleet checks that the file exists at the pull request's head.
    Doc { path: String },
}

/// One step of How to Test.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewStep {
    /// What to do.
    pub action: String,
    /// What to look for.
    pub look: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub source: ReviewStepSource,
}

/// An issue or page the pull request names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLink {
    pub label: String,
    /// An http or https address.
    pub address: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSeverity {
    Low,
    Medium,
    High,
}

/// One thing a review tool found, pinned to a change. **Armada's own note: never posted.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub title: String,
    /// Markdown.
    pub body: String,
    pub severity: ReviewSeverity,
    pub at: ReviewPlace,
}

/// What a run came to, in the shape its kind has. **Validated before it is kept**: an answer
/// that does not parse into one of these, or names a file the pull request does not change, is
/// a failed run with its reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ReviewResult {
    /// How it works: its changes grouped into themes, in the order they were made.
    Themes { summary: String, themes: Vec<ReviewTheme> },
    /// User Impact: before and after.
    Impact { rows: Vec<ReviewImpact> },
    /// How to Test.
    Steps { steps: Vec<ReviewStep> },
    /// Why.
    Reason {
        problem: String,
        cost: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        links: Vec<ReviewLink>,
    },
    /// An answer to a question, or an explanation nobody gave a shape: Markdown.
    Text { body: String },
    /// A review tool's findings. None is an answer: the tool read the change and found nothing.
    Findings { findings: Vec<ReviewFinding> },
    /// Natural Search: the open pull requests that matched, each with why. None is an answer.
    Matches { matches: Vec<ReviewMatch> },
    /// The tool `walk` for a web change: `show_window` steps over the page at `place`. `place` is
    /// the one the walk was written against, filled in by Fleet and never by the agent.
    WebWalk { place: WalkPlace, steps: Vec<WalkStep> },
    /// The tool `walk` for an API: one request a step, what to look for in each response.
    ApiWalk { place: WalkPlace, requests: Vec<ReviewRequest> },
}

/// One run about one pull request at one commit, going or kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRun {
    pub manifest_id: ManifestId,
    pub number: u64,
    pub kind: ReviewRunKind,
    /// The tool's or explanation's id, or the question's words.
    pub name: String,
    /// The commit it ran on, whole. A reader that holds a newer head marks the run out of date.
    pub head: String,
    pub state: ReviewRunState,
    /// RFC 3339.
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Why a failed run failed, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ReviewResult>,
    /// What the vendor billed, in millionths of a dollar. Absent where the call ended without
    /// saying, which is not a cost of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `get_review_runs`: every run kept or going for one pull request, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRuns {
    pub runs: Vec<ReviewRun>,
}

/// The Session of the person's that holds a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullReviewHolder {
    pub id: String,
    pub title: String,
}
