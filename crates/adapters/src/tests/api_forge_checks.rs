//! A page's checks past the first hundred, over GitHub's API: read on to a cap, and where the cap
//! cuts, said in `omitted`. The `gh` reader keeps to the same cap.

use adapter_traits::{CheckStanding, Forge};

use super::api_forge_reads::{lines, not_found, on_github};
use super::github_api::{api, ok, serve};
use crate::pull_request_detail::{folded, MOST_CHECKS};
use crate::ApiForge;

const PAGE_GH: &str = include_str!("github_answers/page_2190.tsv");
const FIRST: &str = include_str!("github_answers/page_checks_first.json");
const MORE: &str = include_str!("github_answers/page_checks_more.json");
const ENDLESS: &str = include_str!("github_answers/page_checks_endless.json");
const NO_THREADS: &str = r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[]}}}}}"#;

/// A hundred check runs, as the forge lists a page of them.
fn a_hundred(from: usize) -> String {
    let check = |n: usize| {
        format!(
            r#"{{"__typename":"CheckRun","name":"job {n}","status":"COMPLETED","conclusion":"SUCCESS","startedAt":"2026-10-12T19:34:57Z","completedAt":"2026-10-12T19:35:57Z","detailsUrl":"https://example.test/{n}"}}"#
        )
    };
    (from..from + 100).map(check).collect::<Vec<_>>().join(",")
}

#[test]
fn checks_past_the_first_hundred_are_read_on_and_the_page_is_the_one_gh_reads() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("reviewThreads") => ok(NO_THREADS),
        body if body.contains("\"after\":\"C1\"") => ok(MORE),
        body if body.contains("baseRefName") => ok(FIRST),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let page = forge.pull_request_page(&repo.root_str(), "2190").unwrap();
    assert_eq!(page, folded(&lines(PAGE_GH)).unwrap());
    let names: Vec<&str> = page.checks.iter().map(|it| it.name.as_str()).collect();
    assert_eq!(names, ["plan", "foundations", "hooks_test", "needs"], "the second page of checks is read on");
    assert_eq!(page.omitted.checks, 0);
    assert!(page.checks.iter().all(|it| it.standing != CheckStanding::Failed));
}

#[test]
fn a_page_with_more_checks_than_the_cap_says_how_many_it_left_out() {
    let (first, more) = (ENDLESS.replace("\"@@CHECKS@@\"", &a_hundred(0)), a_hundred(100));
    let later = [
        r#"{"data":{"repository":{"pullRequest":{"rollup":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":"#,
        r#"{"totalCount":730,"pageInfo":{"hasNextPage":true,"endCursor":"C2"},"nodes":["#,
        &more,
        "]}}}}]}}}}}",
    ]
    .concat();
    let server = serve(move |seen| match seen.body.as_str() {
        body if body.contains("reviewThreads") => ok(NO_THREADS),
        body if body.contains("\"after\":") => ok(&later),
        body if body.contains("baseRefName") => ok(&first),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let page = forge.pull_request_page(&repo.root_str(), "2190").unwrap();
    assert_eq!(page.checks.len(), MOST_CHECKS);
    assert_eq!(page.omitted.checks, 730 - MOST_CHECKS, "the cap cut, and the page says by how many");
    assert_eq!(page.checks[0].name, "job 0");
    let reads = server.seen().iter().filter(|it| it.body.contains("\"after\":")).count();
    assert_eq!(reads, MOST_CHECKS / 100 - 1, "it stopped at the cap and did not read the list to its end");
}

#[test]
fn the_gh_reader_keeps_to_the_same_cap_and_says_the_same() {
    let mut records = lines(PAGE_GH);
    records.extend((0..600).map(|n| format!("check\tjob {n}\tCOMPLETED\tSUCCESS\t1\thttps://example.test/{n}")));
    let page = folded(&records).unwrap();
    assert_eq!(page.checks.len(), MOST_CHECKS);
    assert_eq!(page.omitted.checks, 604 - MOST_CHECKS);
}
