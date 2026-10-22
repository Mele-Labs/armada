//! The pull requests of a repository over GitHub's API: the open ones with their checks, one by
//! number, the newest merged, the merge queue, and the login the token belongs to. The queries
//! ask for what `gh pr list` and `gh pr view` ask for, and the rows are folded by the functions
//! that fold `gh`'s.

use adapter_traits::{
    FromOutside, MergeQueue, OpenPull, OpenPulls, PullFacts, QueueEntry, RecentlyMerged, RecentlyMergedPulls,
};
use serde::{Deserialize, Serialize};

use super::connection::Named;
use super::page::InRepository;
use super::{login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::main_ci::{
    check_state_of, is_a_commit, mergeable_word, newest_first, open_pull_built, review_word, OpenRead, LISTED,
};
use crate::merge_queue::{state_of, QUERY as QUEUE};

/// What `gh pr list` asks of each pull request, and its checks: the last commit's, a hundred.
const FIELDS: &str = "number title headRefName url author{login} headRefOid autoMergeRequest{enabledAt} \
    createdAt updatedAt isDraft mergeable reviewDecision additions deletions \
    commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100){nodes{__typename \
    ... on CheckRun{name status conclusion} ... on StatusContext{context state}}}}}}}";

/// Newest created first, as `gh pr list` lists them.
fn list_query() -> String {
    format!(
        "query($owner:String!,$name:String!,$first:Int!){{repository(owner:$owner,name:$name){{\
         pullRequests(states:OPEN,first:$first,orderBy:{{field:CREATED_AT,direction:DESC}}){{nodes{{{FIELDS}}}}}}}}}"
    )
}

fn one_query() -> String {
    format!(
        "query($owner:String!,$name:String!,$number:Int!){{repository(owner:$owner,name:$name){{\
         pullRequest(number:$number){{state {FIELDS}}}}}}}"
    )
}

const MERGED_QUERY: &str = "query($owner:String!,$name:String!,$base:String!,$first:Int!){\
    repository(owner:$owner,name:$name){pullRequests(states:MERGED,baseRefName:$base,first:$first,\
    orderBy:{field:CREATED_AT,direction:DESC}){nodes{number title headRefName url author{login} \
    mergedAt mergeCommit{oid}}}}}";

#[derive(Serialize)]
struct AskedFirst<'a> {
    owner: &'a str,
    name: &'a str,
    first: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    pull_requests: Option<Nodes<Pull>>,
}

#[derive(Deserialize)]
pub(super) struct Repo<T> {
    pub(super) repository: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Pull {
    state: Option<String>,
    number: Option<u64>,
    title: Option<String>,
    head_ref_name: Option<String>,
    url: Option<String>,
    author: Option<Login>,
    head_ref_oid: Option<String>,
    auto_merge_request: Option<serde::de::IgnoredAny>,
    created_at: Option<String>,
    updated_at: Option<String>,
    is_draft: Option<bool>,
    mergeable: Option<String>,
    review_decision: Option<String>,
    additions: Option<u32>,
    deletions: Option<u32>,
    commits: Option<Nodes<RollupCommit>>,
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
struct Context {
    name: Option<String>,
    context: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    state: Option<String>,
}

/// Every open pull request. **A listing the forge will not answer is asked again for fewer**,
/// as `gh`'s is: a repository with many checks a pull request times the forge out at a hundred.
/// A refusal for want of a token is not one to ask again of.
pub(super) fn open_pulls(
    api: &GitHubApi,
    repository: &GitHubRepository,
    gate: &str,
) -> Result<Option<OpenPulls>, ApiFailure> {
    let named = Named::of(repository);
    let query = list_query();
    let mut failed = None;
    for limit in LISTED {
        let asked = AskedFirst { owner: named.owner(), name: named.name(), first: limit.parse().unwrap_or(25) };
        match api.graphql::<Repo<Listed>, _>(&query, &asked) {
            Ok(found) => {
                let pulls = found.repository.and_then(|it| it.pull_requests);
                return Ok(Some(Nodes::all(pulls).filter_map(|pull| open_pull_of(pull, gate)).collect()));
            }
            Err(no_token @ ApiFailure::NoToken(_)) => return Err(no_token),
            Err(other) => failed = Some(other),
        }
    }
    Err(failed.unwrap_or_else(|| ApiFailure::Malformed("No listing was asked.".to_string())))
}

/// One pull request by its number. `Ok(None)` where it is not open, or not there.
pub(super) fn open_pull(
    api: &GitHubApi,
    repository: &GitHubRepository,
    gate: &str,
    number: u64,
) -> Result<Option<OpenPull>, ApiFailure> {
    let found: InRepository<Pull> = api.graphql(&one_query(), &Named::of(repository).asked(number))?;
    Ok(found.pull().filter(|pull| pull.state.as_deref() == Some("OPEN")).and_then(|pull| open_pull_of(pull, gate)))
}

/// A pull request as `crate::main_ci::open_pull_of_line` makes one from `gh`'s line.
pub(crate) fn open_pull_of(pull: Pull, gate: &str) -> Option<OpenPull> {
    let checks = Nodes::all(pull.commits)
        .next()
        .and_then(|it| it.commit?.status_check_rollup?.contexts)
        .map(|contexts| Nodes::all(Some(contexts)).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|check| {
            let name = check.name.or(check.context).unwrap_or_default();
            let status = check.status.unwrap_or_default().to_ascii_lowercase();
            let conclusion = check.conclusion.or(check.state).unwrap_or_default().to_ascii_lowercase();
            (name, check_state_of(&status, &conclusion))
        })
        .collect();
    let named = |text: Option<String>| text.filter(|it| !it.is_empty()).map(FromOutside::verbatim);
    Some(open_pull_built(
        OpenRead {
            number: pull.number?,
            title: pull.title.unwrap_or_default(),
            branch: pull.head_ref_name.unwrap_or_default(),
            url: pull.url.unwrap_or_default(),
            author: login_of(pull.author),
            checks,
            head: pull.head_ref_oid.unwrap_or_default(),
            auto_merge: pull.auto_merge_request.is_some(),
            facts: PullFacts {
                opened_at: named(pull.created_at),
                updated_at: named(pull.updated_at),
                draft: pull.is_draft.unwrap_or(false),
                mergeable: pull.mergeable.filter(|it| !it.is_empty()).map(|it| mergeable_word(&it)),
                review: Some(review_word(&pull.review_decision.unwrap_or_default())),
                additions: pull.additions,
                deletions: pull.deletions,
            },
        },
        gate,
    ))
}

#[derive(Serialize)]
struct AskedMerged<'a> {
    owner: &'a str,
    name: &'a str,
    base: &'a str,
    first: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Merged {
    number: Option<u64>,
    title: Option<String>,
    head_ref_name: Option<String>,
    url: Option<String>,
    author: Option<Login>,
    merged_at: Option<String>,
    merge_commit: Option<Oid>,
}

#[derive(Deserialize)]
struct Oid {
    oid: Option<String>,
}

/// The most a page of the forge's list holds. **A `limit` past it is read as it**: Fleet asks for
/// thirty.
const MOST_A_PAGE: usize = 100;

/// The newest `limit` pull requests merged into `base`, newest first.
pub(super) fn recently_merged(
    api: &GitHubApi,
    repository: &GitHubRepository,
    base: &str,
    limit: usize,
) -> Result<Option<RecentlyMergedPulls>, ApiFailure> {
    if base.is_empty() || base.starts_with('-') {
        return Ok(None);
    }
    let named = Named::of(repository);
    let asked = AskedMerged { owner: named.owner(), name: named.name(), base, first: limit.clamp(1, MOST_A_PAGE) };
    let found: Repo<MergedList> = api.graphql(MERGED_QUERY, &asked)?;
    let nodes = found.repository.and_then(|it| it.pull_requests);
    Ok(Some(newest_first(Nodes::all(nodes).filter_map(recently_merged_of).collect())))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergedList {
    pull_requests: Option<Nodes<Merged>>,
}

pub(crate) fn recently_merged_of(pull: Merged) -> Option<RecentlyMerged> {
    let merged_at = pull.merged_at.filter(|it| !it.is_empty())?;
    let author = login_of(pull.author);
    let commit = pull.merge_commit.and_then(|it| it.oid).unwrap_or_default();
    Some(RecentlyMerged {
        number: pull.number?,
        title: FromOutside::verbatim(pull.title.unwrap_or_default()),
        branch: FromOutside::verbatim(pull.head_ref_name.unwrap_or_default()),
        url: FromOutside::verbatim(pull.url.unwrap_or_default()),
        author: (!author.is_empty()).then(|| FromOutside::verbatim(author)),
        merged_at: FromOutside::verbatim(merged_at),
        commit: is_a_commit(&commit).then(|| FromOutside::verbatim(commit)),
    })
}

#[derive(Serialize)]
struct AskedBranch<'a> {
    owner: &'a str,
    name: &'a str,
    branch: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Queued {
    merge_queue: Option<Entries>,
}

#[derive(Deserialize)]
struct Entries {
    entries: Option<Nodes<Entry>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    position: Option<u32>,
    state: Option<String>,
    pull_request: Option<Numbered>,
}

#[derive(Deserialize)]
struct Numbered {
    number: Option<u64>,
}

/// The queue in position order; a repository with no queue answers with none waiting.
pub(super) fn merge_queue(
    api: &GitHubApi,
    repository: &GitHubRepository,
    base: &str,
) -> Result<Option<MergeQueue>, ApiFailure> {
    if base.is_empty() || base.starts_with('-') {
        return Ok(None);
    }
    let named = Named::of(repository);
    let found: Repo<Queued> = api.graphql(QUEUE, &AskedBranch { owner: named.owner(), name: named.name(), branch: base })?;
    let entries = found.repository.and_then(|it| it.merge_queue).and_then(|it| it.entries);
    Ok(Some(queue_of(Nodes::all(entries))))
}

pub(crate) fn queue_of(entries: impl IntoIterator<Item = Entry>) -> MergeQueue {
    let mut queue: MergeQueue = entries
        .into_iter()
        .filter_map(|entry| {
            Some(QueueEntry {
                number: entry.pull_request?.number?,
                position: entry.position?,
                state: state_of(entry.state.as_deref().unwrap_or_default()),
            })
        })
        .collect();
    queue.sort_by_key(|entry| entry.position);
    queue
}

#[derive(Deserialize)]
struct Account {
    login: Option<String>,
}

/// The login the token belongs to.
pub(super) fn forge_login(api: &GitHubApi) -> Result<Option<String>, ApiFailure> {
    let account: Account = api.get("/user")?.decode("the account a token belongs to")?;
    Ok(account.login.map(|it| it.trim().to_string()).filter(|it| !it.is_empty()))
}
