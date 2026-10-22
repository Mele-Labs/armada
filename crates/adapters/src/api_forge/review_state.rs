//! Who has looked at a pull request, what ran against it and what anybody wrote on it, over
//! GitHub's API: the fields `gh pr view` is asked for, and the line comments `gh api` pages.
//!
//! **Every list is read to its end**, as `gh` reads it, up to [`MOST_PAGES`]. A rollup cut by the
//! cap is [`WhatTheForgeRan::Unreadable`]: the caller is deciding whether work may go out, and
//! a pass over part of the checks is not one it can be given.

use adapter_traits::{FromOutside, Remark, ReviewedBy, UnderReview, WhatPeopleSaid, WhatTheForgeRan};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::connection::{later_page, read_to_the_end, Connection, More, Named, MOST_PAGES};
use super::page::InRepository;
use super::{login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::inline_comments::inline_remark;
use crate::under_review::{remark_of, settled, what_people_said, what_review_verdict, Checks};

const COMMENT: &str = "id author{login} createdAt body url";
const REVIEW: &str = "id author{login} submittedAt body url state";
const CONTEXT: &str = "__typename ... on CheckRun{name status conclusion} ... on StatusContext{context state}";
const PAGE_INFO: &str = "pageInfo{hasNextPage endCursor}";

/// The three lists, a hundred of each; the checks are the last commit's.
fn query() -> String {
    let selection = format!(
        "reviewDecision comments(first:100){{{PAGE_INFO} nodes{{{COMMENT}}}}} \
         reviews(first:100){{{PAGE_INFO} nodes{{{REVIEW}}}}} {}",
        rollup("first:100")
    );
    unasked(&selection)
}

/// The last commit's checks, `window` being which of them.
fn rollup(window: &str) -> String {
    let opens = "commits(last:1){nodes{commit{statusCheckRollup{";
    format!("{opens}contexts({window}){{{PAGE_INFO} nodes{{{CONTEXT}}}}}{}", "}".repeat(4))
}

/// A query for a pull request's fields with no `$after` to ask for.
fn unasked(selection: &str) -> String {
    format!(
        "query($owner:String!,$name:String!,$number:Int!){{repository(owner:$owner,name:$name){{\
         pullRequest(number:$number){{{selection}}}}}}}"
    )
}

fn more_comments() -> String {
    later_page(&format!("more:comments(first:100,after:$after){{{PAGE_INFO} nodes{{{COMMENT}}}}}"))
}

fn more_reviews() -> String {
    later_page(&format!("more:reviews(first:100,after:$after){{{PAGE_INFO} nodes{{{REVIEW}}}}}"))
}

fn more_contexts() -> String {
    later_page(&rollup("first:100,after:$after"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Pull {
    review_decision: Option<String>,
    comments: Option<Connection<Comment>>,
    reviews: Option<Connection<Review>>,
    commits: Option<Nodes<RollupCommit>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comment {
    id: Option<String>,
    author: Option<Login>,
    created_at: Option<String>,
    body: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Review {
    id: Option<String>,
    author: Option<Login>,
    submitted_at: Option<String>,
    body: Option<String>,
    url: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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
    contexts: Option<Connection<Context>>,
}

#[derive(Deserialize)]
struct Context {
    name: Option<String>,
    context: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    state: Option<String>,
}

impl RollupCommit {
    fn contexts(self) -> Option<Connection<Context>> {
        self.commit?.status_check_rollup?.contexts
    }
}

/// The page of a list after a cursor.
struct Later<'a> {
    api: &'a GitHubApi,
    named: &'a Named,
    number: u64,
}

impl Later<'_> {
    fn page<T: DeserializeOwned>(&self, query: &str, after: &str) -> Result<T, ApiFailure> {
        self.api.graphql(query, &self.named.asked_after(self.number, after))
    }
}

/// `Ok(None)` is no such pull request. It is read as `gh` reads it: comments, then review bodies,
/// then verdicts, and the checks totalled for a repository whose gate is `gate`.
pub(super) fn under_review(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
    gate: &str,
) -> Result<Option<UnderReview>, ApiFailure> {
    let named = Named::of(repository);
    let found: InRepository<Pull> = api.graphql(&query(), &named.asked(number))?;
    let Some(pull) = found.pull() else { return Ok(None) };

    let ask = Later { api, named: &named, number };
    let comments = read_to_the_end(pull.comments, MOST_PAGES, |after| {
        let found: InRepository<More<Comment>> = ask.page(&more_comments(), after)?;
        Ok(found.pull().and_then(|it| it.more))
    })?;
    let reviews = read_to_the_end(pull.reviews, MOST_PAGES, |after| {
        let found: InRepository<More<Review>> = ask.page(&more_reviews(), after)?;
        Ok(found.pull().and_then(|it| it.more))
    })?;
    let first = Nodes::all(pull.commits).next().and_then(RollupCommit::contexts);
    let contexts = read_to_the_end(first, MOST_PAGES, |after| {
        let found: InRepository<Pull> = ask.page(&more_contexts(), after)?;
        Ok(found.pull().and_then(|it| Nodes::all(it.commits).next()).and_then(RollupCommit::contexts))
    })?;

    let people = what_people_said(&pull.review_decision.unwrap_or_default());
    let mut checks = Checks::gated_by(gate);
    for context in &contexts.rows {
        let name = context.name.as_deref().or(context.context.as_deref()).unwrap_or_default();
        let conclusion = context.conclusion.as_deref().or(context.state.as_deref()).unwrap_or_default();
        checks.saw(name, context.status.as_deref().unwrap_or_default(), conclusion);
    }
    let mut remarks: Vec<Remark> = comments
        .rows
        .into_iter()
        .filter(|it| !it.body.as_deref().unwrap_or_default().is_empty())
        .filter_map(|it| {
            remark_of(
                it.id.unwrap_or_default(),
                login_of(it.author),
                it.created_at.unwrap_or_default(),
                it.body.unwrap_or_default(),
                it.url.unwrap_or_default(),
            )
        })
        .collect();
    let mut verdicts: Vec<ReviewedBy> = Vec::new();
    for Review { id, author, submitted_at, body, url, state } in reviews.rows {
        let by = login_of(author);
        if let Some(said) = body.filter(|it| !it.is_empty()) {
            remarks.extend(remark_of(
                id.unwrap_or_default(),
                by.clone(),
                submitted_at.unwrap_or_default(),
                said,
                url.unwrap_or_default(),
            ));
        }
        if let Some(verdict) = state.as_deref().and_then(what_review_verdict) {
            verdicts.push(ReviewedBy { by: FromOutside::verbatim(by), verdict });
        }
    }
    let mut read = settled(people, checks, remarks, verdicts);
    if contexts.cut && !matches!(people, WhatPeopleSaid::Unreadable) {
        read.checks = WhatTheForgeRan::Unreadable;
    }
    Ok(Some(read))
}

/// The most pages of line comments read: 3,000 comments.
const MOST_COMMENT_PAGES: usize = 30;

/// One line comment.
#[derive(Deserialize)]
pub(crate) struct Inline {
    id: Option<u64>,
    user: Option<Login>,
    created_at: Option<String>,
    body: Option<String>,
    html_url: Option<String>,
    path: Option<String>,
    line: Option<u32>,
    original_line: Option<u32>,
    diff_hunk: Option<String>,
}

/// Every line comment on pull request `number`. **A list cut by the cap is `None`**, the forge's
/// silence: the sweep that counts them keeps its last count on a silence, and a short list would
/// read as comments gone.
pub(super) fn inline_remarks(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<Vec<Remark>>, ApiFailure> {
    let paged = api.get_paged(&format!("{}/pulls/{number}/comments?per_page=100", repository.path()), MOST_COMMENT_PAGES)?;
    if paged.cut {
        return Ok(None);
    }
    let mut remarks = Vec::new();
    for page in &paged.pages {
        let comments: Vec<Option<Inline>> = page.decode("a pull request's line comments")?;
        remarks.extend(inline_remarks_of(comments.into_iter().flatten()));
    }
    Ok(Some(remarks))
}

/// The remarks a page of line comments holds, as `crate::inline_comments::folded` makes them: a
/// comment with no body is not one.
pub(crate) fn inline_remarks_of(comments: impl IntoIterator<Item = Inline>) -> Vec<Remark> {
    comments
        .into_iter()
        .filter(|it| !it.body.as_deref().unwrap_or_default().is_empty())
        .filter_map(|it| {
            inline_remark(
                it.id.map(|id| id.to_string()).unwrap_or_default(),
                login_of(it.user),
                it.created_at.unwrap_or_default(),
                it.body.unwrap_or_default(),
                it.html_url.unwrap_or_default(),
                it.path.unwrap_or_default(),
                it.line.or(it.original_line).unwrap_or(0),
                it.diff_hunk.unwrap_or_default(),
            )
        })
        .collect()
}
