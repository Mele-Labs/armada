//! The merge queue's runs over GitHub's API: which are unfinished, why a commit's failed jobs
//! failed, and stopping one. Each answer here is made up, and the server records what was sent.

use adapter_traits::{Forge, InfraCause};

use super::api_forge_reads::on_github;
use super::github_api::{api, ok, serve, Reply};
use crate::ApiForge;

const RUNS: &str = r#"{"workflow_runs":[
{"id":11,"head_branch":"gh-readonly-queue/main/pr-4039-aaaa","head_sha":"1111111111111111111111111111111111111111","status":"in_progress"},
{"id":12,"head_branch":"gh-readonly-queue/main/pr-4041-bbbb","head_sha":"2222222222222222222222222222222222222222","status":"completed"},
{"id":13,"head_branch":"gh-readonly-queue/main/pr-4044-cccc","head_sha":"3333333333333333333333333333333333333333","status":"queued"},
{"id":14,"head_branch":"somewhere-else","head_sha":"4444444444444444444444444444444444444444","status":"queued"}
]}"#;

const CHECKS: &str = r#"{"check_runs":[
{"id":501,"name":"native","conclusion":"failure","html_url":"https://example.test/job/501"},
{"id":502,"name":"lint","conclusion":"success","html_url":"https://example.test/job/502"},
{"id":503,"name":"unit","conclusion":"timed_out","html_url":"https://example.test/job/503"}
]}"#;

fn not_found() -> Reply {
    Reply { status: 404, headers: Vec::new(), body: r#"{"message":"Not Found"}"#.to_string() }
}

#[test]
fn the_queues_unfinished_runs_are_read_with_the_pull_request_each_branch_names() {
    let server = serve(|seen| match seen.target.as_str() {
        target if target.contains("/actions/runs?event=merge_group") => ok(RUNS),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let runs = forge.queue_runs(&on_github().root_str(), "main").expect("answered");
    let seen: Vec<(u64, Option<u64>)> = runs.iter().map(|it| (it.id, it.number)).collect();
    assert_eq!(seen, [(11, Some(4039)), (13, Some(4044)), (14, None)], "a finished run is not one");
    assert_eq!(runs[0].head.as_ref().map(|it| it.as_str()), Some("1".repeat(40).as_str()));
}

#[test]
fn a_failed_job_is_infrastructure_where_its_annotations_say_so() {
    let sha = "5".repeat(40);
    let server = serve(|seen| match seen.target.as_str() {
        target if target.contains("/check-runs/501/annotations") => ok(
            r#"[{"message":"The job was not acquired by Runner of type hosted even after multiple attempts","title":"native"}]"#,
        ),
        target if target.contains("/check-runs/503/annotations") => ok(r#"[{"message":"error[E0308]: mismatched types"}]"#),
        target if target.contains("/check-runs?") => ok(CHECKS),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    let failures = forge.infra_failures(&on_github().root_str(), &sha).expect("answered");
    let seen: Vec<(&str, InfraCause)> = failures.iter().map(|it| (it.job.as_written(), it.cause)).collect();
    assert_eq!(seen, [("native", InfraCause::NoRunner), ("unit", InfraCause::Other)], "a passing job is not listed");
    assert_eq!(failures[0].log_url.as_ref().map(|it| it.as_written()), Some("https://example.test/job/501"));
}

#[test]
fn annotations_that_will_not_read_make_the_whole_answer_silence() {
    let sha = "5".repeat(40);
    let server = serve(|seen| match seen.target.as_str() {
        target if target.contains("/check-runs?") => ok(CHECKS),
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    assert_eq!(forge.infra_failures(&on_github().root_str(), &sha), None, "a reason unread is not a code failure");
}

#[test]
fn a_run_is_stopped_with_one_post_and_nothing_else() {
    let server = serve(|seen| match seen.target.as_str() {
        target if target.ends_with("/actions/runs/11/cancel") => Reply { status: 202, headers: Vec::new(), body: "{}".to_string() },
        _ => not_found(),
    });
    let forge = ApiForge::through(api(&server.url, &["t"]).0);
    assert_eq!(forge.cancel_run(&on_github().root_str(), 11), Ok(()));
    let sent: Vec<String> = server.seen().iter().map(|it| format!("{} {}", it.method, it.target)).collect();
    assert_eq!(sent, ["POST /repos/mele-labs/armada/actions/runs/11/cancel"]);
    assert!(forge.cancel_run(&on_github().root_str(), 99).is_err(), "the forge's refusal is the answer");
}
