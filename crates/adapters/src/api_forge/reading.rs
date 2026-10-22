//! The open pull requests as a pull reading reads them, over GitHub's API: the query `gh api graphql`
//! is given, its nodes folded by the function that folds `gh`'s lines.

use adapter_traits::{FollowedPull, FollowedPulls};
use core_model::Timestamp;
use serde::{Deserialize, Serialize};

use super::connection::Named;
use super::open_pulls::Repo;
use super::page::InRepository;
use super::{login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::pull_reading::{list_query, one_query, followed_built, FollowedPullRead, FollowedRead};

#[derive(Serialize)]
struct Asked<'a> {
    owner: &'a str,
    name: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    pull_requests: Option<Nodes<Pull>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Pull {
    state: Option<String>,
    number: Option<u64>,
    title: Option<String>,
    auto_merge_request: Option<serde::de::IgnoredAny>,
    head_ref_name: Option<String>,
    url: Option<String>,
    head_ref_oid: Option<String>,
    mergeable: Option<String>,
    author: Option<Login>,
    merge_queue_entry: Option<InQueue>,
    commits: Option<Nodes<RollupCommit>>,
}

#[derive(Deserialize)]
struct InQueue {
    state: Option<String>,
}

#[derive(Deserialize)]
struct RollupCommit {
    commit: Option<Committed>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Committed {
    status_check_rollup: Option<Rollup>,
}

#[derive(Deserialize)]
struct Rollup {
    contexts: Option<Nodes<Context>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Context {
    name: Option<String>,
    context: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    state: Option<String>,
    details_url: Option<String>,
    target_url: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

/// Every open pull request, the hundred last updated.
pub(super) fn read(api: &GitHubApi, repository: &GitHubRepository, gate: &str) -> Result<Option<FollowedPulls>, ApiFailure> {
    let named = Named::of(repository);
    let found: Repo<Listed> = api.graphql(&list_query(), &Asked { owner: named.owner(), name: named.name() })?;
    let pulls = found.repository.and_then(|it| it.pull_requests);
    Ok(Some(Nodes::all(pulls).filter_map(|pull| followed_pull_of(pull, gate)).collect()))
}

/// One pull request by its number. `Ok(None)` where it is not open.
pub(super) fn followed(
    api: &GitHubApi,
    repository: &GitHubRepository,
    gate: &str,
    number: u64,
) -> Result<Option<FollowedPull>, ApiFailure> {
    let found: InRepository<Pull> = api.graphql(&one_query(), &Named::of(repository).asked(number))?;
    Ok(found.pull().filter(|pull| pull.state.as_deref() == Some("OPEN")).and_then(|pull| followed_pull_of(pull, gate)))
}

/// A pull request as `crate::pull_reading::pull_of_line` makes one from `gh`'s line.
pub(crate) fn followed_pull_of(pull: Pull, gate: &str) -> Option<FollowedPull> {
    let contexts = Nodes::all(pull.commits).next().and_then(|it| it.commit?.status_check_rollup?.contexts);
    let checks = Nodes::all(contexts)
        .map(|check| FollowedRead {
            name: check.name.or(check.context).unwrap_or_default(),
            status: check.status.unwrap_or_default().to_ascii_lowercase(),
            conclusion: check.conclusion.or(check.state).unwrap_or_default().to_ascii_lowercase(),
            address: check.details_url.or(check.target_url).unwrap_or_default(),
            seconds: seconds_between(check.started_at.as_deref(), check.completed_at.as_deref()),
        })
        .collect();
    Some(followed_built(
        FollowedPullRead {
            number: pull.number?,
            branch: pull.head_ref_name.unwrap_or_default(),
            url: pull.url.unwrap_or_default(),
            author: login_of(pull.author),
            head: pull.head_ref_oid.unwrap_or_default(),
            mergeable: pull.mergeable.unwrap_or_default(),
            queue: pull.merge_queue_entry.and_then(|it| it.state).unwrap_or_default(),
            checks,
            title: pull.title.unwrap_or_default(),
            auto_merge: pull.auto_merge_request.is_some(),
        },
        gate,
    ))
}

/// How long a check ran, where both instants are known; a run that ended before it began ran 0.
fn seconds_between(started: Option<&str>, completed: Option<&str>) -> u64 {
    let at = |text: &str| Timestamp::from_rfc3339(text).epoch_millis();
    match (started.and_then(at), completed.and_then(at)) {
        (Some(started), Some(completed)) => u64::try_from((completed - started) / 1_000).unwrap_or(0),
        _ => 0,
    }
}
