//! The forge's lists of pull requests, over GitHub's API against `gh`: the open ones, the watch,
//! the newest merged, the merge queue and the account behind the token. A `.json` in
//! `github_answers/` is what GitHub answered and the `.tsv` beside it what `gh`'s own `--jq`
//! printed for the same pull requests, run here with `jq`. All of them are made up.

use adapter_traits::{CiState, Forge, PullMergeable, PullQueue, PullReview, QueueState};

use super::api_forge_reads::{lines, not_found, on_github};
use super::github_api::{api, ok, serve, Reply};
use crate::main_ci::{merged_of_line, newest_first, open_pull_of_line};
use crate::merge_queue::entry_of_line;
use crate::pull_watch::pull_of_line;
use crate::ApiForge;

const OPEN: &str = include_str!("github_answers/open_pulls.json");
const OPEN_ONE: &str = include_str!("github_answers/open_pull_one.json");
const OPEN_GH: &str = include_str!("github_answers/open_pulls.tsv");
const MERGED: &str = include_str!("github_answers/recently_merged.json");
const MERGED_GH: &str = include_str!("github_answers/recently_merged.tsv");
const WATCH: &str = include_str!("github_answers/watch.json");
const WATCH_ONE: &str = include_str!("github_answers/watch_one.json");
const WATCH_GH: &str = include_str!("github_answers/watch.tsv");
const QUEUE: &str = include_str!("github_answers/merge_queue.json");
const QUEUE_GH: &str = include_str!("github_answers/merge_queue.tsv");

fn refused(status: u16) -> Reply {
    Reply { status, headers: Vec::new(), body: r#"{"message":"Server Error"}"#.to_string() }
}

#[test]
fn the_open_pull_requests_over_the_api_are_the_ones_gh_lists() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("pullRequests(states:OPEN") => ok(OPEN),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let pulls = forge.open_pull_requests(&repo.root_str(), "ci").unwrap();
    let expected: Vec<_> = lines(OPEN_GH).iter().filter_map(|line| open_pull_of_line(line, "ci")).collect();
    assert_eq!(pulls, expected);
    assert_eq!(pulls.len(), 3);
    assert_eq!(pulls[0].title.as_written(), "Tab\there", "a tab in a title is a tab");
    assert_eq!(pulls[0].ci, Some(CiState::Passed), "ci gates where there is one");
    assert_eq!(pulls[1].ci, None);
    assert_eq!(pulls[1].facts.mergeable, Some(PullMergeable::Conflicting));
    assert_eq!(pulls[2].ci, Some(CiState::Passed));
    assert_eq!(pulls[2].failing.len(), 1, "lint failed, though ci passed");
    assert_eq!(pulls[2].facts.review, Some(PullReview::Approved));
    assert!(pulls[2].auto_merge);
    assert_eq!(server.seen().len(), 1);
}

#[test]
fn a_listing_the_forge_will_not_answer_is_asked_again_for_fewer_over_the_api_too() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"first\":100") => refused(504),
        body if body.contains("\"first\":25") => ok(OPEN),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let pulls = forge.open_pull_requests(&repo.root_str(), "ci").unwrap();
    assert_eq!(pulls.len(), 3);
    assert_eq!(server.seen().len(), 2);

    let down = serve(|_| refused(504));
    let forge = ApiForge::through(api(&down.url, &["t"]).0);
    assert_eq!(forge.open_pull_requests(&repo.root_str(), "ci"), None);
    assert_eq!(down.seen().len(), 2, "both sizes were asked");
}

#[test]
fn one_open_pull_request_over_the_api_is_the_one_gh_reads_and_a_closed_one_is_none() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"number\":2190") => ok(OPEN_ONE),
        body if body.contains("\"number\":2189") => ok(&OPEN_ONE.replace("\"OPEN\"", "\"CLOSED\"")),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let pull = forge.open_pull_request(&root, "ci", 2190).unwrap();
    assert_eq!(Some(pull), open_pull_of_line(&lines(OPEN_GH)[2], "ci"));
    assert_eq!(forge.open_pull_request(&root, "ci", 2189), None, "closed since");
    assert_eq!(forge.open_pull_request(&root, "ci", 1), None);
}

#[test]
fn the_newest_merged_over_the_api_are_the_ones_gh_lists_newest_first() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("pullRequests(states:MERGED") => ok(MERGED),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let merged = forge.recently_merged_pull_requests(&repo.root_str(), "main", 30).unwrap();
    assert_eq!(merged, newest_first(lines(MERGED_GH).iter().filter_map(|line| merged_of_line(line)).collect()));
    let numbers: Vec<u64> = merged.iter().map(|it| it.number).collect();
    assert_eq!(numbers, [2700, 2703, 2701], "newest merge first, and one never merged is not listed");
    assert!(merged[0].commit.is_some() && merged[1].commit.is_none(), "an oid that is no commit names none");
    assert_eq!(merged[2].author, None);
    let asked = &server.seen()[0].body;
    assert!(asked.contains(r#""base":"main""#) && asked.contains(r#""first":30"#), "{asked}");
    assert_eq!(forge.recently_merged_pull_requests(&repo.root_str(), "--main", 30), None, "an option is not a branch");
}

#[test]
fn the_watch_over_the_api_is_the_watch_gh_reads() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("pullRequests(states:OPEN") => ok(WATCH),
        body if body.contains("pullRequest(number:$number)") && body.contains("\"number\":2190") => ok(WATCH_ONE),
        body if body.contains("pullRequest(number:$number)") && body.contains("\"number\":2189") => {
            ok(&WATCH_ONE.replace("\"OPEN\"", "\"MERGED\""))
        }
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let watched = forge.pull_watch(&root, "ci").unwrap();
    let expected: Vec<_> = lines(WATCH_GH).iter().filter_map(|line| pull_of_line(line, "ci")).collect();
    assert_eq!(watched, expected);
    let first = &watched[0];
    assert_eq!(first.queue, PullQueue::Unmergeable);
    assert!(first.conflicting && first.auto_merge);
    let hung: Vec<(&str, bool)> = first.checks.iter().map(|it| (it.name.as_written(), it.hung)).collect();
    assert_eq!(
        hung,
        [("rust_test", true), ("ci", false), ("quick", false), ("deploy/preview", false), ("app_smoke", false)],
        "only the job stopped by the limit is hung"
    );
    assert_eq!(first.checks[3].log_url.as_ref().map(|it| it.as_written()), Some("https://preview.example/1"));
    assert_eq!(first.title.as_ref().map(|it| it.as_written()), Some("Read the forge\tover the API"));
    assert!(watched[1].head.is_none() && watched[1].title.is_none() && watched[1].author.is_none());

    assert_eq!(forge.watched_pull(&root, "ci", 2190).as_ref(), Some(first));
    assert_eq!(forge.watched_pull(&root, "ci", 2189), None, "merged since");
}

#[test]
fn the_merge_queue_over_the_api_is_the_queue_gh_reads_in_position_order() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("mergeQueue(branch:$branch)") && body.contains(r#""branch":"main""#) => ok(QUEUE),
        body if body.contains("mergeQueue(branch:$branch)") => {
            ok(r#"{"data":{"repository":{"mergeQueue":null}}}"#)
        }
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let queue = forge.merge_queue(&root, "main").unwrap();
    let mut expected: Vec<_> = lines(QUEUE_GH).iter().filter_map(|line| entry_of_line(line)).collect();
    expected.sort_by_key(|entry| entry.position);
    assert_eq!(queue, expected);
    let seen: Vec<(u64, u32, QueueState)> = queue.iter().map(|it| (it.number, it.position, it.state)).collect();
    assert_eq!(
        seen,
        [(20, 1, QueueState::AwaitingChecks), (21, 2, QueueState::Queued), (22, 3, QueueState::InQueue)]
    );
    assert_eq!(forge.merge_queue(&root, "other"), Some(Vec::new()), "a branch with no queue has none waiting");
    assert_eq!(forge.merge_queue(&root, "--main"), None);
}

#[test]
fn the_login_a_token_belongs_to_is_read_over_the_api() {
    let server = serve(|seen| match seen.target.as_str() {
        "/user" => ok(r#"{"login":"nick-mele","id":7,"type":"User"}"#),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    assert_eq!(forge.forge_login(&repo.root_str()).as_deref(), Some("nick-mele"));

    let nobody = serve(|_| ok(r#"{"login":""}"#));
    let forge = ApiForge::through(api(&nobody.url, &["t"]).0);
    assert_eq!(forge.forge_login(&repo.root_str()), None);
}
