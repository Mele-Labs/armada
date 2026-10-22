//! What the forge says of one pull request in a single query each: its facts, and what became of
//! it. The same fields `gh pr view` is asked for, folded by the same functions.

use adapter_traits::{LandingOnTheForge, PullRequestFacts};
use serde::Deserialize;

use super::connection::Named;
use super::Nodes;
use super::page::InRepository;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::landing::{landing_of, unknown, LandingRead};
use crate::pull_request_facts::{facts_of, FactsRead};

/// The closing references are read a hundred at a time, as `gh` reads them.
const FACTS: &str = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){\
    pullRequest(number:$number){state isDraft autoMergeRequest{enabledAt} headRefName url title body \
    closingIssuesReferences(first:100){nodes{number}}}}}";

const LANDING: &str = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){\
    pullRequest(number:$number){number title state mergeable baseRefName baseRefOid headRefOid mergedAt}}}";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Facts {
    state: Option<String>,
    is_draft: Option<bool>,
    auto_merge_request: Option<serde::de::IgnoredAny>,
    head_ref_name: Option<String>,
    url: Option<String>,
    title: Option<String>,
    body: Option<String>,
    closing_issues_references: Option<Nodes<Closing>>,
}

#[derive(Deserialize)]
struct Closing {
    number: Option<u64>,
}

/// `Ok(None)` is no such pull request, or a word for its state this build has no meaning for, as
/// `gh`'s silence.
pub(super) fn facts(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<PullRequestFacts>, ApiFailure> {
    let found: InRepository<Facts> = api.graphql(FACTS, &Named::of(repository).asked(number))?;
    Ok(found.pull().and_then(facts_from))
}

/// A tab in a title would split the line `gh` reads, so it is a space there and here.
pub(crate) fn facts_from(pull: Facts) -> Option<PullRequestFacts> {
    let trimmed = |text: Option<String>| text.unwrap_or_default().trim().to_string();
    let closes = Nodes::all(pull.closing_issues_references).filter_map(|it| it.number).collect();
    facts_of(FactsRead {
        state: &trimmed(pull.state),
        draft: pull.is_draft?,
        auto_merge: pull.auto_merge_request.is_some(),
        branch: &trimmed(pull.head_ref_name),
        url: &trimmed(pull.url),
        title: &trimmed(pull.title).replace('\t', " "),
        body: pull.body.unwrap_or_default(),
        closes,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Landed {
    number: Option<u64>,
    title: Option<String>,
    state: Option<String>,
    mergeable: Option<String>,
    base_ref_name: Option<String>,
    base_ref_oid: Option<String>,
    head_ref_oid: Option<String>,
    merged_at: Option<String>,
}

pub(super) fn landing(
    api: &GitHubApi,
    repository: &GitHubRepository,
    pull_request: &str,
    number: u64,
) -> Result<Option<LandingOnTheForge>, ApiFailure> {
    let found: InRepository<Landed> = api.graphql(LANDING, &Named::of(repository).asked(number))?;
    Ok(Some(found.pull().map_or_else(unknown, |pull| landed(pull_request, pull))))
}

/// What became of `pull_request`. **A field the forge left blank is the forge's silence**, as for
/// `gh`: a pull request with no base branch is not one to act on. A pull request not yet merged
/// has no instant, which `gh` is asked to spell `-`.
pub(crate) fn landed(pull_request: &str, pull: Landed) -> LandingOnTheForge {
    let number = pull.number.map(|it| it.to_string()).unwrap_or_default();
    let words = [
        number,
        pull.title.unwrap_or_default(),
        pull.state.unwrap_or_default(),
        pull.mergeable.unwrap_or_default(),
        pull.base_ref_name.unwrap_or_default(),
        pull.base_ref_oid.unwrap_or_default(),
        pull.head_ref_oid.unwrap_or_default(),
        pull.merged_at.unwrap_or_else(|| "-".to_string()),
    ]
    .map(|word| word.trim().to_string());
    if words.iter().any(|word| word.is_empty()) {
        return unknown();
    }
    let [number, title, state, mergeable, base, pinned, head, merged] = &words;
    landing_of(
        pull_request,
        &LandingRead { number, title, state, mergeable, base, pinned, head, merged },
    )
}
