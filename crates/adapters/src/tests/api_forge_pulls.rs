//! What the forge says of one pull request, over GitHub's API against `gh`: its facts, what became
//! of it, who has looked at it and what anybody wrote, its line comments and its diff. A `.json`
//! in `github_answers/` is what GitHub answered; the `.tsv` beside it is what `gh`'s own `--jq`
//! printed for the same pull request, run here with `jq`. All of them are made up, as
//! `page_reviewed` is, because nothing here has the pull request they describe.

use adapter_traits::{Forge, PullRequestDiff, PullRequestStanding, WhatPeopleSaid, WhatTheForgeRan};

use super::api_forge_reads::{lines, not_found, on_github};
use super::github_api::{api, ok, serve, Reply};
use crate::inline_comments::folded as inline_folded;
use crate::landing::{fields, landing_of, LandingRead};
use crate::pull_request_facts::parsed;
use crate::under_review::folded;
use crate::ApiForge;

const FACTS: &str = include_str!("github_answers/pr_facts.json");
const FACTS_GH: &str = include_str!("github_answers/pr_facts.tsv");
const LANDING_MERGED: &str = include_str!("github_answers/landing_merged.json");
const LANDING_MERGED_GH: &str = include_str!("github_answers/landing_merged.tsv");
const LANDING_OPEN: &str = include_str!("github_answers/landing_open.json");
const LANDING_OPEN_GH: &str = include_str!("github_answers/landing_open.tsv");
const REVIEW: &str = include_str!("github_answers/under_review.json");
const REVIEW_MORE_COMMENTS: &str = include_str!("github_answers/under_review_more_comments.json");
const REVIEW_MORE_CONTEXTS: &str = include_str!("github_answers/under_review_more_contexts.json");
const REVIEW_GH: &str = include_str!("github_answers/under_review.tsv");
const INLINE: &str = include_str!("github_answers/inline_comments.json");
const INLINE_GH: &str = include_str!("github_answers/inline_comments.tsv");
const DIFF: &str = include_str!("github_answers/pull_diff.patch");
const DIFF_FILES: &str = include_str!("github_answers/pull_diff.files");

const ADDRESS: &str = "https://github.com/Mele-Labs/armada/pull/2190";

#[test]
fn a_pull_requests_facts_over_the_api_are_the_facts_gh_reads() {
    let server = serve(|seen| match (seen.method.as_str(), seen.body.as_str()) {
        ("POST", body) if body.contains("closingIssuesReferences") => ok(FACTS),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let facts = forge.pull_request_facts(&root, "2190").unwrap();
    assert_eq!(facts, parsed(FACTS_GH.trim_end()).unwrap());
    assert_eq!(facts.standing, PullRequestStanding::Open);
    assert_eq!(facts.closes, [2099, 2101]);
    assert_eq!(facts.base, "main");
    assert!(facts.auto_merge);
    assert_eq!(facts.title, "Read the forge over the API", "a tab in a title is a space");
    // The address of the pull request is the number it ends in, within this repository only.
    assert_eq!(forge.pull_request_facts(&root, ADDRESS), Some(facts));
}

#[test]
fn a_pull_request_is_named_by_its_number_or_its_address_in_this_repository_only() {
    let forge = ApiForge::through(api("http://127.0.0.1:9", &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();
    assert_eq!(forge.pull_number(&root, "2190"), Some(2190));
    assert_eq!(forge.pull_number(&root, ADDRESS), Some(2190));
    assert_eq!(forge.pull_number(&root, "https://GitHub.com/mele-labs/Armada/pull/7/"), Some(7));
    for gh_reads in [
        "main",
        "https://github.com/someone/else/pull/2190",
        "https://github.com/Mele-Labs/armada/issues/2190",
        "see https://github.com/Mele-Labs/armada/pull/2190",
        "",
    ] {
        assert_eq!(forge.pull_number(&root, gh_reads), None, "{gh_reads}");
    }
}

#[test]
fn what_became_of_a_pull_request_over_the_api_is_what_gh_reads() {
    let server = serve(|seen| match seen.body.as_str() {
        body if body.contains("\"number\":2190") => ok(LANDING_MERGED),
        body if body.contains("\"number\":2191") => ok(LANDING_OPEN),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    for (named, answered) in [(ADDRESS, LANDING_MERGED_GH), ("https://github.com/Mele-Labs/armada/pull/2191", LANDING_OPEN_GH)] {
        let [number, title, state, mergeable, base, pinned, head, merged] = fields::<8>(answered.trim_end()).unwrap();
        let expected = landing_of(named, &LandingRead { number, title, state, mergeable, base, pinned, head, merged });
        assert_eq!(forge.landing(&root, named), expected);
    }
    let merged = forge.landing(&root, ADDRESS);
    assert_eq!(merged.became.merged_at.as_deref(), Some("2026-10-21T12:00:00Z"));
    assert!(merged.compared.is_none(), "a merged pull request is compared with nothing");
    let open = forge.landing(&root, "https://github.com/Mele-Labs/armada/pull/2191");
    assert_eq!(open.became.merged_at, None);
    assert!(open.compared.is_some());
    // No pull request by that number is the forge's silence.
    assert_eq!(forge.landing(&root, "7").became, adapter_traits::WhatBecameOfIt::unknown());
}

fn review_server() -> super::github_api::Server {
    serve(|seen| match seen.body.as_str() {
        body if body.contains("\"after\":\"Y1\"") => ok(REVIEW_MORE_COMMENTS),
        body if body.contains("\"after\":\"X1\"") => ok(REVIEW_MORE_CONTEXTS),
        body if body.contains("reviewDecision") => ok(REVIEW),
        _ => not_found(),
    })
}

#[test]
fn who_has_looked_at_a_pull_request_over_the_api_is_what_gh_reads_every_page_of_it() {
    let server = review_server();
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let read = forge.under_review(&repo.root_str(), "2190", "ci");
    assert_eq!(read, folded(&lines(REVIEW_GH), "ci"));
    assert_eq!(read.people, WhatPeopleSaid::ChangesRequested);
    assert_eq!(read.remarks.len(), 4, "a comment with no body is not one");
    assert_eq!(read.verdicts.len(), 2);
    assert!(matches!(read.checks, WhatTheForgeRan::SomeFailed { checks: 4, .. }), "{:?}", read.checks);
    assert_eq!(server.seen().len(), 3, "the pull request, then the comments and the checks after the first page");
    // A pull request by its address is the same pull request.
    assert_eq!(forge.under_review(&repo.root_str(), ADDRESS, "ci"), read);
}

#[test]
fn a_rollup_the_cap_cut_is_not_a_pass_and_a_pull_request_the_forge_will_not_show_is_unreadable() {
    let endless = r#"{"data":{"repository":{"pullRequest":{"commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"pageInfo":{"hasNextPage":true,"endCursor":"X1"},"nodes":[{"__typename":"CheckRun","name":"ci","status":"COMPLETED","conclusion":"SUCCESS"}]}}}}]}}}}}"#;
    let server = serve(move |seen| match seen.body.as_str() {
        body if body.contains("\"after\":\"X1\"") => ok(endless),
        body if body.contains("\"after\":\"Y1\"") => ok(REVIEW_MORE_COMMENTS),
        body if body.contains("reviewDecision") && body.contains("\"number\":1") => ok(REVIEW),
        body if body.contains("reviewDecision") => Reply {
            status: 200,
            headers: Vec::new(),
            body: r#"{"data":null,"errors":[{"message":"Could not resolve to a PullRequest"}]}"#.to_string(),
        },
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let cut = forge.under_review(&root, "1", "ci");
    assert_eq!(cut.people, WhatPeopleSaid::ChangesRequested, "what people said is still read");
    assert_eq!(cut.checks, WhatTheForgeRan::Unreadable, "a pass over part of the checks is not given");
    let gone = forge.under_review(&root, "2", "ci");
    assert_eq!(gone.people, WhatPeopleSaid::Unreadable);
    assert_eq!(gone.checks, WhatTheForgeRan::Unreadable);
}

#[test]
fn line_comments_over_the_api_are_the_comments_gh_reads() {
    let server = serve(|seen| match seen.target.as_str() {
        "/repos/mele-labs/armada/pulls/2190/comments?per_page=100" => ok(INLINE),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let remarks = forge.inline_remarks(&root, ADDRESS).unwrap();
    assert_eq!(remarks, inline_folded(INLINE_GH));
    assert_eq!(remarks.len(), 3, "a comment with no body is not one");
    assert_eq!(remarks[1].inline.as_ref().map(|it| it.line), Some(17), "where it was, once the diff moved");
    assert!(remarks[2].inline.is_none());
    assert_eq!(forge.inline_remarks(&root, "2191"), None, "a pull request the forge does not have");
}

#[test]
fn a_diff_over_the_api_is_the_diff_gh_reads_asked_for_as_one() {
    let server = serve(|seen| match (seen.target.as_str(), seen.accept.as_deref()) {
        ("/repos/mele-labs/armada/pulls/2190", Some("application/vnd.github.v3.diff")) => ok(DIFF),
        ("/repos/mele-labs/armada/pulls/2191", _) => Reply {
            status: 406,
            headers: Vec::new(),
            body: r#"{"message":"Sorry, the diff exceeded the maximum number of files (300)."}"#.to_string(),
        },
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let diff = forge.pull_request_diff(&root, "2190").unwrap();
    assert_eq!(diff, PullRequestDiff { files: lines(DIFF_FILES), patch: DIFF.to_string() });
    assert_eq!(diff.files.len(), 3);
    assert_eq!(forge.pull_request_diff(&root, "2191"), None, "a diff too large to show is the forge's silence");
}
