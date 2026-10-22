//! Taking a pull request toward merged over GitHub's API: out of draft, into auto-merge, and its
//! failed checks started again. Each reads where the pull request stands before it writes, as
//! `gh` does, so the outcome is `gh`'s: what it takes for nothing to do is not a refusal.

use std::collections::BTreeSet;

use adapter_traits::{NotRerun, Rerun};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

use super::writes::{node, owner_and_name, Nothing};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::rerunning::run_ids;

const READY: &str = "mutation($id:ID!){markPullRequestReadyForReview(input:{pullRequestId:$id})\
    {clientMutationId}}";
const ENABLE: &str = "mutation($id:ID!){enablePullRequestAutoMerge(input:{pullRequestId:$id,\
    mergeMethod:MERGE}){clientMutationId}}";
const DISABLE: &str = "mutation($id:ID!){disablePullRequestAutoMerge(input:{pullRequestId:$id})\
    {clientMutationId}}";
const MERGE_NOW: &str = "mutation($id:ID!){mergePullRequest(input:{pullRequestId:$id,\
    mergeMethod:MERGE}){clientMutationId}}";

#[derive(Serialize)]
struct Id<'a> {
    id: &'a str,
}

fn mutate(api: &GitHubApi, query: &str, id: &str) -> Result<(), ApiFailure> {
    api.graphql::<IgnoredAny, _>(query, &Id { id }).map(drop)
}

/// [`Forge::mark_ready`](adapter_traits::Forge::mark_ready). A closed one is refused and one
/// already out of draft is done, as `gh pr ready` has it.
pub(super) fn mark_ready(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Result<(), String>, ApiFailure> {
    let pull = match node(api, repository, number)? {
        Ok(pull) => pull,
        Err(said) => return Ok(Err(said)),
    };
    if pull.state != "OPEN" {
        return Ok(Err(format!(
            "Pull request {}#{number} is closed. Only draft pull requests can be marked as \"ready for review\"",
            repository.address().slug()
        )));
    }
    if pull.is_draft {
        mutate(api, READY, &pull.id)?;
    }
    Ok(Ok(()))
}

/// `gh pr merge --auto --merge`, which is not always auto-merge: a queued pull request is left
/// alone; on a base with a merge queue the same mutation queues one that is ready; on one
/// without, a pull request that can merge now is merged now, and only one that cannot waits.
pub(super) fn enable_auto_merge(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Result<(), String>, ApiFailure> {
    let pull = match node(api, repository, number)? {
        Ok(pull) => pull,
        Err(said) => return Ok(Err(said)),
    };
    if pull.is_in_merge_queue || pull.state == "MERGED" {
        return Ok(Ok(()));
    }
    // As `gh pr merge --auto` has it, this being a swap: with no queue, one that can merge now is merged.
    let can_merge_now = matches!(pull.merge_state_status.as_str(), "CLEAN" | "HAS_HOOKS" | "UNSTABLE");
    let query = if pull.is_merge_queue_enabled || !can_merge_now { ENABLE } else { MERGE_NOW };
    mutate(api, query, &pull.id)?;
    Ok(Ok(()))
}

/// [`Forge::disable_auto_merge`](adapter_traits::Forge::disable_auto_merge). A queued pull
/// request is left alone, as `gh` leaves it.
pub(super) fn disable_auto_merge(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Result<(), String>, ApiFailure> {
    let pull = match node(api, repository, number)? {
        Ok(pull) => pull,
        Err(said) => return Ok(Err(said)),
    };
    if !pull.is_in_merge_queue {
        mutate(api, DISABLE, &pull.id)?;
    }
    Ok(Ok(()))
}

const CHECKS: &str = "query($owner:String!,$name:String!,$number:Int!,$after:String){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100,after:$after){\
    pageInfo{hasNextPage endCursor} nodes{__typename \
    ... on CheckRun{name status conclusion startedAt detailsUrl \
    checkSuite{workflowRun{event workflow{name}}}} \
    ... on StatusContext{context state targetUrl}}}}}}}}}}";

#[derive(Serialize)]
struct Asked<'a> {
    owner: &'a str,
    name: &'a str,
    number: u64,
    after: Option<&'a str>,
}

/// The most pages of checks read: 1,000 of them.
const MOST_PAGES: usize = 10;

#[derive(Deserialize)]
struct Checked {
    repository: Option<InRepository>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InRepository {
    pull_request: Option<Commits>,
}

#[derive(Deserialize)]
struct Commits {
    commits: Option<super::Nodes<InCommit>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InCommit {
    commit: Option<Commit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    status_check_rollup: Option<Rollup>,
}

#[derive(Deserialize)]
struct Rollup {
    contexts: Option<Contexts>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contexts {
    page_info: Option<PageInfo>,
    nodes: Option<Vec<Option<Context>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: Option<bool>,
    end_cursor: Option<String>,
}

/// A check run or a status, as `gh pr checks` reads either.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Context {
    name: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    started_at: Option<String>,
    details_url: Option<String>,
    check_suite: Option<Suite>,
    context: Option<String>,
    state: Option<String>,
    target_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Suite {
    workflow_run: Option<WorkflowRun>,
}

#[derive(Deserialize)]
struct WorkflowRun {
    event: Option<String>,
    workflow: Option<Named>,
}

#[derive(Deserialize)]
struct Named {
    name: Option<String>,
}

fn said(word: &Option<String>) -> &str {
    word.as_deref().unwrap_or("")
}

impl Context {
    /// What `gh pr checks` calls its state: a status's own, else a finished run's conclusion,
    /// else where the run is.
    fn standing(&self) -> &str {
        match self.state.as_deref().filter(|state| !state.is_empty()) {
            Some(state) => state,
            None if said(&self.status) == "COMPLETED" => said(&self.conclusion),
            None => said(&self.status),
        }
    }

    /// What `gh pr checks` links to.
    fn link(&self) -> &str {
        [&self.details_url, &self.target_url]
            .into_iter()
            .filter_map(|it| it.as_deref())
            .find(|it| !it.is_empty())
            .unwrap_or("")
    }

    /// `gh pr checks` keeps the latest of a repeated check: a status by its context, a run by
    /// its name, its workflow and the event that started it.
    fn repeats(&self) -> String {
        if let Some(context) = self.context.as_deref().filter(|it| !it.is_empty()) {
            return format!("context:{context}");
        }
        let run = self.check_suite.as_ref().and_then(|it| it.workflow_run.as_ref());
        let workflow = run.and_then(|it| it.workflow.as_ref()).and_then(|it| it.name.as_deref());
        let event = run.and_then(|it| it.event.as_deref());
        format!("check:{}/{}/{}", self.name.as_deref().unwrap_or(""), workflow.unwrap_or(""), event.unwrap_or(""))
    }
}

/// The links of the failed checks, latest of each only: `bucket == "fail"` is an error, a
/// failure, a timeout or an action required.
fn failed_links(mut contexts: Vec<Context>) -> (usize, Vec<String>) {
    let seen = contexts.len();
    contexts.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    let mut kept = BTreeSet::new();
    let links = contexts
        .iter()
        .filter(|one| kept.insert(one.repeats()))
        .filter(|one| matches!(one.standing(), "ERROR" | "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED"))
        .map(|one| one.link().to_string())
        .collect();
    (seen, links)
}

/// [`Forge::rerun_failed`](adapter_traits::Forge::rerun_failed): the checks asked of the last
/// commit, each failed run's failed jobs started again.
pub(super) fn rerun_failed(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Result<Rerun, NotRerun>, ApiFailure> {
    let (owner, name) = owner_and_name(repository);
    let mut contexts = Vec::new();
    let mut after: Option<String> = None;
    // A failing check past the first hundred is still started again, up to a cap.
    for _ in 0..MOST_PAGES {
        let asked: Checked =
            api.graphql(CHECKS, &Asked { owner: &owner, name: &name, number, after: after.as_deref() })?;
        let rollup = asked
            .repository
            .and_then(|it| it.pull_request)
            .into_iter()
            .flat_map(|it| super::Nodes::all(it.commits))
            .filter_map(|it| it.commit)
            .find_map(|it| it.status_check_rollup)
            .and_then(|it| it.contexts);
        let Some(page) = rollup else { break };
        contexts.extend(page.nodes.unwrap_or_default().into_iter().flatten());
        match page.page_info {
            Some(PageInfo { has_next_page: Some(true), end_cursor: Some(cursor) }) => after = Some(cursor),
            _ => break,
        }
    }
    let (seen, links) = failed_links(contexts);
    let runs = run_ids(&links.join("\n"));
    if runs.is_empty() {
        let said = match (seen, links.is_empty()) {
            (0, _) => format!("no checks reported on pull request {number}"),
            (_, true) => format!("no check has failed on pull request {number}"),
            _ => "no failed check names a run the forge can start again".to_string(),
        };
        return Ok(Err(NotRerun { said }));
    }
    for id in &runs {
        api.post(&format!("{}/actions/runs/{id}/rerun-failed-jobs", repository.path()), &Nothing {})?;
    }
    Ok(Ok(Rerun { runs: runs.len() }))
}
