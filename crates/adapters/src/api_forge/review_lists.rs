//! One list of pull requests waiting on a person, over GitHub's API: the search `gh` asks, read
//! into the rows `crate::pull_review_lists` folds from `gh`'s records.

use adapter_traits::{ListedPull, ReviewList, ReviewListAsk};
use serde::{Deserialize, Serialize};

use super::{joined, login_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi};
use crate::pull_request_detail::review_standing;
use crate::pull_review_lists::{checks_of, search_of, team_named, Teams, MOST_A_CALL, QUERY};

/// The most pages of the person's teams read: 1,000 teams.
const MOST_TEAM_PAGES: usize = 10;

#[derive(Serialize)]
struct Asked<'a> {
    q: &'a str,
    n: usize,
}

/// `data` of the search.
#[derive(Deserialize)]
pub(crate) struct Searched {
    search: Option<Nodes<Found>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
    repository: Option<Repository>,
    number: Option<u64>,
    title: Option<String>,
    head_ref_name: Option<String>,
    url: Option<String>,
    is_draft: Option<bool>,
    head_ref_oid: Option<String>,
    author: Option<Login>,
    updated_at: Option<String>,
    review_decision: Option<String>,
    commits: Option<Nodes<Commit>>,
    review_requests: Option<Nodes<Request>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    name_with_owner: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    commit: Option<Rollup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rollup {
    status_check_rollup: Option<State>,
}

#[derive(Deserialize)]
struct State {
    state: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    requested_reviewer: Option<Team>,
}

/// A team asked; a person asked carries none of these, the search asking only for teams.
#[derive(Deserialize)]
struct Team {
    name: Option<String>,
    slug: Option<String>,
    organization: Option<Login>,
}

/// One of `user/teams`.
#[derive(Deserialize)]
struct HeldTeam {
    slug: String,
    organization: Login,
}

/// One list. `Ok(None)` where the search would not mean what it says, as for `gh`.
pub(super) fn read(api: &GitHubApi, ask: &ReviewListAsk<'_>) -> Result<Option<Vec<ListedPull>>, ApiFailure> {
    let Some(search) = search_of(ask) else { return Ok(None) };
    let asked = Asked { q: &search, n: ask.most.clamp(1, MOST_A_CALL) };
    if ask.list != ReviewList::Team {
        let found: Searched = api.graphql(QUERY, &asked)?;
        return Ok(Some(listed(found, Teams::Unwanted)));
    }
    // The teams the person is on do not wait on the search, nor it on them.
    let (found, held) = std::thread::scope(|scope| {
        let held = scope.spawn(|| my_teams(api));
        let found: Result<Searched, ApiFailure> = api.graphql(QUERY, &asked);
        (found, joined(held))
    });
    let held = held.ok();
    let teams = held.as_deref().map_or(Teams::Unread, Teams::Held);
    Ok(Some(listed(found?, teams)))
}

/// The teams the person is on, as lowercase `org/slug`. A refusal is a token without `read:org`,
/// and the list carries on without them. A list cut at the cap names the first team asked for
/// a team it did not reach, which is what an unread list does for all of them.
fn my_teams(api: &GitHubApi) -> Result<Vec<String>, ApiFailure> {
    let paged = api.get_paged("/user/teams?per_page=100", MOST_TEAM_PAGES)?;
    let mut teams = Vec::new();
    for page in &paged.pages {
        let held: Vec<HeldTeam> = page.decode("the teams a person is on")?;
        teams.extend(held.into_iter().filter_map(|team| {
            Some(format!("{}/{}", team.organization.login?, team.slug).to_lowercase())
        }));
    }
    Ok(teams)
}

/// The search's rows, as `crate::pull_review_lists::listed_folded` makes them from `gh`'s. A
/// node with no number is not a pull request, and is dropped.
pub(crate) fn listed(found: Searched, teams: Teams<'_>) -> Vec<ListedPull> {
    Nodes::all(found.search)
        .filter_map(|pull| {
            let number = pull.number?;
            let rollup = Nodes::all(pull.commits)
                .next()
                .and_then(|it| it.commit?.status_check_rollup?.state)
                .unwrap_or_default();
            let asked: Vec<(String, String)> = Nodes::all(pull.review_requests)
                .filter_map(|request| {
                    let team = request.requested_reviewer?;
                    let slug = team.slug.filter(|slug| !slug.is_empty())?;
                    let org = login_of(team.organization);
                    Some((format!("{org}/{slug}"), team.name.unwrap_or_else(|| slug.clone())))
                })
                .collect();
            Some(ListedPull {
                repository: pull.repository.and_then(|it| it.name_with_owner).unwrap_or_default(),
                number,
                title: pull.title.unwrap_or_default(),
                branch: pull.head_ref_name.unwrap_or_default(),
                address: pull.url.unwrap_or_default(),
                draft: pull.is_draft.unwrap_or(false),
                head: pull.head_ref_oid.unwrap_or_default(),
                author: login_of(pull.author),
                updated_at: pull.updated_at.unwrap_or_default(),
                checks: checks_of(&rollup),
                review: review_standing(&pull.review_decision.unwrap_or_default()),
                team: team_named(&asked, &teams),
            })
        })
        .collect()
}
