//! A repository's issues over GitHub's API: the open ones, one by number, and the title and body
//! of a linked one. The queries are the ones `gh issue list` and `gh api graphql` are given in
//! `crate::issue_tracker`, and the rows are made the way it makes them.

use adapter_traits::{
    FromOutside, IssueComment, LinkedPullRequest, LinkedPullRequestState, OpenIssue, ReadIssue,
};
use serde::{Deserialize, Serialize};

use super::connection::{read_to_the_end, Connection, Named};
use super::open_pulls::Repo;
use super::{login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::issue_tracker::{EXCERPT, LISTED, READ_ISSUE_QUERY};

/// A page of the listing is a hundred, which is as many as the forge gives.
const A_PAGE: usize = 100;

const ISSUE_FIELDS: &str = "number title url labels(first:100){nodes{name}} assignees(first:100){nodes{login}} \
    updatedAt body closedByPullRequestsReferences(first:1){nodes{number}}";

/// Newest created first, as `gh issue list` lists them.
fn list_query(after: bool) -> String {
    let (declared, asked) = match after {
        true => (",$after:String!", ",after:$after"),
        false => ("", ""),
    };
    format!(
        "query($owner:String!,$name:String!,$first:Int!{declared}){{repository(owner:$owner,name:$name){{\
         issues(states:OPEN,first:$first{asked},orderBy:{{field:CREATED_AT,direction:DESC}}){{\
         pageInfo{{hasNextPage endCursor}} nodes{{{ISSUE_FIELDS}}}}}}}}}"
    )
}

#[derive(Serialize)]
struct AskedFirst<'a> {
    owner: &'a str,
    name: &'a str,
    first: usize,
}

#[derive(Serialize)]
struct AskedNext<'a> {
    owner: &'a str,
    name: &'a str,
    first: usize,
    after: &'a str,
}

#[derive(Deserialize)]
struct Listing {
    issues: Option<Connection<Listed>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Listed {
    number: Option<u64>,
    title: Option<String>,
    url: Option<String>,
    labels: Option<Nodes<Label>>,
    assignees: Option<Nodes<Login>>,
    updated_at: Option<String>,
    body: Option<String>,
    closed_by_pull_requests_references: Option<Nodes<Numbered>>,
}

#[derive(Deserialize)]
struct Label {
    name: Option<String>,
}

#[derive(Deserialize)]
struct Numbered {
    #[allow(dead_code)]
    number: Option<u64>,
}

/// The open issues, the newest edit first. The listing is the newest created two hundred, as
/// `gh` lists it; a backlog past that is read newest edit first among them.
pub(super) fn open_issues(api: &GitHubApi, repository: &GitHubRepository) -> Result<Vec<OpenIssue>, ApiFailure> {
    let named = Named::of(repository);
    let first = AskedFirst { owner: named.owner(), name: named.name(), first: A_PAGE };
    let found: Repo<Listing> = api.graphql(&list_query(false), &first)?;
    let pages = LISTED.parse::<usize>().unwrap_or(A_PAGE).div_ceil(A_PAGE);
    let issues = read_to_the_end(found.repository.and_then(|it| it.issues), pages, |after| {
        let next = AskedNext { owner: named.owner(), name: named.name(), first: A_PAGE, after };
        let found: Repo<Listing> = api.graphql(&list_query(true), &next)?;
        Ok(found.repository.and_then(|it| it.issues))
    })?;
    let mut issues: Vec<OpenIssue> = issues.rows.into_iter().filter_map(open_issue_of).collect();
    // RFC 3339 in one zone sorts as text.
    issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(issues)
}

/// An issue as `crate::issue_tracker::issue_of_line` makes one from `gh`'s line.
pub(crate) fn open_issue_of(issue: Listed) -> Option<OpenIssue> {
    let number = issue.number?;
    let names = |names: Vec<String>| -> Vec<FromOutside> {
        names.into_iter().filter(|one| !one.is_empty()).map(FromOutside::verbatim).collect()
    };
    Some(OpenIssue {
        reference: format!("#{number}"),
        title: FromOutside::verbatim(issue.title.unwrap_or_default()),
        url: FromOutside::verbatim(issue.url.unwrap_or_default()),
        labels: names(Nodes::all(issue.labels).filter_map(|it| it.name).collect()),
        assignees: names(Nodes::all(issue.assignees).filter_map(|it| it.login).collect()),
        updated_at: issue.updated_at.unwrap_or_default(),
        excerpt: FromOutside::verbatim(excerpt_of(&issue.body.unwrap_or_default())),
        linked_pull_request: Nodes::all(issue.closed_by_pull_requests_references).next().is_some(),
    })
}

/// The start of a body with every run of white space one space and none trimmed, as `gh`'s
/// `gsub("\\s+"; " ")` leaves it, cut at [`EXCERPT`] characters.
fn excerpt_of(body: &str) -> String {
    let mut folded = String::with_capacity(EXCERPT);
    let mut in_space = false;
    for c in body.chars() {
        if c.is_whitespace() {
            if !in_space {
                folded.push(' ');
            }
            in_space = true;
        } else {
            folded.push(c);
            in_space = false;
        }
        if folded.chars().count() >= EXCERPT {
            break;
        }
    }
    folded.chars().take(EXCERPT).collect()
}

#[derive(Deserialize)]
struct Reading {
    issue: Option<Read>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Read {
    number: Option<u64>,
    title: Option<String>,
    url: Option<String>,
    state: Option<String>,
    body: Option<String>,
    created_at: Option<String>,
    author: Option<Login>,
    labels: Option<Nodes<Label>>,
    assignees: Option<Nodes<Login>>,
    comments: Option<Nodes<Said>>,
    closed_by_pull_requests_references: Option<Nodes<Linked>>,
    timeline_items: Option<Nodes<CrossReference>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Said {
    author: Option<Login>,
    created_at: Option<String>,
    body: Option<String>,
}

#[derive(Deserialize)]
struct CrossReference {
    source: Option<Linked>,
}

#[derive(Deserialize)]
struct Linked {
    number: Option<u64>,
    title: Option<String>,
    url: Option<String>,
    state: Option<String>,
}

/// The issue by number. `Ok(None)` where the forge has none by that number.
pub(super) fn read_issue(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<ReadIssue>, ApiFailure> {
    let found: Repo<Reading> = api.graphql(READ_ISSUE_QUERY, &Named::of(repository).asked(number))?;
    Ok(found.repository.and_then(|it| it.issue).and_then(read_issue_of))
}

/// An issue as `crate::issue_tracker::read_issue_of_records` makes one from `gh`'s records.
pub(crate) fn read_issue_of(issue: Read) -> Option<ReadIssue> {
    let names = |names: Vec<String>| -> Vec<FromOutside> {
        names.into_iter().filter(|one| !one.is_empty()).map(FromOutside::verbatim).collect()
    };
    let number = issue.number?;
    let state = issue.state.unwrap_or_default();
    let mut read = ReadIssue::bare(
        format!("#{number}"),
        FromOutside::verbatim(issue.title.unwrap_or_default()),
        FromOutside::verbatim(issue.url.unwrap_or_default()),
        !state.eq_ignore_ascii_case("open"),
    );
    read.body = FromOutside::verbatim(issue.body.unwrap_or_default());
    read.author = FromOutside::verbatim(login_of(issue.author));
    read.opened_at = issue.created_at.unwrap_or_default();
    read.labels = names(Nodes::all(issue.labels).filter_map(|it| it.name).collect());
    read.assignees = names(Nodes::all(issue.assignees).filter_map(|it| it.login).collect());
    read.comments = Nodes::all(issue.comments)
        .map(|it| IssueComment {
            author: FromOutside::verbatim(login_of(it.author)),
            at: it.created_at.unwrap_or_default(),
            body: FromOutside::verbatim(it.body.unwrap_or_default()),
        })
        .collect();
    // The pull requests that close it, then the ones that name it; a cross-reference from an
    // issue has no source here, and one that does both is listed once.
    let named = Nodes::all(issue.timeline_items).filter_map(|it| it.source);
    for pull in Nodes::all(issue.closed_by_pull_requests_references).chain(named) {
        let Some(number) = pull.number else { continue };
        let url = pull.url.unwrap_or_default();
        if read.linked_pull_requests.iter().all(|one| one.url.as_written() != url) {
            read.linked_pull_requests.push(LinkedPullRequest {
                number,
                title: FromOutside::verbatim(pull.title.unwrap_or_default()),
                url: FromOutside::verbatim(url),
                state: match pull.state.as_deref() {
                    Some("MERGED") => LinkedPullRequestState::Merged,
                    Some("CLOSED") => LinkedPullRequestState::Closed,
                    _ => LinkedPullRequestState::Open,
                },
            });
        }
    }
    Some(read)
}

/// Whether the forge's refusal says there is no such issue: a 404, or GraphQL's own words.
pub(super) fn is_no_such_issue(why: &ApiFailure) -> bool {
    match why {
        ApiFailure::Refused { status, .. } => *status == 404,
        ApiFailure::GraphQl(messages) => messages.iter().any(|it| it.contains("Could not resolve to an")),
        _ => false,
    }
}

const TEXT_QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){\
    issue(number:$number){title body}}}";

#[derive(Deserialize)]
struct Texting {
    issue: Option<Text>,
}

#[derive(Deserialize)]
struct Text {
    title: Option<String>,
    body: Option<String>,
}

/// An issue's title and body, as `gh issue view --json title,body` prints them: the title, a
/// blank line, the body. Whatever it said is trimmed, and nothing said is the reason it was not
/// read.
pub(super) fn text(api: &GitHubApi, repository: &GitHubRepository, number: u64) -> Result<String, ApiFailure> {
    let found: Repo<Texting> = api.graphql(TEXT_QUERY, &Named::of(repository).asked(number))?;
    let issue = found.repository.and_then(|it| it.issue);
    let text = issue.map(|it| format!("{}\n\n{}", it.title.unwrap_or_default(), it.body.unwrap_or_default()));
    Ok(text.unwrap_or_default().trim().to_string())
}
