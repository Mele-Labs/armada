//! What a person writes on a pull request over GitHub's API: a comment, a verdict, a review
//! with comments, and a conversation resolved.
//!
//! **A review with comments is the same five mutations** `crate::pull_request_review` makes
//! through `gh`, taken in its order by its `post_with`, so a refusal part-way deletes the pending
//! review there as here. Only how a step is taken differs.

use std::collections::BTreeMap;

use adapter_traits::PullRequestWrite;
use serde::{Deserialize, Serialize};

use super::writes::{node, sentence};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::pull_request_review::{post_with, Step};
use crate::pull_request_threads::{RESOLVE, UNRESOLVE};

#[derive(Serialize)]
struct Comment<'a> {
    body: &'a str,
}

#[derive(Serialize)]
struct Verdict<'a> {
    event: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

/// A step's values: text, or the one line number.
#[derive(Serialize)]
#[serde(untagged)]
enum Value<'a> {
    Text(&'a str),
    Line(u64),
}

/// A mutation's answer, whichever it is: the id of what it made sits one level down.
#[derive(Deserialize)]
struct Made {
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answered {
    pull_request_review: Option<Made>,
    thread: Option<Made>,
    comment: Option<Made>,
}

/// [`Forge::write_pull_request`](adapter_traits::Forge::write_pull_request).
pub(super) fn write(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
    pull_request: &str,
    write: &PullRequestWrite,
) -> Result<Result<(), String>, ApiFailure> {
    if let PullRequestWrite::Review { verdict, body, comments, commit } = write {
        let pull = match node(api, repository, number)? {
            Ok(pull) => pull,
            Err(said) => return Ok(Err(said)),
        };
        let mut step = |step: &Step| taken(api, step);
        return Ok(post_with(
            &mut step,
            (&pull.id, &pull.head_ref_oid),
            *verdict,
            body.as_deref(),
            comments,
            commit.as_deref(),
        ));
    }
    // The blank-body rules, and their sentences, are the ones `gh` is held to.
    let (_, body) = match crate::pull_request_writes::arguments(pull_request, write) {
        Ok(it) => it,
        Err(said) => return Ok(Err(said)),
    };
    let path = repository.path();
    match write {
        PullRequestWrite::Comment { .. } => {
            api.post(&format!("{path}/issues/{number}/comments"), &Comment { body: body.unwrap_or("") })?;
        }
        PullRequestWrite::Approve { .. } => verdict(api, &path, number, "APPROVE", body)?,
        PullRequestWrite::RequestChanges { .. } => verdict(api, &path, number, "REQUEST_CHANGES", body)?,
        PullRequestWrite::Review { .. } => {}
    }
    Ok(Ok(()))
}

fn verdict(api: &GitHubApi, path: &str, number: u64, event: &str, body: Option<&str>) -> Result<(), ApiFailure> {
    api.post(&format!("{path}/pulls/{number}/reviews"), &Verdict { event, body }).map(drop)
}

/// A step taken over the API: the id it answered with, or the forge's sentence.
fn taken(api: &GitHubApi, step: &Step) -> Result<String, String> {
    let mut values: BTreeMap<&str, Value<'_>> = BTreeMap::new();
    for (name, value) in &step.strings {
        values.insert(name, Value::Text(value));
    }
    if let Some(line) = step.line {
        values.insert("line", Value::Line(line));
    }
    if let Some(text) = &step.text {
        values.insert("body", Value::Text(text));
    }
    let answered: BTreeMap<String, Answered> = api.graphql(step.query, &values).map_err(sentence)?;
    Ok(answered
        .into_values()
        .find_map(|it| it.pull_request_review.or(it.thread).or(it.comment))
        .and_then(|it| it.id)
        .unwrap_or_default())
}

#[derive(Serialize)]
struct Thread<'a> {
    thread: &'a str,
}

/// What a resolve mutation answers: the thread, as the forge now holds it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Held {
    thread: Option<IsResolved>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsResolved {
    is_resolved: Option<bool>,
}

/// [`Forge::resolve_review_thread`](adapter_traits::Forge::resolve_review_thread).
pub(super) fn resolve(api: &GitHubApi, thread: &str, resolved: bool) -> Result<Result<bool, String>, ApiFailure> {
    let query = if resolved { RESOLVE } else { UNRESOLVE };
    let answered: BTreeMap<String, Held> = api.graphql(query, &Thread { thread })?;
    Ok(answered
        .into_values()
        .find_map(|it| it.thread)
        .and_then(|it| it.is_resolved)
        .ok_or_else(|| "the forge did not say whether the thread is resolved".to_string()))
}
