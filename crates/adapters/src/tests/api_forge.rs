//! The forge over GitHub's API against the forge through `gh`: each read moved to the API,
//! answered by a socket on this machine replaying what GitHub answered, gives the value `gh`'s
//! records fold to for the same pull request. `github_answers/` holds both: a `.json` is the API's
//! answer, a `.tsv` what `gh`'s reduction printed for it. `page_2190`, `patches_2190` and
//! `review_list` are this repository's own, trimmed; `*_reviewed` are made up, because nothing
//! here has been reviewed on the forge.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use adapter_traits::{FilePatch, Forge, ReviewList, ReviewListAsk};

use super::github_api::{api, ok, serve, Reply};
use super::repo::TempRepo;
use crate::github_api::ApiFailure;
use crate::pull_request_detail::{folded, profile_from, with_profiles, with_teams};
use crate::pull_request_patch::listed;
use crate::pull_request_threads::threads_folded;
use crate::pull_review_lists::{listed_folded, Teams};
use crate::ApiForge;

const LIST: &str = include_str!("github_answers/review_list.json");
const LIST_GH: &str = include_str!("github_answers/review_list.tsv");
const PAGE: &str = include_str!("github_answers/page_2190.json");
const PAGE_GH: &str = include_str!("github_answers/page_2190.tsv");
const REVIEWED: &str = include_str!("github_answers/page_reviewed.json");
const REVIEWED_MORE_FILES: &str = include_str!("github_answers/page_reviewed_more_files.json");
const REVIEWED_GH: &str = include_str!("github_answers/page_reviewed.tsv");
const THREADS: &str = include_str!("github_answers/threads_reviewed.json");
const THREADS_GH: &str = include_str!("github_answers/threads_reviewed.tsv");
const PATCHES: &str = include_str!("github_answers/patches_2190.json");
const PATCHES_GH: &str = include_str!("github_answers/patches_2190.tsv");
/// What GitHub answers for the threads of a pull request nobody has commented a line of.
const NO_THREADS: &str = r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[]}}}}}"#;

fn lines(tsv: &str) -> Vec<String> {
    tsv.lines().filter(|line| !line.is_empty()).map(str::to_string).collect()
}

fn not_found() -> Reply {
    Reply { status: 404, headers: Vec::new(), body: r#"{"message":"Not Found"}"#.to_string() }
}

fn checkout(remote: &str) -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.git(&["remote", "add", "origin", remote]);
    repo
}

fn on_github() -> TempRepo {
    checkout("https://github.com/Mele-Labs/armada.git")
}

/// Requests that answer only once `n` of them are in, or after a while alone: read one after
/// another, the first waits it out and is refused.
struct Together {
    arrived: Mutex<usize>,
    all_in: Condvar,
    n: usize,
}

impl Together {
    fn of(n: usize) -> Arc<Together> {
        Arc::new(Together { arrived: Mutex::new(0), all_in: Condvar::new(), n })
    }

    fn arrive(&self) -> bool {
        let mut arrived = self.arrived.lock().unwrap();
        *arrived += 1;
        self.all_in.notify_all();
        let (arrived, _) = self
            .all_in
            .wait_timeout_while(arrived, Duration::from_secs(3), |arrived| *arrived < self.n)
            .unwrap();
        *arrived >= self.n
    }
}

fn together(group: &Together, reply: Reply) -> Reply {
    if group.arrive() {
        reply
    } else {
        Reply { status: 503, headers: Vec::new(), body: r#"{"message":"asked alone"}"#.to_string() }
    }
}

#[test]
fn a_review_list_over_the_api_is_the_list_gh_reads() {
    let teams = Together::of(2);
    let server = serve(move |seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("POST", "/graphql") if seen.body.contains("-user-review-requested") => together(&teams, ok(LIST)),
        ("POST", "/graphql") => ok(LIST),
        ("GET", "/user/teams?per_page=100") => {
            together(&teams, ok(r#"[{"slug":"platform","organization":{"login":"Mele-Labs"}}]"#))
        }
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let repositories = vec!["mele-labs/armada".to_string()];
    let ask = |list| ReviewListAsk { list, login: "NickMele", repositories: &repositories, most: 50 };

    let requested = forge.pull_review_list(&repo.root_str(), &ask(ReviewList::Requested)).unwrap();
    assert_eq!(requested, listed_folded(&lines(LIST_GH), Teams::Unwanted));
    assert_eq!(requested.len(), 6, "the node that is no pull request is dropped");

    // The person is on Platform, the second team asked of #2712, and on no team asked of #2711.
    let team = forge.pull_review_list(&repo.root_str(), &ask(ReviewList::Team)).unwrap();
    let held = vec!["mele-labs/platform".to_string()];
    assert_eq!(team, listed_folded(&lines(LIST_GH), Teams::Held(&held)));
    let named: Vec<Option<&str>> = team.iter().map(|row| row.team.as_deref()).collect();
    assert_eq!(named, vec![None, None, Some("Platform"), Some("Reviewers"), None, None]);

    let searched = &server.seen()[0].body;
    assert!(searched.contains(r#""q":"is:pr is:open sort:updated-desc repo:mele-labs/armada user-review-requested:NickMele""#), "{searched}");
}

#[test]
fn a_page_over_the_api_is_the_page_gh_reads() {
    let server = serve(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("POST", "/graphql") if seen.body.contains("reviewThreads") => ok(NO_THREADS),
        ("POST", "/graphql") if seen.body.contains("baseRefName") => ok(PAGE),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let page = forge.pull_request_page(&repo.root_str(), "2190").unwrap();
    assert_eq!(page, folded(&lines(PAGE_GH)).unwrap());
    let read = (page.conversation.len(), page.checks.len(), page.files.len(), page.commits.len());
    assert_eq!(read, (2, 4, 7, 4));
}

#[test]
fn a_reviewed_page_over_the_api_is_the_page_gh_reads_with_its_people_read_at_once() {
    // The page beside its threads; then three profiles and a team's people beside each other.
    let (first, people) = (Together::of(2), Together::of(4));
    let server = serve(move |seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("POST", "/graphql") if seen.body.contains("more:files") => ok(REVIEWED_MORE_FILES),
        ("POST", "/graphql") if seen.body.contains("reviewThreads") => together(&first, ok(THREADS)),
        ("POST", "/graphql") if seen.body.contains("baseRefName") => together(&first, ok(REVIEWED)),
        ("GET", "/users/grace") => {
            together(&people, ok(r#"{"name":"Grace Hopper","html_url":"https://github.com/grace"}"#))
        }
        ("GET", "/users/linus") => together(&people, ok(r#"{"name":null,"html_url":"https://github.com/linus"}"#)),
        ("GET", "/users/barbara") => together(&people, not_found()),
        ("GET", "/orgs/mele-labs/teams/reviewers/members?per_page=100") => {
            together(&people, ok(r#"[{"login":"grace"}]"#))
        }
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let page = forge.pull_request_page(&repo.root_str(), "7").unwrap();
    let mut expected = folded(&lines(REVIEWED_GH)).unwrap();
    expected.threads = threads_folded(&lines(THREADS_GH));
    with_profiles(&mut expected.reviewers, |login| match login {
        "grace" => profile_from("Grace Hopper\thttps://github.com/grace"),
        "linus" => profile_from("\thttps://github.com/linus"),
        _ => None,
    });
    with_teams(&mut expected.reviewers, |slug| (slug == "reviewers").then(|| vec!["grace".to_string()]));
    assert_eq!(page, expected);
    assert_eq!(page.files.len(), 4, "the second page of files is read on");
    assert_eq!(page.reviewers[2].name.as_deref(), Some("Grace Hopper"));
    assert_eq!(page.reviewers[2].team.as_deref(), Some("reviewers"));
}

#[test]
fn patches_over_the_api_are_the_patches_gh_reads() {
    let server = serve(|seen| match seen.target.as_str() {
        "/repos/mele-labs/armada/pulls/2190/files?per_page=100" => ok(PATCHES),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let patches = forge.pull_request_patches(&root, "2190").unwrap();
    assert_eq!(patches, listed(&lines(PATCHES_GH)));
    let first = patches[0].patch.clone().unwrap();
    assert_eq!(forge.pull_request_file_patch(&root, "2190", &patches[0].path), Some(FilePatch::Patch(first)));
    assert_eq!(forge.pull_request_file_patch(&root, "2190", "crates/fleet/src/review.rs"), Some(FilePatch::NoPatch));
    assert_eq!(forge.pull_request_file_patch(&root, "2190", "nowhere.rs"), Some(FilePatch::NoSuchFile));
}

/// Which way a read went.
fn asked(forge: &ApiForge, root: &str, answer: Result<Option<&'static str>, ApiFailure>) -> Option<&'static str> {
    forge.read(root, |_, _| answer, |_| Some("gh"))
}

#[test]
fn a_repository_not_on_github_goes_to_gh_whole() {
    let forge = ApiForge::through(api("http://127.0.0.1:9", &["t"]).0);
    let elsewhere = checkout("https://git.example.com/team/tool.git");
    let no_remote = TempRepo::with_a_commit();
    let github = on_github();
    assert_eq!(asked(&forge, &elsewhere.root_str(), Ok(Some("api"))), Some("gh"));
    assert_eq!(asked(&forge, &no_remote.root_str(), Ok(Some("api"))), Some("gh"));
    assert_eq!(asked(&forge, &github.root_str(), Ok(Some("api"))), Some("api"));
}

#[test]
fn no_token_goes_to_gh_and_any_other_failure_is_silence() {
    let forge = ApiForge::through(api("http://127.0.0.1:9", &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();
    assert_eq!(asked(&forge, &root, Err(ApiFailure::NoToken("not signed in".to_string()))), Some("gh"));
    assert_eq!(asked(&forge, &root, Err(ApiFailure::Unreachable("down".to_string()))), None);
}

#[test]
fn a_checkouts_repository_is_read_from_its_remote_once() {
    let forge = ApiForge::through(api("http://127.0.0.1:9", &["t"]).0);
    let repo = on_github();
    assert_eq!(asked(&forge, &repo.root_str(), Ok(Some("api"))), Some("api"));
    repo.git(&["remote", "set-url", "origin", "https://git.example.com/team/tool.git"]);
    assert_eq!(asked(&forge, &repo.root_str(), Ok(Some("api"))), Some("api"), "the remote is not read again");
}
