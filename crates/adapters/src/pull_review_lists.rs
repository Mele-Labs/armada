//! The open pull requests waiting on a person, as one list at a time: `api graphql` over a
//! search, reduced by `--jq` to a `@tsv` record a pull request, so the forge's words stop in
//! [`listed_folded`]. `docs/concepts/fleet.md`, *The pull requests waiting on a person*.
//!
//! **One forge call a list for all the repositories**, because the search takes a `repo:`
//! qualifier for each and ORs them. Nothing here polls; the caller reads when somebody looks.

use adapter_traits::{LatestReview, LatestReviewSaid, ListedChecks, ListedPull, ReviewList, ReviewListAsk};

use crate::delivery::{read_in, FORGE};
use crate::pull_request_detail::review_standing;
use crate::under_review::as_written;

/// The most rows the forge's search will give a call.
pub(crate) const MOST_A_CALL: usize = 100;

/// `asks`, read only where `$asked`, is the latest ten review requests of each pull request: enough
/// to reach his among others asked about the same time, and one connection a row as `myReview` is,
/// so a search of 50 stays at GitHub's cost of two points.
pub(crate) const QUERY: &str = "query($q:String!,$n:Int!,$me:String,$asked:Boolean!){search(query:$q,type:ISSUE,first:$n){nodes{\
    ... on PullRequest{repository{nameWithOwner} number title headRefName url isDraft headRefOid \
    author{login} updatedAt reviewDecision \
    commits(last:1){nodes{commit{statusCheckRollup{state}}}} \
    reviewRequests(first:10){nodes{requestedReviewer{... on Team{name slug organization{login}}}}} \
    myReview:reviews(author:$me,last:1,states:[APPROVED,CHANGES_REQUESTED,COMMENTED]){nodes{state submittedAt commit{oid}}} \
    asks:timelineItems(itemTypes:[REVIEW_REQUESTED_EVENT],last:10)@include(if:$asked){nodes{\
    ... on ReviewRequestedEvent{createdAt requestedReviewer{... on User{login} ... on Team{slug organization{login}}}}}}}}}}";

/// One pull request a line: repository, number, title, branch, address, draft, head, author,
/// updated, review decision, rollup state, then the teams asked (RS apart) as `org/slug` and the
/// team's name (US apart), then the person's own latest review's state, commit and submitted time,
/// then each review request read (RS apart) as who was asked and when (US apart): a person by login,
/// a team as `org/slug`.
/// `// ""` on every field so a `null` cannot stop jq mid-stream.
const LINES: &str = concat!(
    r#".data.search.nodes[]? | select((.number // null) != null) | "#,
    r#"[(.repository.nameWithOwner // ""), (.number | tostring), (.title // ""), "#,
    r#"(.headRefName // ""), (.url // ""), ((.isDraft // false) | tostring), "#,
    r#"(.headRefOid // ""), (.author.login // ""), (.updatedAt // ""), "#,
    r#"(.reviewDecision // ""), "#,
    r#"(.commits.nodes[0].commit.statusCheckRollup.state // ""), "#,
    r#"([.reviewRequests.nodes[]? | .requestedReviewer | select((.slug // "") != "") | "#,
    r#""\(.organization.login // "")/\(.slug)\u001f\(.name // .slug)"] | join("\u001e")), "#,
    r#"(.myReview.nodes[0].state // ""), (.myReview.nodes[0].commit.oid // ""), "#,
    r#"(.myReview.nodes[0].submittedAt // ""), "#,
    r#"([.asks.nodes[]? | select((.createdAt // "") != "") | .createdAt as $at | (.requestedReviewer // {}) | "#,
    r#""\(.login // "\(.organization.login // "")/\(.slug // "")")\u001f\($at)"] | join("\u001e"))] | @tsv"#
);

/// The teams the signed-in account is on, as `org/slug`, one to a line.
const TEAMS: &str = r#".[] | "\(.organization.login)/\(.slug)""#;

/// Which teams a row may name.
pub(crate) enum Teams<'a> {
    /// Not the team list: no row names one.
    Unwanted,
    /// The forge would not say which teams the person is on (a token without the scope): the
    /// first team asked is named.
    Unread,
    Held(&'a [String]),
}

/// One list, `None` where the forge would not answer.
pub(crate) fn read(in_repo: &str, ask: &ReviewListAsk<'_>) -> Option<Vec<ListedPull>> {
    let search = search_of(ask)?;
    let (q, n) = (format!("q={search}"), format!("n={}", ask.most.clamp(1, MOST_A_CALL)));
    let query = format!("query={QUERY}");
    let me = format!("me={}", ask.login);
    let asked = format!("asked={}", asks_read(ask.list));
    let mut args = vec!["api", "graphql", "-f", &q, "-F", &n, "-F", &asked, "-f", &query, "--jq", LINES];
    // No login leaves `$me` null, and the open list's reviews are nobody's own.
    if !ask.login.is_empty() && ask.list != ReviewList::Open {
        args.extend(["-f", me.as_str()]);
    }
    let run = read_in(in_repo, FORGE, &args).ok()?;
    if !run.status.success() {
        return None;
    }
    let lines: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    Some(match ask.list {
        ReviewList::Team => {
            let held = my_teams(in_repo);
            let teams = held.as_deref().map_or(Teams::Unread, Teams::Held);
            listed_folded(&lines, teams, ask.login)
        }
        _ => listed_folded(&lines, Teams::Unwanted, ask.login),
    })
}

/// Whether a list reads each pull request's review requests: only the two that ask him.
pub(crate) fn asks_read(list: ReviewList) -> bool {
    matches!(list, ReviewList::Requested | ReviewList::Team)
}

/// The search text, or `None` where it would not mean what it says: **no repository, or a name
/// with a space in it, would widen the search past the repositories Fleet serves**.
pub(crate) fn search_of(ask: &ReviewListAsk<'_>) -> Option<String> {
    let name = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '[' | ']'))
    };
    // The open list names no one, so it needs no login; every other list is the login's.
    let login_ok = ask.list == ReviewList::Open || name(ask.login);
    if ask.repositories.is_empty() || !login_ok || !ask.repositories.iter().all(|it| name(it)) {
        return None;
    }
    let mut search = String::from("is:pr is:open sort:updated-desc");
    for repository in ask.repositories {
        search.push_str(" repo:");
        search.push_str(repository);
    }
    let login = ask.login;
    search.push_str(&match ask.list {
        ReviewList::Requested => format!(" user-review-requested:{login}"),
        ReviewList::Team => format!(" review-requested:{login} -user-review-requested:{login}"),
        ReviewList::Assigned => format!(" assignee:{login}"),
        ReviewList::Mine => format!(" author:{login}"),
        ReviewList::Open => String::new(),
    });
    Some(search)
}

/// The teams the person is on. `None` is the forge's refusal: a token without `read:org` gets
/// one, and the list carries on without it.
fn my_teams(in_repo: &str) -> Option<Vec<String>> {
    let run = read_in(in_repo, FORGE, &["api", "user/teams", "--paginate", "--jq", TEAMS]).ok()?;
    run.status.success().then(|| {
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .map(|line| line.trim().to_lowercase())
            .filter(|line| !line.is_empty())
            .collect()
    })
}

/// The records the reduction printed, as rows. Split from [`read`] so a test hands it what jq
/// prints and needs no forge. A line with no number is dropped. `me` is the login the list is for.
pub(crate) fn listed_folded(lines: &[String], teams: Teams<'_>, me: &str) -> Vec<ListedPull> {
    lines
        .iter()
        .filter_map(|line| {
            let mut field = line.split('\t').map(as_written);
            let mut next = || field.next().unwrap_or_default();
            let repository = next();
            let number = next().parse().ok()?;
            let (title, branch, address, draft) = (next(), next(), next(), next());
            let (head, author, updated_at, decision, rollup, asked) =
                (next(), next(), next(), next(), next(), next());
            let (my_state, my_commit, my_at, asks) = (next(), next(), next(), next());
            Some(ListedPull {
                repository,
                number,
                title,
                branch,
                address,
                draft: draft == "true",
                head,
                author,
                updated_at,
                checks: checks_of(&rollup),
                review: review_standing(&decision),
                team: team_named(&asked_of(&asked), &teams),
                my_latest: latest_of(&my_state, &my_commit, &my_at),
                asked_at: asked_at_of(&asked_of(&asks), me, &teams),
            })
        })
        .collect()
}

/// A person's latest review, from the forge's state word, the commit it was on and when it was
/// submitted. A state that is not a submitted verdict, as a pending or dismissed one, is none.
pub(crate) fn latest_of(state: &str, commit: &str, at: &str) -> Option<LatestReview> {
    let said = match state {
        "APPROVED" => LatestReviewSaid::Approved,
        "CHANGES_REQUESTED" => LatestReviewSaid::ChangesRequested,
        "COMMENTED" => LatestReviewSaid::Commented,
        _ => return None,
    };
    let named = |it: &str| (!it.is_empty()).then(|| it.to_string());
    Some(LatestReview { said, commit: named(commit), submitted_at: named(at) })
}

/// The teams asked, as the reduction joins them: `org/slug` US name, RS apart. The review requests
/// read are joined the same way, as who was asked US when.
fn asked_of(joined: &str) -> Vec<(String, String)> {
    joined
        .split('\u{1e}')
        .filter(|one| !one.is_empty())
        .map(|one| one.split_once('\u{1f}').unwrap_or((one, one)))
        .map(|(path, name)| (path.to_string(), name.to_string()))
        .collect()
}

/// A rollup that passed passed, one that failed or errored failed, and anything else, including
/// none at all, is still running.
pub(crate) fn checks_of(rollup: &str) -> ListedChecks {
    match rollup {
        "SUCCESS" => ListedChecks::Passed,
        "FAILURE" | "ERROR" => ListedChecks::Failed,
        _ => ListedChecks::Running,
    }
}

/// The first team asked, as `org/slug` and its name, that the person is on; failing that, the
/// first asked.
pub(crate) fn team_named(requested: &[(String, String)], teams: &Teams<'_>) -> Option<String> {
    if matches!(teams, Teams::Unwanted) {
        return None;
    }
    let on_it = |path: &str| match teams {
        Teams::Held(held) => held.iter().any(|it| it.eq_ignore_ascii_case(path)),
        _ => false,
    };
    requested
        .iter()
        .find(|(path, _)| on_it(path))
        .or(requested.first())
        .map(|(_, name)| name.to_string())
}

/// When the latest of `requests`, each who was asked and when, asked the person: by login `me` off
/// the team list, and of a team they are on on it (any team, where the forge would not say which).
/// A person's login holds no `/` and a team's `org/slug` does.
pub(crate) fn asked_at_of(requests: &[(String, String)], me: &str, teams: &Teams<'_>) -> Option<String> {
    let names_them = |who: &str| match (who.split_once('/'), teams) {
        (None, Teams::Unwanted) => !me.is_empty() && who.eq_ignore_ascii_case(me),
        (Some((org, slug)), Teams::Held(held)) if !org.is_empty() && !slug.is_empty() => {
            held.iter().any(|it| it.eq_ignore_ascii_case(who))
        }
        (Some((org, slug)), Teams::Unread) => !org.is_empty() && !slug.is_empty(),
        _ => false,
    };
    // The forge writes every `createdAt` in UTC with a `Z`, so the latest is the greatest text.
    requests
        .iter()
        .filter(|(who, at)| !at.is_empty() && names_them(who))
        .map(|(_, at)| at)
        .max()
        .cloned()
}
