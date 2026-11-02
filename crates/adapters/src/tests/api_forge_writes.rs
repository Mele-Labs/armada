//! The forge's writes over GitHub's API, against a socket on this machine: each asserts the
//! requests a write sends (method, path, body) and how the answer is read. Nothing here reaches
//! GitHub, and a repository off GitHub or a call with no token is shown to reach `gh` only
//! through stand-ins, since the real one would write.

use adapter_traits::{
    Base, DiffSide, FiledIssue, Forge, NotDelivered, NotFiled, NotRerun, Opened, PullRequestWrite, Review,
    ReviewComment, ReviewSaid, Rerun, Worktree,
};

use super::github_api::{api, ok, serve, Reply, Seen, Server};
use super::repo::TempRepo;
use crate::github_api::{ApiFailure, GitHubApi};
use crate::ApiForge;

#[derive(serde::Serialize)]
struct Empty {}

fn on_github() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.git(&["remote", "add", "origin", "https://github.com/Mele-Labs/armada.git"]);
    repo
}

fn forge(server: &Server) -> ApiForge {
    ApiForge::through(api(&server.url, &["t"]).0)
}

fn refused(status: u16, body: &str) -> Reply {
    Reply { status, headers: Vec::new(), body: body.to_string() }
}

fn not_found() -> Reply {
    refused(404, r#"{"message":"Not Found"}"#)
}

/// Method and target of every request the server saw, in order.
fn asked(server: &Server) -> Vec<String> {
    server.seen().iter().map(|seen| format!("{} {}", seen.method, seen.target)).collect()
}

/// The mutations the server saw, by name, in order.
fn mutations(server: &Server) -> Vec<&'static str> {
    const NAMES: [&str; 12] = [
        "dequeuePullRequest(",
        "addPullRequestReviewThreadReply(",
        "addPullRequestReviewThread(",
        "addPullRequestReview(",
        "submitPullRequestReview(",
        "deletePullRequestReview(",
        "enablePullRequestAutoMerge(",
        "disablePullRequestAutoMerge(",
        "mergePullRequest(",
        "markPullRequestReadyForReview(",
        "unresolveReviewThread(",
        "resolveReviewThread(",
    ];
    server
        .seen()
        .iter()
        .filter_map(|seen| NAMES.into_iter().find(|name| seen.body.contains(name)))
        .collect()
}

fn node(state: &str, draft: bool, status: &str, queued: bool, queue: bool) -> Reply {
    ok(&format!(
        r#"{{"data":{{"repository":{{"pullRequest":{{"id":"PR_1","state":"{state}","isDraft":{draft},"headRefOid":"abc123","mergeStateStatus":"{status}","isInMergeQueue":{queued},"isMergeQueueEnabled":{queue}}}}}}}}}"#
    ))
}

fn is_node(seen: &Seen) -> bool {
    seen.method == "POST" && seen.target == "/graphql" && seen.body.contains("isMergeQueueEnabled")
}

const PULLS: &str = "/repos/mele-labs/armada/pulls";

#[test]
fn a_pull_request_is_looked_for_by_its_branch_and_then_opened() {
    let server = serve(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("GET", "/repos/mele-labs/armada/pulls?head=mele-labs%3Aarmada%2Fx&state=open") => ok("[]"),
        ("POST", PULLS) => ok(r#"{"html_url":"https://github.com/Mele-Labs/armada/pull/9"}"#),
        _ => not_found(),
    });
    let repo = on_github();
    let worktree = Worktree::at(repo.root_str(), "armada/x");
    let base = Base::Inferred("main".to_string());
    let review = Review::assembled("Say it", "In full").as_draft();

    let opened = forge(&server).open_for_review(&worktree, &base, &review);

    assert_eq!(opened, Ok(Opened::PullRequest { url: "https://github.com/Mele-Labs/armada/pull/9".to_string() }));
    assert_eq!(
        asked(&server),
        [
            "GET /repos/mele-labs/armada/pulls?head=mele-labs%3Aarmada%2Fx&state=open",
            "POST /repos/mele-labs/armada/pulls"
        ]
    );
    let seen = server.seen();
    assert_eq!(seen[1].body, r#"{"title":"Say it","body":"In full","head":"armada/x","base":"main","draft":true}"#);
}

#[test]
fn a_branch_with_a_pull_request_open_is_answered_with_it_and_creates_none() {
    let server = serve(|seen| match seen.method.as_str() {
        "GET" => ok(r#"[{"html_url":"https://github.com/Mele-Labs/armada/pull/4"}]"#),
        _ => not_found(),
    });
    let repo = on_github();
    let worktree = Worktree::at(repo.root_str(), "armada/x");

    let opened = forge(&server).open_for_review(&worktree, &Base::Inferred("main".to_string()), &Review::assembled("t", "b"));

    assert_eq!(opened, Ok(Opened::AlreadyOpen { url: "https://github.com/Mele-Labs/armada/pull/4".to_string() }));
    assert_eq!(asked(&server).len(), 1, "{:?}", asked(&server));
}

#[test]
fn a_look_that_fails_is_not_an_answer_and_the_pull_request_is_still_opened() {
    let server = serve(|seen| match seen.method.as_str() {
        "GET" => refused(500, r#"{"message":"Server Error"}"#),
        _ => ok(r#"{"html_url":"https://github.com/Mele-Labs/armada/pull/5"}"#),
    });
    let repo = on_github();
    let worktree = Worktree::at(repo.root_str(), "armada/x");

    let opened = forge(&server).open_for_review(&worktree, &Base::Inferred("main".to_string()), &Review::assembled("t", "b"));

    assert_eq!(opened, Ok(Opened::PullRequest { url: "https://github.com/Mele-Labs/armada/pull/5".to_string() }));
}

#[test]
fn a_branch_is_opened_without_a_look_and_a_refusal_carries_what_was_wrong() {
    let server = serve(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("POST", PULLS) if seen.body.contains("armada/empty") => refused(
            422,
            r#"{"message":"Validation Failed","errors":[{"resource":"PullRequest","code":"custom","message":"No commits between main and armada/empty"}]}"#,
        ),
        ("POST", PULLS) => ok(r#"{"html_url":"https://github.com/Mele-Labs/armada/pull/6"}"#),
        _ => not_found(),
    });
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    let url = forge.open_branch_for_review(&root, "main", "armada/manifest", "Edit", "Body");
    assert_eq!(url, Ok("https://github.com/Mele-Labs/armada/pull/6".to_string()));
    assert_eq!(server.seen()[0].body, r#"{"title":"Edit","body":"Body","head":"armada/manifest","base":"main","draft":false}"#);

    let empty = forge.open_branch_for_review(&root, "main", "armada/empty", "Edit", "Body");
    assert_eq!(
        empty,
        Err(NotDelivered::of(
            "the pull request",
            "GitHub answered 422: Validation Failed: No commits between main and armada/empty"
        ))
    );
    assert_eq!(asked(&server), vec!["POST /repos/mele-labs/armada/pulls"; 2]);
}

#[test]
fn a_comment_and_a_verdict_are_each_one_request() {
    let server = serve(|_| ok("{}"));
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    forge.write_pull_request(&root, "12", &PullRequestWrite::Comment { body: "Looks right".into() }).unwrap();
    forge.write_pull_request(&root, "12", &PullRequestWrite::Approve { body: None }).unwrap();
    forge.write_pull_request(&root, "12", &PullRequestWrite::Approve { body: Some("  ".into()) }).unwrap();
    forge.write_pull_request(&root, "12", &PullRequestWrite::Approve { body: Some("Good".into()) }).unwrap();
    forge.write_pull_request(&root, "12", &PullRequestWrite::RequestChanges { body: "Not yet".into() }).unwrap();

    let seen = server.seen();
    let sent: Vec<(String, String)> = seen.iter().map(|it| (format!("{} {}", it.method, it.target), it.body.clone())).collect();
    let review = "POST /repos/mele-labs/armada/pulls/12/reviews".to_string();
    assert_eq!(
        sent,
        vec![
            ("POST /repos/mele-labs/armada/issues/12/comments".to_string(), r#"{"body":"Looks right"}"#.to_string()),
            (review.clone(), r#"{"event":"APPROVE"}"#.to_string()),
            (review.clone(), r#"{"event":"APPROVE"}"#.to_string()),
            (review.clone(), r#"{"event":"APPROVE","body":"Good"}"#.to_string()),
            (review, r#"{"event":"REQUEST_CHANGES","body":"Not yet"}"#.to_string()),
        ]
    );
}

#[test]
fn a_blank_comment_or_reason_is_refused_in_gh_s_words_and_sends_nothing() {
    let server = serve(|_| ok("{}"));
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    let comment = forge.write_pull_request(&root, "12", &PullRequestWrite::Comment { body: " ".into() });
    let changes = forge.write_pull_request(&root, "12", &PullRequestWrite::RequestChanges { body: "".into() });

    assert_eq!(comment, Err("a comment needs something to say".to_string()));
    assert_eq!(changes, Err("a request for changes needs a reason".to_string()));
    assert!(server.seen().is_empty());
}

#[test]
fn a_refused_comment_is_the_forge_s_sentence() {
    let server = serve(|_| refused(403, r#"{"message":"Resource not accessible by personal access token"}"#));
    let repo = on_github();

    let refused = forge(&server).write_pull_request(&repo.root_str(), "12", &PullRequestWrite::Comment { body: "x".into() });

    assert_eq!(refused, Err("GitHub answered 403: Resource not accessible by personal access token".to_string()));
}

fn review_server(refuse_comment: bool) -> Server {
    serve(move |seen| {
        if is_node(seen) {
            return node("OPEN", false, "CLEAN", false, false);
        }
        let body = &seen.body;
        if body.contains("addPullRequestReviewThreadReply(") {
            ok(r#"{"data":{"addPullRequestReviewThreadReply":{"comment":{"id":"C_1"}}}}"#)
        } else if body.contains("addPullRequestReviewThread(") && refuse_comment {
            ok(r#"{"errors":[{"message":"Pull request review thread line must be part of the diff"}],"data":null}"#)
        } else if body.contains("addPullRequestReviewThread(") {
            ok(r#"{"data":{"addPullRequestReviewThread":{"thread":{"id":"T_1"}}}}"#)
        } else if body.contains("addPullRequestReview(") {
            ok(r#"{"data":{"addPullRequestReview":{"pullRequestReview":{"id":"R_1"}}}}"#)
        } else if body.contains("submitPullRequestReview(") {
            ok(r#"{"data":{"submitPullRequestReview":{"pullRequestReview":{"id":"R_1"}}}}"#)
        } else if body.contains("deletePullRequestReview(") {
            ok(r#"{"data":{"deletePullRequestReview":{"pullRequestReview":{"id":"R_1"}}}}"#)
        } else {
            not_found()
        }
    })
}

fn with_comments() -> PullRequestWrite {
    PullRequestWrite::Review {
        verdict: ReviewSaid::ChangesRequested,
        body: Some("Two things".into()),
        comments: vec![
            ReviewComment::Line { path: "src/a.rs".into(), line: 12, side: DiffSide::Left, body: "Why?".into() },
            ReviewComment::File { path: "src/b.rs".into(), body: "Whole file".into() },
            ReviewComment::Reply { thread: "T_9".into(), body: "Agreed".into() },
        ],
        commit: None,
    }
}

#[test]
fn a_review_with_comments_is_opened_filled_and_submitted_at_the_branch_head() {
    let server = review_server(false);
    let repo = on_github();

    forge(&server).write_pull_request(&repo.root_str(), "12", &with_comments()).unwrap();

    assert_eq!(
        mutations(&server),
        vec![
            "addPullRequestReview(",
            "addPullRequestReviewThread(",
            "addPullRequestReviewThread(",
            "addPullRequestReviewThreadReply(",
            "submitPullRequestReview(",
        ]
    );
    let seen = server.seen();
    // The node read, then the review opened at the head the read named.
    assert!(seen[1].body.contains(r#""pr":"PR_1""#) && seen[1].body.contains(r#""commit":"abc123""#), "{}", seen[1].body);
    assert!(seen[2].body.contains(r#""review":"R_1""#) && seen[2].body.contains(r#""line":12"#), "{}", seen[2].body);
    assert!(seen[2].body.contains(r#""side":"LEFT""#) && seen[2].body.contains(r#""body":"Why?""#), "{}", seen[2].body);
    assert!(seen[5].body.contains(r#""event":"REQUEST_CHANGES""#) && seen[5].body.contains(r#""body":"Two things""#));
}

#[test]
fn a_review_the_forge_refuses_part_way_is_deleted_and_the_sentence_is_the_forge_s() {
    let server = review_server(true);
    let repo = on_github();

    let refused = forge(&server).write_pull_request(&repo.root_str(), "12", &with_comments());

    assert_eq!(refused, Err("Pull request review thread line must be part of the diff".to_string()));
    assert_eq!(
        mutations(&server),
        vec!["addPullRequestReview(", "addPullRequestReviewThread(", "deletePullRequestReview("]
    );
}

#[test]
fn a_thread_is_resolved_or_reopened_as_the_forge_says() {
    let server = serve(|seen| {
        if seen.body.contains("unresolveReviewThread(") {
            ok(r#"{"data":{"unresolveReviewThread":{"thread":{"isResolved":false}}}}"#)
        } else if seen.body.contains("resolveReviewThread(") {
            ok(r#"{"data":{"resolveReviewThread":{"thread":{"isResolved":true}}}}"#)
        } else {
            not_found()
        }
    });
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    assert_eq!(forge.resolve_review_thread(&root, "T_1", true), Ok(true));
    assert_eq!(forge.resolve_review_thread(&root, "T_1", false), Ok(false));
    assert!(server.seen()[0].body.contains(r#""thread":"T_1""#));
}

/// Auto-merge asked of a pull request standing as `node` says; the mutations that followed.
fn auto_merge_after(reply: fn() -> Reply) -> Vec<&'static str> {
    let server = serve(move |seen| {
        if is_node(seen) {
            reply()
        } else {
            ok(r#"{"data":{"clientMutationId":null}}"#)
        }
    });
    let repo = on_github();
    forge(&server).enable_auto_merge(&repo.root_str(), "12").unwrap();
    mutations(&server)
}

#[test]
fn auto_merge_does_what_gh_pr_merge_auto_does_on_each_kind_of_base() {
    // A merge queue: one that is ready is queued by the same mutation that waits for one that is not.
    assert_eq!(auto_merge_after(|| node("OPEN", false, "CLEAN", false, true)), vec!["enablePullRequestAutoMerge("]);
    assert_eq!(auto_merge_after(|| node("OPEN", false, "BLOCKED", false, true)), vec!["enablePullRequestAutoMerge("]);
    // No queue: one that cannot merge yet waits; one that can is merged now.
    assert_eq!(auto_merge_after(|| node("OPEN", false, "BLOCKED", false, false)), vec!["enablePullRequestAutoMerge("]);
    assert_eq!(auto_merge_after(|| node("OPEN", false, "CLEAN", false, false)), vec!["mergePullRequest("]);
    assert_eq!(auto_merge_after(|| node("OPEN", false, "UNSTABLE", false, false)), vec!["mergePullRequest("]);
    // Already queued or merged: nothing to do, and not a refusal.
    assert!(auto_merge_after(|| node("OPEN", false, "CLEAN", true, true)).is_empty());
    assert!(auto_merge_after(|| node("MERGED", false, "CLEAN", false, true)).is_empty());
}

#[test]
fn auto_merge_asks_for_a_merge_commit_and_a_refusal_is_the_forge_s_sentence() {
    let server = serve(|seen| {
        if is_node(seen) {
            node("OPEN", false, "BLOCKED", false, false)
        } else {
            ok(r#"{"errors":[{"message":"Auto merge is not allowed for this repository"}],"data":null}"#)
        }
    });
    let repo = on_github();

    let refused = forge(&server).enable_auto_merge(&repo.root_str(), "12");

    assert_eq!(refused, Err("Auto merge is not allowed for this repository".to_string()));
    let sent = &server.seen()[1].body;
    assert!(sent.contains("mergeMethod:MERGE") && sent.contains(r#""id":"PR_1""#), "{sent}");
}

#[test]
fn auto_merge_is_taken_back_unless_the_pull_request_is_queued() {
    let queued = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = queued.clone();
    let server = serve(move |seen| {
        if is_node(seen) {
            node("OPEN", false, "BLOCKED", flag.load(std::sync::atomic::Ordering::SeqCst), true)
        } else {
            ok(r#"{"data":{"clientMutationId":null}}"#)
        }
    });
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    forge.disable_auto_merge(&root, "12").unwrap();
    assert_eq!(mutations(&server), vec!["disablePullRequestAutoMerge("]);
    queued.store(true, std::sync::atomic::Ordering::SeqCst);
    forge.disable_auto_merge(&root, "12").unwrap();
    assert_eq!(mutations(&server), vec!["disablePullRequestAutoMerge("], "a queued one is left alone");
}

#[test]
fn a_queued_pull_request_is_taken_out_of_the_queue_and_one_not_queued_is_done() {
    let queued = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = queued.clone();
    let server = serve(move |seen| {
        if is_node(seen) {
            node("OPEN", false, "BLOCKED", flag.load(std::sync::atomic::Ordering::SeqCst), true)
        } else {
            ok(r#"{"data":{"clientMutationId":null}}"#)
        }
    });
    let repo = on_github();
    let root = repo.root_str();
    let forge = forge(&server);

    forge.dequeue(&root, "12").unwrap();
    assert_eq!(mutations(&server), Vec::<&str>::new(), "nothing queued, nothing to take out");
    queued.store(true, std::sync::atomic::Ordering::SeqCst);
    forge.dequeue(&root, "12").unwrap();
    assert_eq!(mutations(&server), vec!["dequeuePullRequest("]);
    let sent = server.seen().iter().map(|seen| seen.body.clone()).find(|body| body.contains("dequeuePullRequest(")).unwrap();
    assert!(sent.contains(r#""id":"PR_1""#), "{sent}");
}

#[test]
fn a_draft_is_marked_ready_and_a_pull_request_out_of_draft_is_left_alone() {
    let draft = |draft: bool, state: &'static str| {
        let server = serve(move |seen| {
            if is_node(seen) {
                node(state, draft, "CLEAN", false, true)
            } else {
                ok(r#"{"data":{"clientMutationId":null}}"#)
            }
        });
        let repo = on_github();
        let outcome = forge(&server).mark_ready(&repo.root_str(), "12");
        (outcome, mutations(&server))
    };
    assert_eq!(draft(true, "OPEN"), (Ok(()), vec!["markPullRequestReadyForReview("]));
    assert_eq!(draft(false, "OPEN"), (Ok(()), vec![]));
    assert_eq!(
        draft(true, "CLOSED"),
        (
            Err("Pull request mele-labs/armada#12 is closed. Only draft pull requests can be marked as \"ready for review\""
                .to_string()),
            vec![]
        )
    );
}

fn checks(contexts: &str) -> Reply {
    ok(&format!(
        r#"{{"data":{{"repository":{{"pullRequest":{{"commits":{{"nodes":[{{"commit":{{"statusCheckRollup":{{"contexts":{{"nodes":[{contexts}]}}}}}}}}]}}}}}}}}}}"#
    ))
}

fn checks_paged(contexts: &str, page_info: &str) -> Reply {
    ok(&format!(
        r#"{{"data":{{"repository":{{"pullRequest":{{"commits":{{"nodes":[{{"commit":{{"statusCheckRollup":{{"contexts":{{"pageInfo":{page_info},"nodes":[{contexts}]}}}}}}}}]}}}}}}}}}}"#
    ))
}

fn run(name: &str, started: &str, conclusion: &str, id: u32) -> String {
    format!(
        r#"{{"__typename":"CheckRun","name":"{name}","status":"COMPLETED","conclusion":"{conclusion}","startedAt":"{started}","detailsUrl":"https://github.com/Mele-Labs/armada/actions/runs/{id}/job/1","checkSuite":{{"workflowRun":{{"event":"pull_request","workflow":{{"name":"ci"}}}}}}}}"#
    )
}

#[test]
fn failed_checks_are_found_as_gh_pr_checks_buckets_them_and_each_run_is_started_again() {
    let contexts = [
        // Failed, then run again and passed: the latest of the check is the one that counts.
        run("lint", "2026-10-22T10:00:00Z", "FAILURE", 111),
        run("lint", "2026-10-22T11:00:00Z", "SUCCESS", 112),
        run("test", "2026-10-22T10:30:00Z", "FAILURE", 222),
        run("build", "2026-10-22T10:30:00Z", "TIMED_OUT", 333),
        // Two jobs of one run name one run once; cancelled and skipped are not failed.
        run("test (b)", "2026-10-22T10:31:00Z", "ACTION_REQUIRED", 222),
        run("cancelled", "2026-10-22T10:30:00Z", "CANCELLED", 444),
        run("skipped", "2026-10-22T10:30:00Z", "SKIPPED", 555),
        // A status that failed names no run.
        r#"{"__typename":"StatusContext","context":"deploy","state":"FAILURE","targetUrl":"https://deploy.example/1"}"#.to_string(),
    ]
    .join(",");
    let server = serve(move |seen| {
        if seen.method == "POST" && seen.target == "/graphql" {
            checks(&contexts)
        } else {
            ok("{}")
        }
    });
    let repo = on_github();

    let rerun = forge(&server).rerun_failed(&repo.root_str(), "12");

    assert_eq!(rerun, Ok(Rerun { runs: 2 }));
    assert_eq!(
        asked(&server)[1..],
        [
            "POST /repos/mele-labs/armada/actions/runs/222/rerun-failed-jobs",
            "POST /repos/mele-labs/armada/actions/runs/333/rerun-failed-jobs"
        ]
    );
}

#[test]
fn a_failed_check_past_the_first_hundred_is_started_again_too() {
    let first = run("lint", "2026-10-22T10:00:00Z", "SUCCESS", 1);
    let second = run("deploy", "2026-10-22T10:00:00Z", "FAILURE", 666);
    let server = serve(move |seen| {
        if seen.method == "POST" && seen.target == "/graphql" {
            let (nodes, page) = match seen.body.contains(r#""after":"C1""#) {
                false => (&first, r#"{"hasNextPage":true,"endCursor":"C1"}"#),
                true => (&second, r#"{"hasNextPage":false,"endCursor":"C2"}"#),
            };
            checks_paged(nodes, page)
        } else {
            ok("{}")
        }
    });
    let repo = on_github();

    let rerun = forge(&server).rerun_failed(&repo.root_str(), "12");

    assert_eq!(rerun, Ok(Rerun { runs: 1 }));
    assert_eq!(
        asked(&server),
        ["POST /graphql", "POST /graphql", "POST /repos/mele-labs/armada/actions/runs/666/rerun-failed-jobs"]
    );
}

#[test]
fn nothing_is_started_again_when_nothing_failed_and_the_sentence_says_so() {
    let passing = run("test", "2026-10-22T10:30:00Z", "SUCCESS", 1);
    let server = serve(move |_| checks(&passing));
    let repo = on_github();

    let rerun = forge(&server).rerun_failed(&repo.root_str(), "12");

    assert_eq!(rerun, Err(NotRerun { said: "no check has failed on pull request 12".to_string() }));
    assert_eq!(asked(&server).len(), 1);
}

#[test]
fn an_issue_is_filed_and_named_by_its_address() {
    let server = serve(|seen| match (seen.method.as_str(), seen.target.as_str()) {
        ("POST", "/repos/mele-labs/armada/issues") => ok(r#"{"html_url":"https://github.com/Mele-Labs/armada/issues/77"}"#),
        _ => not_found(),
    });
    let repo = on_github();

    let filed = forge(&server).file_issue(&repo.root_str(), "A finding", "Where it was");

    assert_eq!(filed, Ok(FiledIssue { url: "https://github.com/Mele-Labs/armada/issues/77".to_string() }));
    assert_eq!(server.seen()[0].body, r#"{"title":"A finding","body":"Where it was"}"#);
}

#[test]
fn a_write_the_forge_refuses_is_its_sentence_and_is_not_tried_through_gh() {
    let server = serve(|_| refused(410, r#"{"message":"Issues are disabled for this repo"}"#));
    let repo = on_github();

    let filed = forge(&server).file_issue(&repo.root_str(), "A finding", "Where");

    assert_eq!(filed, Err(NotFiled { said: "GitHub answered 410: Issues are disabled for this repo".to_string() }));
    assert_eq!(asked(&server).len(), 1);
}

#[test]
fn a_repository_off_github_or_a_call_with_no_token_is_gh_s_and_asks_nothing() {
    let server = serve(|_| ok("{}"));
    let elsewhere = TempRepo::with_a_commit();
    elsewhere.git(&["remote", "add", "origin", "https://git.example.com/team/tool.git"]);
    let github = on_github();
    let no_token = ApiForge::through(GitHubApi::against(&server.url, || Err(ApiFailure::NoToken("not signed in".to_string()))));
    let through = |forge: &ApiForge, root: &str| {
        forge.written(root, |api, _| api.post("/x", &Empty {}).map(|_| Ok("api")), |_| Ok::<_, String>("gh"), |said| said)
    };

    assert_eq!(through(&forge(&server), &elsewhere.root_str()), Ok("gh"));
    assert_eq!(through(&no_token, &github.root_str()), Ok("gh"));
    assert!(server.seen().is_empty());
}
