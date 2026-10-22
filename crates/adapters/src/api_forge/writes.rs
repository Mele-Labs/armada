//! What every write over GitHub's API shares: when it is the API's and when it is `gh`'s, how a
//! failure reads, and the one query that names a pull request's node and where it stands.
//!
//! **A write is never retried and never falls back after it was sent.** `gh` answers only a
//! call that holds no token ([`ApiFailure::NoToken`]), which fails before anything is sent. Any
//! other failure is the forge's sentence, as `gh`'s stderr was.

use serde::{Deserialize, Serialize};

use super::{numbered, ApiForge};
use crate::gh_forge::GhForge;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

impl ApiForge {
    /// The API's outcome where the repository is GitHub's and `gh` holds a token, else `gh`'s.
    /// `api` answers with the outcome the caller reads, or with why nothing could be asked.
    pub(crate) fn written<T, E>(
        &self,
        in_repo: &str,
        api: impl FnOnce(&GitHubApi, &GitHubRepository) -> Result<Result<T, E>, ApiFailure>,
        gh: impl FnOnce(&GhForge) -> Result<T, E>,
        refused: impl FnOnce(String) -> E,
    ) -> Result<T, E> {
        let Some(repository) = self.on_github(in_repo) else {
            return gh(&self.gh);
        };
        match api(&self.api, &repository) {
            Ok(outcome) => outcome,
            Err(ApiFailure::NoToken(_)) => gh(&self.gh),
            Err(failure) => Err(refused(sentence(failure))),
        }
    }

    /// [`written`](ApiForge::written) for a pull request named by number, which is all the API
    /// takes; anything else is `gh`'s.
    pub(crate) fn written_on_pull<T, E>(
        &self,
        in_repo: &str,
        pull_request: &str,
        api: impl FnOnce(&GitHubApi, &GitHubRepository, u64) -> Result<Result<T, E>, ApiFailure>,
        gh: impl FnOnce(&GhForge) -> Result<T, E>,
        refused: impl FnOnce(String) -> E,
    ) -> Result<T, E> {
        let Some(number) = numbered(pull_request) else { return gh(&self.gh) };
        self.written(in_repo, |api_, repository| api(api_, repository, number), gh, refused)
    }
}

/// The forge's own words: a query's messages as they were, anything else as the client says it.
pub(super) fn sentence(failure: ApiFailure) -> String {
    match failure {
        ApiFailure::GraphQl(messages) => messages.join("; "),
        other => other.to_string(),
    }
}

/// `owner` and `name`, which every GraphQL query about a repository takes.
pub(super) fn owner_and_name(repository: &GitHubRepository) -> (String, String) {
    let slug = repository.address().slug();
    let (owner, name) = slug.split_once('/').unwrap_or((&slug, ""));
    (owner.to_string(), name.to_string())
}

/// A body that says nothing: `{}`, for a `POST` that takes none.
#[derive(Serialize)]
pub(super) struct Nothing {}

const NODE: &str = "query($owner:String!,$name:String!,$number:Int!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    id state isDraft headRefOid mergeStateStatus isInMergeQueue isMergeQueueEnabled}}}";

#[derive(Serialize)]
struct Asked<'a> {
    owner: &'a str,
    name: &'a str,
    number: u64,
}

/// Where a pull request stands, as `gh` reads it before it writes.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Node {
    pub(super) id: String,
    #[serde(default)]
    pub(super) state: String,
    #[serde(default)]
    pub(super) is_draft: bool,
    #[serde(default)]
    pub(super) head_ref_oid: String,
    #[serde(default)]
    pub(super) merge_state_status: String,
    #[serde(default)]
    pub(super) is_in_merge_queue: bool,
    #[serde(default)]
    pub(super) is_merge_queue_enabled: bool,
}

#[derive(Deserialize)]
struct Found {
    repository: Option<InRepository>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InRepository {
    pull_request: Option<Node>,
}

/// The pull request's node, or the forge's sentence that there is none.
pub(super) fn node(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Result<Node, String>, ApiFailure> {
    let (owner, name) = owner_and_name(repository);
    let found: Found = api.graphql(NODE, &Asked { owner: &owner, name: &name, number })?;
    Ok(found
        .repository
        .and_then(|it| it.pull_request)
        .ok_or_else(|| format!("no pull request {number} in {owner}/{name}")))
}

/// `value` as one query value: everything but unreserved characters escaped.
pub(super) fn escaped(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(char::from(byte)),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
