//! The merge queue's unfinished runs, the reason a job of one failed, and stopping a run. **The
//! vendor's words stop here**: a runner that never came and a job that ran out of time are said
//! in annotations as sentences, and [`cause_of`] turns them into [`InfraCause`].

use adapter_traits::{FromOutside, InfraCause, InfraFailure, Oid, QueueRun, QueueRuns};

use crate::delivery::{run_in, said, FORGE};
use crate::main_ci::{asked, is_a_commit};

/// The most failed jobs whose annotations are read for one commit.
pub(crate) const MOST_READ: usize = 20;

const RUNS: &str = "\
    .workflow_runs[]? | [(.id | tostring), (.head_branch // \"\"), (.head_sha // \"\"), \
    (.status // \"\")] | @tsv";

const FAILED: &str = "\
    .check_runs[]? | select(.conclusion == \"failure\" or .conclusion == \"timed_out\" \
    or .conclusion == \"startup_failure\") | \
    [(.id | tostring), (.name // \"\"), (.html_url // .details_url // \"\")] | @tsv";

const MESSAGES: &str = "[.[]? | (.message // \"\"), (.title // \"\")] | join(\" \")";

/// The merge queue's runs not yet finished, from the forge's newest hundred.
pub(crate) fn read(in_repo: &str, base: &str) -> Option<QueueRuns> {
    let path = "repos/{owner}/{repo}/actions/runs?event=merge_group&per_page=100";
    let said = asked(in_repo, &["api", path, "--jq", RUNS])?;
    Some(said.lines().filter_map(|line| run_of_line(base, line)).collect())
}

fn run_of_line(base: &str, line: &str) -> Option<QueueRun> {
    let mut field = line.split('\t');
    let (id, branch, head, status) = (field.next()?, field.next()?, field.next()?, field.next()?);
    run_of_row(base, id, branch, head, status)
}

/// One run as the forge lists it. `None` for a run that is over, or that has no id.
pub(crate) fn run_of_row(base: &str, id: &str, branch: &str, head: &str, status: &str) -> Option<QueueRun> {
    if status == "completed" {
        return None;
    }
    Some(QueueRun {
        id: id.parse().ok()?,
        number: queue_number_of(base, branch),
        branch: FromOutside::verbatim(branch),
        head: Oid::parse(head),
    })
}

/// The pull request a queue branch is named for: `gh-readonly-queue/main/pr-12-<sha>` is 12.
pub(crate) fn queue_number_of(base: &str, branch: &str) -> Option<u64> {
    let named = branch.strip_prefix(&format!("gh-readonly-queue/{base}/pr-"))?;
    named.split_once('-')?.0.parse().ok()
}

/// The failed jobs of `commit`, each with why. `None` where the forge would not say, **and also
/// where one failed job's annotations would not be read**, so a failure whose reason is unknown is
/// never taken for one that is the code's.
pub(crate) fn failures(in_repo: &str, commit: &str) -> Option<Vec<InfraFailure>> {
    if !is_a_commit(commit) {
        return None;
    }
    let path = format!("repos/{{owner}}/{{repo}}/commits/{commit}/check-runs?per_page=100");
    let listed = asked(in_repo, &["api", "--paginate", &path, "--jq", FAILED])?;
    let mut out = Vec::new();
    for line in listed.lines().take(MOST_READ) {
        let mut field = line.split('\t');
        let (id, name, url) = (field.next()?, field.next()?, field.next().unwrap_or_default());
        let notes = format!("repos/{{owner}}/{{repo}}/check-runs/{id}/annotations?per_page=100");
        let text = asked(in_repo, &["api", &notes, "--jq", MESSAGES])?;
        out.push(failure_of(name, url, &text));
    }
    Some(out)
}

/// One failed job with the text of its annotations.
pub(crate) fn failure_of(name: &str, url: &str, annotations: &str) -> InfraFailure {
    InfraFailure {
        job: FromOutside::verbatim(name),
        log_url: Some(url).filter(|it| !it.is_empty()).map(FromOutside::verbatim),
        cause: cause_of(annotations),
    }
}

/// What a job's annotations say went wrong. **Both sentences are the forge's own**; anything else,
/// or nothing, is [`InfraCause::Other`].
pub(crate) fn cause_of(annotations: &str) -> InfraCause {
    if annotations.contains("was not acquired by Runner") {
        InfraCause::NoRunner
    } else if annotations.contains("exceeded the maximum execution time") {
        InfraCause::TimedOut
    } else {
        InfraCause::Other
    }
}

/// Stop run `id`, as `gh` writes it.
pub(crate) fn cancel(in_repo: &str, id: u64) -> Result<(), String> {
    let path = format!("repos/{{owner}}/{{repo}}/actions/runs/{id}/cancel");
    let run = run_in(in_repo, FORGE, &["api", "--method", "POST", &path])
        .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(said(&run)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_RUNNER: &str = "The job was not acquired by Runner of type hosted even after multiple attempts";
    const TIME: &str = "The job running on runner GitHub Actions 41 has exceeded the maximum execution time of 1h0m0s";

    #[test]
    fn an_annotation_that_says_no_runner_came_is_that_cause() {
        assert_eq!(cause_of(NO_RUNNER), InfraCause::NoRunner);
        assert_eq!(cause_of(&format!("Internal server error. {NO_RUNNER}")), InfraCause::NoRunner);
    }

    #[test]
    fn an_annotation_that_says_the_time_ran_out_is_that_cause() {
        assert_eq!(cause_of(TIME), InfraCause::TimedOut);
        assert_eq!(cause_of("The operation was canceled. exceeded the maximum execution time of 6h"), InfraCause::TimedOut);
    }

    #[test]
    fn a_failure_the_forge_gave_no_such_reason_for_is_not_infrastructure() {
        assert_eq!(cause_of("Process completed with exit code 101."), InfraCause::Other);
        assert_eq!(cause_of(""), InfraCause::Other);
    }

    #[test]
    fn a_queue_branch_names_its_pull_request_up_to_the_dash() {
        assert_eq!(queue_number_of("main", "gh-readonly-queue/main/pr-4039-b04cc295a8"), Some(4039));
        assert_eq!(queue_number_of("main", "gh-readonly-queue/main/pr-4039"), None);
        assert_eq!(queue_number_of("main", "gh-readonly-queue/other/pr-4039-b04cc2"), None);
        assert_eq!(queue_number_of("main", "armada/session-1"), None);
    }

    #[test]
    fn only_a_run_not_yet_over_is_a_queue_run() {
        let sha = "a".repeat(40);
        let live = run_of_row("main", "77", "gh-readonly-queue/main/pr-9-bb", &sha, "in_progress").expect("live");
        assert_eq!((live.id, live.number, live.head.map(|it| it.as_str().to_string())), (77, Some(9), Some(sha.clone())));
        assert!(run_of_row("main", "78", "gh-readonly-queue/main/pr-9-bb", &sha, "completed").is_none());
        assert_eq!(run_of_line("main", &format!("79\tgh-readonly-queue/main/pr-3-cc\t{sha}\tqueued")).map(|it| it.id), Some(79));
    }

    #[test]
    fn a_failed_job_carries_its_name_its_page_and_why() {
        let one = failure_of("native", "https://example.test/job/1", NO_RUNNER);
        assert_eq!(one.job.as_written(), "native");
        assert_eq!(one.log_url.as_ref().map(|it| it.as_written()), Some("https://example.test/job/1"));
        assert_eq!(one.cause, InfraCause::NoRunner);
        assert_eq!(failure_of("lint", "", "").log_url, None);
    }
}
