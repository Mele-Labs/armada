//! A repository's issues over GitHub's API against `gh`: the open ones, one by number, a linked
//! one's text and a kit source's files. A `.json` in `github_answers/` is what GitHub answered
//! and the `.tsv` beside it what `gh`'s own `--jq` printed for the same issues, run here with
//! `jq`. All of them are made up.

use adapter_traits::{FromOutside, IssueField, LinkedPullRequestState, TrackerChoice, Trackers};

use super::api_forge_reads::{lines, not_found, on_github};
use super::github_api::{api, ok, serve, Reply};
use crate::api_forge::source;
use crate::issue_tracker::{issue_of_line, read_issue_of_records};
use crate::ApiForge;

const FIRST: &str = include_str!("github_answers/issues_open_first.json");
const NEXT: &str = include_str!("github_answers/issues_open_next.json");
const OPEN_GH: &str = include_str!("github_answers/issues_open.tsv");
const READ: &str = include_str!("github_answers/issue_read.json");
const READ_GH: &str = include_str!("github_answers/issue_read.tsv");

fn trackers(server_url: &str) -> impl Trackers {
    ApiForge::through(api(server_url, &["t"]).0).trackers()
}

fn graphql_refused(message: &str) -> Reply {
    let body = format!(r#"{{"data":{{"repository":{{"issue":null}}}},"errors":[{{"type":"NOT_FOUND","message":"{message}"}}]}}"#);
    Reply { status: 200, headers: Vec::new(), body }
}

#[test]
fn the_open_issues_over_the_api_are_the_issues_gh_lists_newest_edit_first() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"after\":\"I1\"") => ok(NEXT),
        body if body.contains("issues(states:OPEN") => ok(FIRST),
        _ => not_found(),
    });
    let repo = on_github();
    let tracker = trackers(&server.url).tracker(&TrackerChoice::of_the_forge());

    let issues = tracker.open_issues(&repo.root_str()).unwrap();
    let mut expected: Vec<_> = lines(OPEN_GH).iter().filter_map(|line| issue_of_line(line)).collect();
    expected.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    assert_eq!(issues, expected);
    let numbers: Vec<&str> = issues.iter().map(|it| it.reference.as_str()).collect();
    assert_eq!(numbers, ["#2050", "#2099", "#2101"], "the page after the first is read, and the newest edit is first");
    assert!(issues[2].linked_pull_request && !issues[0].linked_pull_request);
    assert_eq!(issues[2].excerpt.as_written().chars().count(), 240, "the excerpt is cut at 240 characters");
    assert!(issues[2].excerpt.as_written().starts_with(" It says running. - one - two with a tab And"));
    assert_eq!(issues[1].title.as_written(), "Tab\there");
    assert_eq!(server.seen().len(), 2);
}

#[test]
fn an_issue_over_the_api_is_the_issue_gh_reads_with_its_comments_and_pull_requests() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"number\":214") => ok(READ),
        body if body.contains("\"number\":999") => graphql_refused("Could not resolve to an Issue with the number of 999."),
        body if body.contains("\"number\":998") => Reply { status: 404, headers: Vec::new(), body: r#"{"message":"Not Found"}"#.to_string() },
        body if body.contains("\"number\":997") => graphql_refused("Something else went wrong."),
        _ => not_found(),
    });
    let repo = on_github();
    let root = repo.root_str();
    let tracker = trackers(&server.url).tracker(&TrackerChoice::of_the_forge());

    let issue = tracker.read_issue(&root, "#214").unwrap().unwrap();
    assert_eq!(Some(issue.clone()), read_issue_of_records(READ_GH));
    assert!(issue.closed);
    assert_eq!(issue.comments.len(), 2);
    let pulls: Vec<(u64, LinkedPullRequestState)> =
        issue.linked_pull_requests.iter().map(|it| (it.number, it.state)).collect();
    assert_eq!(
        pulls,
        [
            (300, LinkedPullRequestState::Merged),
            (301, LinkedPullRequestState::Open),
            (302, LinkedPullRequestState::Closed)
        ],
        "one that closes it and names it is listed once, and a reference from an issue is none"
    );
    assert_eq!(
        issue.fields,
        [
            field("Labels", &["bug", "armada: in progress"]),
            field("Milestone", &["v2"]),
            field("Projects", &["Roadmap: Done"]),
        ]
    );
    assert_eq!(tracker.read_issue(&root, "214").unwrap(), Some(issue));

    // No issue by that number is none, as `gh`'s words for it are; anything else is the tracker's.
    assert_eq!(tracker.read_issue(&root, "999"), Ok(None));
    assert_eq!(tracker.read_issue(&root, "998"), Ok(None));
    let refused = tracker.read_issue(&root, "997").unwrap_err();
    assert_eq!(refused.doing, "reading the issue");
    assert!(refused.said.contains("Something else went wrong."), "{}", refused.said);
    let asked = server.seen().len();
    assert_eq!(tracker.read_issue(&root, "text that is no number"), Ok(None));
    assert_eq!(server.seen().len(), asked, "text that is no number asks nothing");
}

#[test]
fn a_linked_issues_text_is_read_over_the_api_and_a_link_that_is_no_issue_is_not() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"number\":214") && body.contains("issue(number:$number){title body}") => ok(
            r#"{"data":{"repository":{"issue":{"title":"Doctor reports a stale pid","body":"It says running.\n\nTwo machines.\n"}}}}"#,
        ),
        body if body.contains("\"number\":5") => ok(r#"{"data":{"repository":{"issue":{"title":"","body":null}}}}"#),
        _ => graphql_refused("Could not resolve to an Issue with the number of 1."),
    });
    let links = trackers(&server.url).links(&TrackerChoice::of_the_forge());

    let read = links.read_link("Look at https://github.com/Mele-Labs/armada/issues/214#issuecomment-1 please");
    assert_eq!(read, Some(Ok("Doctor reports a stale pid\n\nIt says running.\n\nTwo machines.".to_string())));
    assert_eq!(links.read_link("https://github.com/Mele-Labs/armada/issues/5"), Some(Err("the lookup said nothing".to_string())));
    assert!(matches!(links.read_link("https://github.com/Mele-Labs/armada/issues/1"), Some(Err(_))));
    let asked = server.seen().len();
    assert_eq!(links.read_link("https://github.com/Mele-Labs/armada/pull/214"), None, "a pull request is no issue link");
    assert_eq!(links.read_link("nothing to read here"), None);
    assert_eq!(server.seen().len(), asked);
    // The call a Fleet without the API would run is still rendered.
    assert!(links.resolve("https://github.com/Mele-Labs/armada/issues/214").is_some());
}

fn field(name: &str, values: &[&str]) -> IssueField {
    IssueField { name: name.to_string(), values: values.iter().map(|one| FromOutside::verbatim(*one)).collect() }
}

const SEARCH: &str = r#"{"data":{"search":{"nodes":[
 {"number":12,"title":"Older","url":"https://github.com/Mele-Labs/armada/issues/12","state":"CLOSED",
  "author":{"login":"nick"},"labels":{"nodes":[{"name":"bug"}]},"assignees":{"nodes":[{"login":"sam"}]},
  "updatedAt":"2026-10-18T10:00:00Z","body":"It  drops\none","closedByPullRequestsReferences":{"nodes":[{"number":40}]},
  "milestone":{"title":"v2"},
  "projectItems":{"nodes":[{"project":{"title":"Roadmap"},"fieldValueByName":{"name":"In progress"}},
                           {"project":{"title":"Backlog"},"fieldValueByName":null},
                           {"project":{"title":"Dates"},"fieldValueByName":{}}]}},
 {},
 {"number":13,"title":"Newer","url":"https://github.com/Mele-Labs/armada/issues/13","state":"OPEN","author":null,
  "labels":{"nodes":[]},"assignees":{"nodes":[]},"updatedAt":"2026-10-20T10:00:00Z","body":"",
  "closedByPullRequestsReferences":{"nodes":[]},"milestone":null,"projectItems":{"nodes":[]}}]}}}"#;

#[test]
fn a_search_over_the_api_asks_for_the_query_among_the_repositorys_issues_and_reads_every_field() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("search(query:$q,type:ISSUE,first:$first)") => ok(SEARCH),
        _ => not_found(),
    });
    let repo = on_github();
    let tracker = trackers(&server.url).tracker(&TrackerChoice::of_the_forge());

    let found = tracker.search_issues(&repo.root_str(), "is:closed label:bug").unwrap();
    let seen = server.seen();
    let asked = &seen[0].body;
    assert!(
        asked.contains(r#""q":"repo:mele-labs/armada is:issue is:closed label:bug sort:updated-desc""#),
        "{asked}"
    );
    assert!(asked.contains(r#""first":100"#) && asked.contains("projectItems"), "{asked}");
    let references: Vec<&str> = found.iter().map(|it| it.reference.as_str()).collect();
    assert_eq!(references, ["#13", "#12"], "newest edit first, and a node that is no issue is none");
    let full = &found[1];
    assert_eq!(full.author.as_written(), "nick");
    assert!(full.closed && full.linked_pull_request);
    assert_eq!(full.excerpt.as_written(), "It drops one");
    assert_eq!(
        full.fields,
        [
            field("Labels", &["bug"]),
            field("Milestone", &["v2"]),
            field("Projects", &["Roadmap: In progress", "Backlog", "Dates"]),
        ]
    );
    assert!(!found[0].closed && found[0].author.as_written().is_empty() && found[0].fields.is_empty());
}

#[test]
fn a_token_that_may_not_read_projects_still_searches_without_them() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("projectItems") => Reply {
            status: 200,
            headers: Vec::new(),
            body: r#"{"data":null,"errors":[{"type":"INSUFFICIENT_SCOPES","message":"Your token has not been granted the required scopes to execute this query. The 'projectItems' field requires one of the following scopes: ['read:project'], but your token has only been granted the: ['repo'] scopes."}]}"#.to_string(),
        },
        body if body.contains("search(query:$q") => ok(&SEARCH.replace(r#""projectItems":{"nodes":[{"project""#, r#""ignored":{"nodes":[{"project""#)),
        _ => not_found(),
    });
    let repo = on_github();
    let tracker = trackers(&server.url).tracker(&TrackerChoice::of_the_forge());

    let found = tracker.search_issues(&repo.root_str(), "label:bug").unwrap();
    assert_eq!(server.seen().len(), 2, "asked once with projects, once without");
    assert_eq!(found[1].fields, [field("Labels", &["bug"]), field("Milestone", &["v2"])]);
}

#[test]
fn a_search_over_the_api_that_is_refused_says_so_in_the_forges_words() {
    let server = serve(|_| graphql_refused("The query is too long."));
    let repo = on_github();
    let tracker = trackers(&server.url).tracker(&TrackerChoice::of_the_forge());
    let refused = tracker.search_issues(&repo.root_str(), "label:bug").unwrap_err();
    assert_eq!(refused.doing, "searching the issues");
    assert!(refused.said.contains("The query is too long."), "{}", refused.said);
    let guide = tracker.query_guide();
    assert_eq!((guide.open.as_str(), guide.assigned_to_me.as_str()), ("is:open", "is:open assignee:@me"));
}

#[test]
fn a_manifests_choice_of_another_tracker_is_kept_whatever_reads_the_forges() {
    let refused = trackers("http://127.0.0.1:9").tracker(&TrackerChoice::named(Some("trello"), &Default::default()));
    assert!(refused.open_issues("/repo").unwrap_err().said.contains("`trello`"));
    let links = trackers("http://127.0.0.1:9").links(&TrackerChoice::named(Some("trello"), &Default::default()));
    assert_eq!(links.read_link("https://github.com/o/r/issues/1"), None);
}

#[test]
fn a_kit_sources_files_are_read_over_the_api() {
    let server = serve(|seen| match seen.target.as_str() {
        "/repos/o/skills/git/trees/HEAD?recursive=1" => ok(
            r#"{"sha":"x","tree":[{"path":"skills","type":"tree"},{"path":"skills/a/SKILL.md","type":"blob"},{"path":"README.md","type":"blob"},{"path":"vendor","type":"commit"}],"truncated":false}"#,
        ),
        "/repos/o/skills/contents/skills/a/SKILL.md" => {
            assert_eq!(seen.accept.as_deref(), Some("application/vnd.github.raw"));
            ok("---\nname: a\n---\n")
        }
        "/repos/o/skills/contents/odd%23name%3F.md" => ok("odd"),
        _ => not_found(),
    });
    let (api, _) = api(&server.url, &["t"]);

    assert_eq!(source::paths(&api, "o/skills").unwrap(), ["skills/a/SKILL.md", "README.md"]);
    assert_eq!(source::file(&api, "o/skills", "skills/a/SKILL.md").unwrap(), b"---\nname: a\n---\n");
    assert_eq!(source::file(&api, "o/skills", "odd#name?.md").unwrap(), b"odd");
    assert!(source::file(&api, "o/skills", "gone.md").is_err());
    assert!(source::paths(&api, "o/none").is_err());
}
