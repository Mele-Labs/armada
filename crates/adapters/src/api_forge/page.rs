//! One pull request's page over GitHub's API: the fields `gh pr view` reads, asked by the same
//! names in one GraphQL query, then the parts `crate::pull_request_detail` folds `gh`'s records
//! into. The threads are read beside the page, then every profile and team beside each other.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{
    CheckRow, CommitLine, Omitted, PageFile, PullRequestPageFacts, ReviewThreadFacts, ReviewerKind, Said,
};
use core_model::Timestamp;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{joined, login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::pull_request_detail::{
    check_standing, file_change, mergeability, review_said, review_standing, with_profiles, with_teams, Head,
    PageParts, Profile, MOST_CHECKS, MOST_COMMITS, MOST_FILES,
};

/// `rollup` is the last commit's checks, as `gh` reads `statusCheckRollup`; `commits` is the
/// list. Comments and reviews are read from the end, since the page keeps the latest.
pub(crate) const QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    baseRefName author{login} createdAt additions deletions mergeable reviewDecision headRefOid body \
    latestReviews(first:100){nodes{author{login} state}} \
    reviewRequests(first:100){nodes{requestedReviewer{__typename ... on User{login} ... on Bot{login} \
    ... on Mannequin{login} ... on Team{slug name}}}} \
    comments(last:100){totalCount nodes{author{login} createdAt body}} \
    reviews(last:100){totalCount nodes{author{login} submittedAt state body}} \
    rollup:commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100){totalCount \
    pageInfo{hasNextPage endCursor} nodes{__typename \
    ... on CheckRun{name status conclusion startedAt completedAt detailsUrl} \
    ... on StatusContext{context state targetUrl}}}}}}} \
    files(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{path additions deletions changeType}} \
    commits(first:100){totalCount pageInfo{hasNextPage endCursor} nodes{commit{oid messageHeadline \
    committedDate authoredDate authors(first:1){nodes{user{login}}}}}}}}}";

/// The files past the first hundred, a hundred at a time.
const MORE_FILES: &str = "query($owner:String!,$name:String!,$number:Int!,$after:String!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    more:files(first:100,after:$after){totalCount pageInfo{hasNextPage endCursor} \
    nodes{path additions deletions changeType}}}}}";

/// The commits past the first hundred, a hundred at a time.
const MORE_COMMITS: &str = "query($owner:String!,$name:String!,$number:Int!,$after:String!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    more:commits(first:100,after:$after){totalCount pageInfo{hasNextPage endCursor} \
    nodes{commit{oid messageHeadline committedDate authoredDate authors(first:1){nodes{user{login}}}}}}}}}";

/// The checks past the first hundred, a hundred at a time.
const MORE_CHECKS: &str = "query($owner:String!,$name:String!,$number:Int!,$after:String!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    rollup:commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100,after:$after){totalCount \
    pageInfo{hasNextPage endCursor} nodes{__typename \
    ... on CheckRun{name status conclusion startedAt completedAt detailsUrl} \
    ... on StatusContext{context state targetUrl}}}}}}}}}}";

/// The most pages of a team's people read: 1,000 people.
const MOST_MEMBER_PAGES: usize = 10;

#[derive(Serialize)]
pub(super) struct Asked<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub number: u64,
}

#[derive(Serialize)]
struct AskedAfter<'a> {
    owner: &'a str,
    name: &'a str,
    number: u64,
    after: &'a str,
}

/// `data`, down to the pull request.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InRepository<T> {
    repository: Option<InPull<T>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InPull<T> {
    pull_request: Option<T>,
}

impl<T> InRepository<T> {
    pub(super) fn pull(self) -> Option<T> {
        self.repository?.pull_request
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Pull {
    base_ref_name: Option<String>,
    author: Option<Login>,
    created_at: Option<String>,
    additions: Option<u64>,
    deletions: Option<u64>,
    mergeable: Option<String>,
    review_decision: Option<String>,
    head_ref_oid: Option<String>,
    body: Option<String>,
    latest_reviews: Option<Nodes<Verdict>>,
    review_requests: Option<Nodes<Request>>,
    comments: Option<Counted<Comment>>,
    reviews: Option<Counted<Reviewed>>,
    rollup: Option<Nodes<RollupCommit>>,
    files: Option<Counted<File>>,
    commits: Option<Counted<CommitNode>>,
}

/// A list with its length on the forge, and where the next page starts.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Counted<T> {
    total_count: Option<usize>,
    page_info: Option<PageInfo>,
    nodes: Option<Vec<Option<T>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: Option<bool>,
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
struct More<T> {
    more: Option<Counted<T>>,
}

#[derive(Deserialize)]
struct Verdict {
    author: Option<Login>,
    state: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    requested_reviewer: Option<Requested>,
}

#[derive(Deserialize)]
struct Requested {
    #[serde(rename = "__typename")]
    kind: Option<String>,
    login: Option<String>,
    slug: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comment {
    author: Option<Login>,
    created_at: Option<String>,
    body: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reviewed {
    author: Option<Login>,
    submitted_at: Option<String>,
    state: Option<String>,
    body: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RollupCommit {
    commit: Option<RollupOf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RollupOf {
    status_check_rollup: Option<Contexts>,
}

#[derive(Deserialize)]
struct Contexts {
    contexts: Option<Counted<Context>>,
}

/// `rollup` of a later page of checks.
#[derive(Deserialize)]
struct MoreChecks {
    rollup: Option<Nodes<RollupCommit>>,
}

/// A check run or a status context: one carries `name`, `status` and `conclusion`, the other
/// `context` and `state`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Context {
    name: Option<String>,
    context: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    state: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    details_url: Option<String>,
    target_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    path: Option<String>,
    additions: Option<u64>,
    deletions: Option<u64>,
    change_type: Option<String>,
}

#[derive(Deserialize)]
struct CommitNode {
    commit: Option<CommitOf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitOf {
    oid: Option<String>,
    message_headline: Option<String>,
    committed_date: Option<String>,
    authored_date: Option<String>,
    authors: Option<Nodes<CommitAuthor>>,
}

#[derive(Deserialize)]
struct CommitAuthor {
    user: Option<Login>,
}

/// `users/<login>`.
#[derive(Deserialize)]
struct UserProfile {
    name: Option<String>,
    html_url: Option<String>,
}

/// The page, its threads, then its people's names and teams. `Ok(None)` is no such pull request,
/// or one with no base, as `gh`'s silence.
pub(super) fn read(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<PullRequestPageFacts>, ApiFailure> {
    let (page, threads) = std::thread::scope(|scope| {
        let threads = scope.spawn(|| threads_of(api, repository, number));
        (page_of(api, repository, number), joined(threads))
    });
    let Some(mut page) = page? else { return Ok(None) };
    // **A page without its threads is not a page**, for `crate::pull_request_detail::read`'s reason.
    page.threads = threads?;
    let (profiles, members) = people_of(api, repository, &page);
    let mut profiles = profiles;
    with_profiles(&mut page.reviewers, |login| profiles.remove(login).flatten());
    with_teams(&mut page.reviewers, |slug| members.get(slug).cloned().flatten());
    Ok(Some(page))
}

/// Every person's profile and every team's people, all at once. Any that fails is `None`: a name
/// is garnish, and a reviewer the forge will not name stays bare.
#[allow(clippy::type_complexity)]
fn people_of(
    api: &GitHubApi,
    repository: &GitHubRepository,
    page: &PullRequestPageFacts,
) -> (BTreeMap<String, Option<Profile>>, BTreeMap<String, Option<Vec<String>>>) {
    let of_kind = |kind: ReviewerKind| -> BTreeSet<&str> {
        page.reviewers.iter().filter(|it| it.kind == kind).map(|it| it.login.as_str()).collect()
    };
    let (people, teams) = (of_kind(ReviewerKind::Person), of_kind(ReviewerKind::Team));
    let owner = repository.address().slug();
    let owner = owner.split('/').next().unwrap_or_default().to_string();
    std::thread::scope(|scope| {
        let profiles: Vec<_> =
            people.iter().map(|login| (*login, scope.spawn(move || profile_of(api, login)))).collect();
        let owner = &owner;
        let members: Vec<_> =
            teams.iter().map(|slug| (*slug, scope.spawn(move || members_of(api, owner, slug)))).collect();
        (
            profiles.into_iter().map(|(login, read)| (login.to_string(), joined(read))).collect(),
            members.into_iter().map(|(slug, read)| (slug.to_string(), joined(read))).collect(),
        )
    })
}

fn profile_of(api: &GitHubApi, login: &str) -> Option<Profile> {
    let profile: UserProfile = api.get(&format!("/users/{login}")).ok()?.decode("a profile").ok()?;
    Profile::of(profile.name, profile.html_url)
}

/// A team's people, by login. A list cut at the cap leaves those past it ungrouped.
fn members_of(api: &GitHubApi, owner: &str, slug: &str) -> Option<Vec<String>> {
    let paged = api.get_paged(&format!("/orgs/{owner}/teams/{slug}/members?per_page=100"), MOST_MEMBER_PAGES).ok()?;
    let mut members = Vec::new();
    for page in &paged.pages {
        let people: Vec<super::Login> = page.decode("a team's people").ok()?;
        members.extend(people.into_iter().filter_map(|it| it.login));
    }
    Some(members)
}

fn threads_of(api: &GitHubApi, repository: &GitHubRepository, number: u64) -> Result<Vec<ReviewThreadFacts>, ApiFailure> {
    let slug = repository.address().slug();
    let (owner, name) = slug.split_once('/').unwrap_or((&slug, ""));
    let found: InRepository<super::threads::Threads> =
        api.graphql(crate::pull_request_threads::QUERY, &Asked { owner, name, number })?;
    Ok(super::threads::threads(found.pull()))
}

/// The page without its threads or its people's names.
fn page_of(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<PullRequestPageFacts>, ApiFailure> {
    let slug = repository.address().slug();
    let (owner, name) = slug.split_once('/').unwrap_or((&slug, ""));
    let found: InRepository<Pull> = api.graphql(QUERY, &Asked { owner, name, number })?;
    let Some(mut pull) = found.pull() else { return Ok(None) };
    let asked = Asked { owner, name, number };
    pull.files = rest_of(api, MORE_FILES, &asked, pull.files.take(), MOST_FILES, more_of)?;
    pull.commits = rest_of(api, MORE_COMMITS, &asked, pull.commits.take(), MOST_COMMITS, more_of)?;
    if let Some(contexts) = contexts_of(&mut pull.rollup) {
        *contexts = rest_of(api, MORE_CHECKS, &asked, contexts.take(), MOST_CHECKS, |found: InRepository<MoreChecks>| {
            Nodes::all(found.pull()?.rollup).next()?.commit?.status_check_rollup?.contexts
        })?;
    }
    Ok(parts_of(pull).facts())
}

/// Where the checks of the last commit are held, to be read on.
fn contexts_of(rollup: &mut Option<Nodes<RollupCommit>>) -> Option<&mut Option<Counted<Context>>> {
    let commit = rollup.as_mut()?.nodes.as_mut()?.first_mut()?.as_mut()?;
    Some(&mut commit.commit.as_mut()?.status_check_rollup.as_mut()?.contexts)
}

/// A later page of a list the query named `more`.
fn more_of<T>(found: InRepository<More<T>>) -> Option<Counted<T>> {
    found.pull()?.more
}

/// A list read on, a page at a time, until the forge has no more or `most` rows are held.
/// `pick` takes the list out of the answer to a later page.
fn rest_of<T, R: DeserializeOwned>(
    api: &GitHubApi,
    query: &str,
    asked: &Asked<'_>,
    first: Option<Counted<T>>,
    most: usize,
    pick: impl Fn(R) -> Option<Counted<T>>,
) -> Result<Option<Counted<T>>, ApiFailure> {
    let Some(mut list) = first else { return Ok(None) };
    loop {
        let held = list.nodes.as_ref().map_or(0, Vec::len);
        let next = list.page_info.take().filter(|it| it.has_next_page == Some(true)).and_then(|it| it.end_cursor);
        let Some(after) = next.filter(|_| held < most) else { return Ok(Some(list)) };
        let found: R = api.graphql(
            query,
            &AskedAfter { owner: asked.owner, name: asked.name, number: asked.number, after: &after },
        )?;
        let Some(more) = pick(found) else { return Ok(Some(list)) };
        list.nodes.get_or_insert_with(Vec::new).extend(more.nodes.unwrap_or_default());
        list.page_info = more.page_info;
    }
}

/// What the API answered, as the parts `gh`'s records fold into, the reduction's choices made
/// here: a draft review is its author's alone, and a review with no body and no verdict says
/// nothing.
pub(crate) fn parts_of(pull: Pull) -> PageParts {
    let mut parts = PageParts {
        head: Some(Head {
            base: pull.base_ref_name.unwrap_or_default(),
            author: login_of(pull.author),
            created_at: pull.created_at.unwrap_or_default(),
            additions: pull.additions.unwrap_or(0),
            deletions: pull.deletions.unwrap_or(0),
            mergeable: mergeability(pull.mergeable.as_deref().unwrap_or_default()),
            review: review_standing(pull.review_decision.as_deref().unwrap_or_default()),
            commit: pull.head_ref_oid.unwrap_or_default(),
            body: pull.body.unwrap_or_default(),
        }),
        ..PageParts::default()
    };
    for verdict in Nodes::all(pull.latest_reviews) {
        let (login, state) = (login_of(verdict.author), verdict.state.unwrap_or_default());
        if state == "APPROVED" {
            parts.approvers.insert(login.clone());
        }
        if state != "PENDING" {
            parts.verdicts.insert(login, review_said(&state));
        }
    }
    for request in Nodes::all(pull.review_requests) {
        let Some(who) = request.requested_reviewer else { continue };
        let kind = if who.kind.as_deref() == Some("Team") { ReviewerKind::Team } else { ReviewerKind::Person };
        let name = who.login.or(who.slug).or(who.name).unwrap_or_default();
        if !name.is_empty() {
            parts.requested.push((kind, name));
        }
    }
    let (comments, unread_comments) = counted(pull.comments);
    parts.said.extend(comments.into_iter().map(|it| Said::Comment {
        author: login_of(it.author),
        at: it.created_at.unwrap_or_default(),
        body: it.body.unwrap_or_default(),
    }));
    let (reviews, unread_reviews) = counted(pull.reviews);
    parts.said.extend(reviews.into_iter().filter_map(|it| {
        let (state, body) = (it.state.unwrap_or_default(), it.body.unwrap_or_default());
        let says = !body.is_empty() || state == "APPROVED" || state == "CHANGES_REQUESTED";
        (state != "PENDING" && says).then(|| Said::Review {
            author: login_of(it.author),
            at: it.submitted_at.unwrap_or_default(),
            verdict: review_said(&state),
            body,
        })
    }));
    let contexts = Nodes::all(pull.rollup)
        .next()
        .and_then(|it| it.commit?.status_check_rollup?.contexts);
    let (checks, unread_checks) = counted(contexts);
    parts.checks.extend(checks.into_iter().map(check_of));
    let (files, unread_files) = counted(pull.files);
    parts.files.extend(files.into_iter().map(|it| PageFile {
        path: it.path.unwrap_or_default(),
        additions: it.additions.unwrap_or(0),
        deletions: it.deletions.unwrap_or(0),
        change: file_change(it.change_type.as_deref().unwrap_or_default()),
    }));
    let (commits, unread_commits) = counted(pull.commits);
    parts.commits.extend(commits.into_iter().filter_map(|it| it.commit).map(|it| {
        let author = Nodes::all(it.authors).next().map(|it| login_of(it.user)).unwrap_or_default();
        let at = it.committed_date.or(it.authored_date).unwrap_or_default();
        CommitLine {
            sha: it.oid.unwrap_or_default().chars().take(7).collect(),
            headline: it.message_headline.unwrap_or_default(),
            author: (!author.is_empty()).then_some(author),
            at: (!at.is_empty()).then_some(at),
        }
    }));
    parts.unread = Omitted {
        conversation: unread_comments + unread_reviews,
        files: unread_files,
        commits: unread_commits,
        checks: unread_checks,
    };
    parts
}

/// A list's rows, and how many more the forge holds than were read.
fn counted<T>(list: Option<Counted<T>>) -> (Vec<T>, usize) {
    let Some(list) = list else { return (Vec::new(), 0) };
    let rows: Vec<T> = list.nodes.unwrap_or_default().into_iter().flatten().collect();
    let unread = list.total_count.map_or(0, |total| total.saturating_sub(rows.len()));
    (rows, unread)
}

/// A check run carries its own words; a status context's `state` stands where a conclusion
/// would. Seconds only for a finished run with both instants.
fn check_of(it: Context) -> CheckRow {
    let status = it.status.unwrap_or_default();
    let conclusion = it.conclusion.or(it.state).unwrap_or_default();
    let seconds = match (status.as_str(), &it.started_at, &it.completed_at) {
        ("COMPLETED", Some(started), Some(completed)) => {
            let at = |text: &String| Timestamp::from_rfc3339(text.clone()).epoch_millis();
            match (at(started), at(completed)) {
                (Some(started), Some(completed)) => u64::try_from((completed - started) / 1_000).ok(),
                _ => None,
            }
        }
        _ => None,
    };
    let address = it.details_url.or(it.target_url).filter(|it| !it.is_empty());
    CheckRow {
        standing: check_standing(&status, &conclusion),
        seconds,
        address,
        name: it.name.or(it.context).unwrap_or_default(),
    }
}
