//! The open pull requests as a watcher reads them, in one forge call: `gh api graphql` reduced
//! by `--jq` to `@tsv`, so the vendor's words stop in [`check_state`] and [`queue_of`].
//! `docs/concepts/fleet.md`, *Telling the owner of a pull request*.

use adapter_traits::{CiState, FromOutside, PullQueue, WatchedCheck, WatchedPull, WatchedPulls};

use crate::delivery::{run_in, FORGE};
use crate::under_review::as_written;

/// The check whose result gates a pull request, where it has one.
const GATE: &str = "ci";

/// A job gets 45 minutes. One that ended cancelled after at least this long was stopped by the
/// limit, and a runner that hung is the likeliest cause.
const HUNG_AFTER_SECONDS: u64 = 40 * 60;

const QUERY: &str = "query($owner:String!,$name:String!){repository(owner:$owner,name:$name){\
    pullRequests(states:OPEN,first:100,orderBy:{field:UPDATED_AT,direction:DESC}){nodes{\
    number headRefName url headRefOid mergeable mergeQueueEntry{state} \
    commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100){nodes{__typename \
    ... on CheckRun{name status conclusion detailsUrl startedAt completedAt} \
    ... on StatusContext{context state targetUrl}}}}}}}}}}}";

/// One pull request a line: number, branch, address, head, mergeability, queue state, then its
/// checks (RS apart) of name, status, conclusion, address and seconds it ran (US apart).
const LINES: &str = "\
    .data.repository.pullRequests.nodes[]? | [(.number | tostring), (.headRefName // \"\"), \
    (.url // \"\"), (.headRefOid // \"\"), (.mergeable // \"\"), (.mergeQueueEntry.state // \"\"), \
    ([.commits.nodes[0].commit.statusCheckRollup.contexts.nodes[]? | \
    [(.name // .context // \"\"), (.status // \"\"), (.conclusion // .state // \"\"), \
    (.detailsUrl // .targetUrl // \"\"), \
    (if .startedAt and .completedAt then ((.completedAt | fromdateiso8601) - \
    (.startedAt | fromdateiso8601)) else 0 end | tostring)] | join(\"\\u001f\")] \
    | join(\"\\u001e\"))] | @tsv";

/// Every open pull request, `None` where the forge would not answer.
pub(crate) fn read(in_repo: &str) -> Option<WatchedPulls> {
    let query = format!("query={QUERY}");
    let run = run_in(
        in_repo,
        FORGE,
        &[
            "api",
            "graphql",
            "-F",
            "owner={owner}",
            "-F",
            "name={repo}",
            "-f",
            &query,
            "--jq",
            LINES,
        ],
    )
    .ok()?;
    run.status.success().then(|| {
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .filter_map(pull_of_line)
            .collect()
    })
}

fn pull_of_line(line: &str) -> Option<WatchedPull> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let branch = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let head = field.next().unwrap_or_default();
    let mergeable = field.next().unwrap_or_default();
    let queue = field.next().unwrap_or_default();
    let rollup = field.next().unwrap_or_default();
    let mut checks: Vec<WatchedCheck> = rollup
        .split('\u{1e}')
        .filter(|one| !one.is_empty())
        .map(|one| {
            let mut part = one.split('\u{1f}');
            let name = part.next().unwrap_or_default();
            let status = part.next().unwrap_or_default().to_ascii_lowercase();
            let conclusion = part.next().unwrap_or_default().to_ascii_lowercase();
            let address = part.next().unwrap_or_default();
            let seconds: u64 = part.next().unwrap_or_default().parse().unwrap_or(0);
            let state = check_state(&status, &conclusion);
            WatchedCheck {
                name: FromOutside::verbatim(as_written(name)),
                state,
                required: false,
                hung: state == CiState::Failed
                    && conclusion == "cancelled"
                    && seconds >= HUNG_AFTER_SECONDS,
                log_url: (!address.is_empty()).then(|| FromOutside::verbatim(as_written(address))),
            }
        })
        .collect();
    let gated = checks.iter().any(|check| check.name.as_written() == GATE);
    for check in &mut checks {
        check.required = !gated || check.name.as_written() == GATE;
    }
    Some(WatchedPull {
        number,
        branch: FromOutside::verbatim(as_written(branch)),
        url: FromOutside::verbatim(as_written(url)),
        head: is_a_commit(head).then(|| head.to_string()),
        conflicting: mergeable.eq_ignore_ascii_case("conflicting"),
        queue: queue_of(queue),
        checks,
    })
}

/// How one check came out, in the words the forge lists: a check run's `status` and `conclusion`,
/// or a status context's `state` alone. **A word with no name here is a failure.**
fn check_state(status: &str, conclusion: &str) -> CiState {
    match (status, conclusion) {
        (_, "success" | "neutral" | "skipped") => CiState::Passed,
        (_, "pending" | "expected") => CiState::Pending,
        ("", "") => CiState::Pending,
        (status, "") if status != "completed" => CiState::Pending,
        _ => CiState::Failed,
    }
}

/// An entry in the queue is going on unless the forge marked it unmergeable.
fn queue_of(state: &str) -> PullQueue {
    match state.to_ascii_lowercase().as_str() {
        "" => PullQueue::Outside,
        "unmergeable" => PullQueue::Unmergeable,
        _ => PullQueue::Waiting,
    }
}

fn is_a_commit(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "1111111111111111111111111111111111111111";

    #[test]
    fn a_line_is_read_into_checks_a_conflict_and_a_queue_state() {
        let line = format!(
            "7\tb\tu\t{SHA}\tCONFLICTING\tUNMERGEABLE\trust_test\u{1f}COMPLETED\u{1f}FAILURE\u{1f}http://l/1\u{1f}60\u{1e}ci\u{1f}COMPLETED\u{1f}FAILURE\u{1f}\u{1f}0"
        );
        let pull = pull_of_line(&line).unwrap();
        assert!(pull.conflicting);
        assert_eq!(pull.queue, PullQueue::Unmergeable);
        assert_eq!(pull.head.as_deref(), Some(SHA));
        assert!(pull.failed());
        let [test, gate] = &pull.checks[..] else {
            panic!("two checks")
        };
        assert!(
            !test.required && gate.required,
            "ci gates when there is one"
        );
        assert_eq!(
            test.log_url.as_ref().map(|it| it.as_written()),
            Some("http://l/1")
        );
        assert!(gate.log_url.is_none());
    }

    #[test]
    fn a_cancelled_job_that_ran_the_whole_limit_is_hung_and_a_quick_one_is_not() {
        let hung = |seconds: u64, conclusion: &str| {
            let line =
                format!("1\tb\tu\t\t\t\tci\u{1f}COMPLETED\u{1f}{conclusion}\u{1f}\u{1f}{seconds}");
            pull_of_line(&line).unwrap().checks[0].hung
        };
        assert!(hung(2700, "CANCELLED"));
        assert!(
            !hung(30, "CANCELLED"),
            "stopped by a person or a newer push"
        );
        assert!(!hung(2700, "FAILURE"));
    }

    #[test]
    fn nothing_known_is_no_conflict_no_queue_and_a_check_not_finished_is_pending() {
        let pull = pull_of_line("8\tc\tu2\tdef\tUNKNOWN\t\tci\u{1f}IN_PROGRESS\u{1f}\u{1f}\u{1f}0")
            .unwrap();
        assert!(!pull.conflicting);
        assert_eq!(pull.queue, PullQueue::Outside);
        assert_eq!(pull.head, None);
        assert_eq!(pull.checks[0].state, CiState::Pending);
        assert!(!pull.failed());
    }
}
