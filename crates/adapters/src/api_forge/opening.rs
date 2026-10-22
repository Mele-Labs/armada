//! Opening a pull request and filing an issue over GitHub's API.
//!
//! **Asked before it is created**, as `gh` does: a second pull request for a branch is refused
//! with a sentence rather than answered with the first, so an open one is looked for by its
//! branch. A look that fails is not an answer, and creating goes on, as it does with `gh`.

use adapter_traits::{FiledIssue, NotDelivered, NotFiled, Opened, Review};
use serde::{Deserialize, Serialize};

use super::writes::escaped;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

#[derive(Serialize)]
struct NewPull<'a> {
    title: &'a str,
    body: &'a str,
    head: &'a str,
    base: &'a str,
    draft: bool,
}

#[derive(Serialize)]
struct NewIssue<'a> {
    title: &'a str,
    body: &'a str,
}

/// A pull request or an issue, down to the address.
#[derive(Deserialize)]
struct Address {
    html_url: Option<String>,
}

const NO_ADDRESS: &str = "the forge made the pull request but gave no address for it";

/// An open pull request for `branch`, if there is one. Any failure but a missing token is none.
fn already_open(api: &GitHubApi, repository: &GitHubRepository, branch: &str) -> Result<Option<String>, ApiFailure> {
    let slug = repository.address().slug();
    let owner = slug.split_once('/').map_or(slug.as_str(), |(owner, _)| owner);
    let head = escaped(&format!("{owner}:{branch}"));
    let listed = api
        .get(&format!("{}/pulls?head={head}&state=open", repository.path()))
        .and_then(|answer| answer.decode::<Vec<Address>>("a list of pull requests"));
    match listed {
        Ok(pulls) => Ok(pulls.into_iter().find_map(|pull| pull.html_url)),
        Err(missing @ ApiFailure::NoToken(_)) => Err(missing),
        Err(_) => Ok(None),
    }
}

fn created(
    api: &GitHubApi,
    repository: &GitHubRepository,
    new: &NewPull<'_>,
) -> Result<Result<String, NotDelivered>, ApiFailure> {
    let answer = api.post(&format!("{}/pulls", repository.path()), new)?;
    let made: Address = answer.decode("a pull request")?;
    Ok(made.html_url.ok_or_else(|| NotDelivered::of("the pull request", NO_ADDRESS)))
}

/// [`Forge::open_for_review`](adapter_traits::Forge::open_for_review).
pub(super) fn open(
    api: &GitHubApi,
    repository: &GitHubRepository,
    branch: &str,
    base: &str,
    review: &Review,
) -> Result<Result<Opened, NotDelivered>, ApiFailure> {
    if let Some(url) = already_open(api, repository, branch)? {
        return Ok(Ok(Opened::AlreadyOpen { url }));
    }
    let new = NewPull { title: review.title(), body: review.body(), head: branch, base, draft: review.draft() };
    Ok(created(api, repository, &new)?.map(|url| Opened::PullRequest { url }))
}

/// [`Forge::open_branch_for_review`](adapter_traits::Forge::open_branch_for_review): the same
/// pull request with no look for one first, as it is today.
pub(super) fn open_branch(
    api: &GitHubApi,
    repository: &GitHubRepository,
    base: &str,
    branch: &str,
    title: &str,
    body: &str,
) -> Result<Result<String, NotDelivered>, ApiFailure> {
    created(api, repository, &NewPull { title, body, head: branch, base, draft: false })
}

/// [`Forge::file_issue`](adapter_traits::Forge::file_issue).
pub(super) fn file(
    api: &GitHubApi,
    repository: &GitHubRepository,
    title: &str,
    body: &str,
) -> Result<Result<FiledIssue, NotFiled>, ApiFailure> {
    let answer = api.post(&format!("{}/issues", repository.path()), &NewIssue { title, body })?;
    let made: Address = answer.decode("an issue")?;
    Ok(made.html_url.map(|url| FiledIssue { url }).ok_or_else(|| NotFiled {
        said: "the forge filed the issue but printed no address for it".to_string(),
    }))
}
