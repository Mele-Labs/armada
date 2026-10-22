//! The forge's reads of CI, over GitHub's API against `gh`: each answer is a `.json` GitHub
//! sent, and the `.tsv` beside it what `gh`'s own `--jq` printed for it, so a read gives what
//! `gh` read. `check_runs`, `workflow_runs` and `commit_pulls` are trimmed to the fields read.

use std::sync::{Arc, OnceLock};

use adapter_traits::{CiRun, CiState, Forge, FromOutside};

use super::github_api::{api, ok, serve, Reply};
use super::repo::TempRepo;
use crate::main_ci::{folded_runs, named_in, tail, without_timestamps, LOG_TAIL};
use crate::ApiForge;

const CHECK_RUNS: &str = include_str!("github_answers/check_runs.json");
const CHECK_RUNS_MORE: &str = include_str!("github_answers/check_runs_more.json");
const CHECK_RUNS_GH: &str = include_str!("github_answers/check_runs_both.tsv");
const WORKFLOW_RUNS: &str = include_str!("github_answers/workflow_runs.json");
const COMMIT_PULLS: &str = include_str!("github_answers/commit_pulls.json");
const COMMIT_PULLS_GH: &str = include_str!("github_answers/commit_pulls.tsv");

pub(super) const COMMIT: &str = "dcbd37f60b0000000000000000000000000000aa";

pub(super) fn not_found() -> Reply {
    Reply { status: 404, headers: Vec::new(), body: r#"{"message":"Not Found"}"#.to_string() }
}

pub(super) fn on_github() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.git(&["remote", "add", "origin", "https://github.com/Mele-Labs/armada.git"]);
    repo
}

pub(super) fn lines(tsv: &str) -> Vec<String> {
    tsv.lines().filter(|line| !line.is_empty()).map(str::to_string).collect()
}

#[test]
fn the_base_head_over_the_api_is_the_commit_gh_reads() {
    let server = serve(|seen| match seen.target.as_str() {
        "/repos/mele-labs/armada/git/ref/heads/main" => {
            ok(&format!(r#"{{"ref":"refs/heads/main","object":{{"sha":"{COMMIT}","type":"commit"}}}}"#))
        }
        "/repos/mele-labs/armada/git/ref/heads/odd" => ok(r#"{"object":{"sha":"not a commit"}}"#),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();
    assert_eq!(forge.base_head_on_the_forge(&root, "main").as_deref(), Some(COMMIT));
    assert_eq!(forge.base_head_on_the_forge(&root, "odd"), None);
    assert_eq!(forge.base_head_on_the_forge(&root, "gone"), None);
}

#[test]
fn the_jobs_on_a_commit_over_the_api_are_the_jobs_gh_reads_every_page() {
    let url = Arc::new(OnceLock::<String>::new());
    let next = url.clone();
    let server = serve(move |seen| {
        let first = format!("/repos/mele-labs/armada/commits/{COMMIT}/check-runs?per_page=100");
        if seen.target == first {
            let link = format!("<{}{first}&page=2>; rel=\"next\"", next.get().unwrap());
            return Reply { status: 200, headers: vec![("Link", link)], body: CHECK_RUNS.to_string() };
        }
        if seen.target == format!("{first}&page=2") {
            return ok(CHECK_RUNS_MORE);
        }
        not_found()
    });
    url.set(server.url.clone()).unwrap();
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();

    let runs = forge.ci_runs_on(&repo.root_str(), COMMIT).unwrap();
    assert_eq!(runs, folded_runs(CHECK_RUNS_GH));
    let seen: Vec<(&str, CiState)> = runs.iter().map(|run| (run.name.as_written(), run.state)).collect();
    assert_eq!(
        seen,
        [
            ("ci", CiState::Passed),
            ("lint", CiState::Passed),
            ("deploy", CiState::Pending),
            ("build\tshard 1", CiState::Failed),
            ("acceptance", CiState::Passed),
        ],
        "the newest of a name stands, and a skipped one never replaces one that ran"
    );
    assert_eq!(forge.ci_runs_on(&repo.root_str(), "not-a-commit"), None);
}

#[test]
fn the_merge_queues_commit_over_the_api_is_the_one_gh_reads() {
    let server = serve(|seen| match seen.target.as_str() {
        "/repos/mele-labs/armada/actions/runs?event=merge_group&per_page=100" => ok(WORKFLOW_RUNS),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();
    // #26 is not #2649, and a run with no branch is nobody's.
    assert_eq!(forge.queue_commit_of(&root, "main", 26).as_deref(), Some("1111111111111111111111111111111111111111"));
    assert_eq!(forge.queue_commit_of(&root, "main", 2649).as_deref(), Some("2222222222222222222222222222222222222222"));
    assert_eq!(forge.queue_commit_of(&root, "main", 7), None);
}

#[test]
fn the_pull_request_that_merged_a_commit_over_the_api_is_the_one_gh_reads() {
    let (b, c) = ("b".repeat(40), "c".repeat(40));
    let server = serve(move |seen| {
        let target = seen.target.as_str();
        if target == format!("/repos/mele-labs/armada/commits/{COMMIT}/pulls") {
            ok(COMMIT_PULLS)
        } else if target == format!("/repos/mele-labs/armada/commits/{b}") {
            ok(r#"{"commit":{"message":"Merge pull request #12 from owner/some/branch\n\nbody"}}"#)
        } else if target == format!("/repos/mele-labs/armada/commits/{c}") {
            ok(r#"{"commit":{"message":"A change nobody numbered"}}"#)
        } else {
            not_found()
        }
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let mapped = forge.merged_by(&root, COMMIT).unwrap();
    let mut field = COMMIT_PULLS_GH.trim_end().split('\t');
    assert_eq!(mapped.number.to_string(), field.next().unwrap());
    assert_eq!(mapped.url.as_ref().map(FromOutside::as_written), field.next());
    assert_eq!(mapped.branch.as_ref().map(FromOutside::as_written), field.next());

    // No mapping: what the merge commit's subject says, and only that.
    let named = forge.merged_by(&root, &"b".repeat(40)).unwrap();
    assert_eq!(named, named_in("Merge pull request #12 from owner/some/branch").unwrap());
    assert_eq!(forge.merged_by(&root, &"c".repeat(40)), None);
}

fn a_failed_job(handle: &str) -> CiRun {
    let mut jobs = folded_runs(&format!("{handle}\tci\tcompleted\tfailure\t"));
    if jobs.is_empty() {
        // A handle with no digits is still a handle; build the run by hand.
        return CiRun {
            name: FromOutside::verbatim("ci"),
            state: CiState::Failed,
            handle: FromOutside::verbatim(handle),
            log_url: None,
        };
    }
    jobs.remove(0)
}

fn stamped_log(lines: usize) -> String {
    (0..lines).map(|n| format!("2026-10-06T10:00:00.1234567Z line {n}\n")).collect()
}

#[test]
fn a_log_is_read_from_the_storage_the_api_redirects_to_without_the_token() {
    let storage = serve(|seen| match seen.target.as_str() {
        "/logs/job-11" => ok(&stamped_log(10_000)),
        "/logs/expired" => Reply { status: 403, headers: Vec::new(), body: "expired".to_string() },
        _ => not_found(),
    });
    let storage_url = storage.url.clone();
    let server = serve(move |seen| {
        let to = |file: &str| Reply {
            status: 302,
            headers: vec![("Location", format!("{storage_url}/logs/{file}"))],
            body: String::new(),
        };
        match seen.target.as_str() {
            "/repos/mele-labs/armada/actions/jobs/11/logs" => to("job-11"),
            "/repos/mele-labs/armada/actions/jobs/12/logs" => to("expired"),
            _ => not_found(),
        }
    });
    let forge = ApiForge::through(api(&server.url, &["secret-token"]).0);
    let repo = on_github();
    let root = repo.root_str();

    let log = forge.ci_log(&root, &a_failed_job("11")).unwrap();
    let whole = stamped_log(10_000);
    let kept = tail(whole.as_bytes(), LOG_TAIL);
    assert_eq!(log, FromOutside::verbatim(without_timestamps(&String::from_utf8_lossy(&kept))));
    assert!(log.as_written().ends_with("line 9999\n"), "the end of the log is kept");
    assert!(!log.as_written().contains("line 0\n"), "and only the end");
    assert!(log.as_written().len() <= LOG_TAIL);

    let asked = server.seen();
    assert_eq!(asked[0].authorization.as_deref(), Some("Bearer secret-token"));
    let fetched = storage.seen();
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].authorization, None, "the storage is not given the token");

    assert_eq!(forge.ci_log(&root, &a_failed_job("12")), None, "a log whose address has expired");
    assert_eq!(forge.ci_log(&root, &a_failed_job("99")), None, "a job the forge does not have");
    let before = server.seen().len();
    assert_eq!(forge.ci_log(&root, &a_failed_job("../1")), None);
    assert_eq!(server.seen().len(), before, "a handle that is no job id asks nothing");
}

#[test]
fn a_redirect_within_the_apis_own_origin_is_followed_and_one_off_it_is_not() {
    let elsewhere = serve(|_| ok("{}"));
    let off = elsewhere.url.clone();
    let server = serve(move |seen| match seen.target.as_str() {
        "/old" => Reply { status: 301, headers: vec![("Location", "/repos/moved".to_string())], body: String::new() },
        "/repos/moved" => ok(r#"{"moved":true}"#),
        "/away" => Reply { status: 302, headers: vec![("Location", format!("{off}/there"))], body: String::new() },
        _ => not_found(),
    });
    let (github, _) = api(&server.url, &["secret-token"]);
    assert_eq!(github.get("/old").unwrap().body, br#"{"moved":true}"#);
    github.get("/away").unwrap();
    assert!(elsewhere.seen().is_empty(), "the token was not carried off the API's origin");
}
