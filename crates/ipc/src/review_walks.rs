//! Natural Search and the Walk tool: the two pieces of the Reviews tab that are not about one
//! pull request's explanation. `docs/concepts/pull-review.md`, *Natural Search* and *Walk*.
//!
//! **Both ride the run engine** (`review_runs.rs`): a search is a run of kind `search`, kept under
//! [`SEARCH_RUN_SCOPE`] and the person's words, and a walk is a run of the tool `walk`, kept per
//! commit like any other. What is new here is what neither needs a run for: where a change runs
//! (found, remembered, edited) and holding a pulled-down change up while its walk is open.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::pull_reviews::PullReviewRow;

/// The `manifest_id` and `number` a search run is kept under: a search spans every repository
/// Fleet serves, so it belongs to none. The run's `head` is empty and its `name` is the person's
/// words, lower cased and single spaced.
pub const SEARCH_RUN_SCOPE: &str = "*";

/// `start_review_search`'s body. `POST /review_searches`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartReviewSearch {
    /// What the person wants, in their own words: "PRs that touch files I've touched".
    pub words: String,
    /// Run it again, whatever is kept. Without it a kept search of the same words is answered as it
    /// stands.
    #[serde(default)]
    pub again: bool,
}

/// `forget_review_search`'s body. `POST /review_searches/forget`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetReviewSearch {
    /// The words of the search to drop, as he asked them: Fleet reads them as a search is keyed.
    pub words: String,
}

/// One pull request a search turned up, and the one line that says why it matched. **Always an
/// open pull request of a served repository**: Fleet refuses a run whose answer names any other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewMatch {
    pub pull: PullReviewRow,
    pub reason: String,
}

/// How a change is reached to be walked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalkHow {
    /// Already running somewhere: a preview a deploy posted, or a staging address.
    Deployed,
    /// Pulled down into a slot of the repository's pool and started there with `command`.
    Pull,
}

/// Where a change runs. **At least one of `address` and `api` is set**; for `pull`, `command` is
/// too, and Fleet waits for `address` (else `api`) to answer before the walk opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkPlace {
    pub how: WalkHow,
    /// The web address, an http or https one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub address: String,
    /// What starts the change, run in the pulled-down checkout. Only for `pull`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    /// The API's base address, an http or https one, where the change has an API.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api: String,
}

/// Where Fleet looked for a place, in the person's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalkSource {
    /// The pull request itself: a deploy bot's preview link in its comments, or the details address
    /// of a check that deploys.
    PullRequest,
    /// What Fleet remembers of this repository and the area of code the pull request touches.
    Memory,
}

/// One source tried, and what it came to. Asking the person is the third source and is not a line
/// here: Bridge draws it when `place` is absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkTried {
    pub source: WalkSource,
    pub found: bool,
    /// The words: the bot's line or the check's name and address, a remembered place's own words,
    /// or what was not there.
    pub said: String,
    /// Who said it: the bot's login, the check's name, or the area a memory is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// `find_walk_place`: `GET /walk_finds/:repository/:number`. Reads, spends nothing, starts nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkPlaceFound {
    /// The two-deep directory with the most changed files (most changed lines on a tie), as
    /// `crates/fleet`: what a place is remembered for.
    pub area: String,
    /// Each source tried, in order, stopping at the first that found a place.
    pub tried: Vec<WalkTried>,
    /// The place the first source to find one gave. Absent: nothing was found, and the person is asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<WalkPlace>,
}

/// A place kept for one area of one repository, so the next pull request there does not ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedPlace {
    pub id: String,
    pub manifest_id: ManifestId,
    /// `owner/name`, as the forge names the repository.
    pub repository: String,
    /// A directory, two deep at most, with no trailing slash: `crates/fleet`.
    pub area: String,
    pub place: WalkPlace,
}

/// `list_walk_places`: every place kept, across the repositories Fleet serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedPlaces {
    pub places: Vec<RememberedPlace>,
}

/// `save_walk_place`'s body. `POST /walk_places/:repository`. Replaces the place kept for the area.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveWalkPlace {
    pub area: String,
    pub place: WalkPlace,
}

/// `open_review_walk`'s body. `POST /review_walks/:repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenReviewWalk {
    pub number: u64,
    pub place: WalkPlace,
}

/// How far a walk's place has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewWalkState {
    /// The pull request's head is being checked out in a slot. Only for `pull`.
    CheckingOut,
    /// The command runs and Fleet waits for the address to answer. Only for `pull`.
    Starting,
    /// The address answers. A `deployed` place is ready at once.
    Ready,
    /// It did not come up, and `failed` says why. The slot is already given back.
    Failed,
    /// Closed: the server is stopped and the slot given back.
    Closed,
}

/// A place held up for a walk, carried by `open_review_walk`, `close_review_walk` and
/// `review_walk.changed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewWalk {
    pub id: String,
    pub manifest_id: ManifestId,
    pub number: u64,
    pub state: ReviewWalkState,
    /// The slot a `pull` walk holds, until it is closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    /// What the walk window opens: the place's `address`, else its `api` base.
    pub address: String,
    /// The API's base address, where there is one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api: String,
    /// Why a failed walk failed, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
}

/// An HTTP method a walk's request may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReviewRequestMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// One request of an API walk, and what to look for in its response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequest {
    /// What the step says while it is on screen.
    pub say: String,
    pub method: ReviewRequestMethod,
    /// A path on the API's base address: it starts with `/` and names no host.
    pub path: String,
    /// The request body, JSON text, where the method takes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// What to look for in the response.
    pub look: String,
}
