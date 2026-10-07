//! What the forge says about a repository's base branch: where it stands, what
//! CI ran on that commit, a failed job's log, and the pull request that merged
//! it. `docs/concepts/fleet.md`, *What Fleet knows about main's CI*.
//!
//! **Five small asks, never one big one**, so Fleet pays for each only when it
//! needs it: the head is a ref lookup, the runs are asked once the head moves,
//! a log is asked for a failed job only, and the pull request only for a red.
//! The open pull requests are one listing, whatever their number.
//!
//! **No parse**, `crate::under_review`'s way: `--jq` reduces to `@tsv` and the
//! vendor's words (`success`, `timed_out`, ...) stop in [`state_of`].

use std::io::Read;
use std::process::{Command, Stdio};

use adapter_traits::{
    CiRun, CiRuns, CiState, CommitStatus, FromOutside, MergedPull, OpenPull, OpenPulls,
    RecentlyMerged, RecentlyMergedPulls, StatusState,
};

use crate::delivery::{run_in, FORGE};
use crate::under_review::as_written;

/// How much of a log's end is kept. A failure is printed last and a log runs to
/// megabytes; 256 KiB holds a runner's whole summary with room to spare.
pub(crate) const LOG_TAIL: usize = 256 * 1024;

const RUNS: &str = "\
    .check_runs[] | [(.id | tostring), (.name // \"\"), (.status // \"\"), \
    (.conclusion // \"\"), (.html_url // .details_url // \"\")] | @tsv";

/// The commit `base` is at on the forge, read as a ref and nothing dearer.
pub(crate) fn head(in_repo: &str, base: &str) -> Option<String> {
    let path = format!("repos/{{owner}}/{{repo}}/git/ref/heads/{base}");
    let said = asked(in_repo, &["api", &path, "--jq", ".object.sha"])?;
    let sha = said.trim();
    is_a_commit(sha).then(|| sha.to_string())
}

/// Every CI job on `commit`. `None` where the forge would not answer.
pub(crate) fn runs(in_repo: &str, commit: &str) -> Option<CiRuns> {
    if !is_a_commit(commit) {
        return None;
    }
    let path = format!("repos/{{owner}}/{{repo}}/commits/{commit}/check-runs?per_page=100");
    let said = asked(in_repo, &["api", "--paginate", &path, "--jq", RUNS])?;
    Some(folded_runs(&said))
}

pub(crate) fn folded_runs(lines: &str) -> CiRuns {
    lines
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut field = line.split('\t');
            let handle = field.next()?;
            let name = field.next()?;
            let status = field.next().unwrap_or_default();
            let conclusion = field.next().unwrap_or_default();
            let url = field.next().unwrap_or_default();
            (!handle.is_empty()).then(|| CiRun {
                name: FromOutside::verbatim(as_written(name)),
                state: state_of(status, conclusion),
                handle: FromOutside::verbatim(handle),
                log_url: (!url.is_empty()).then(|| FromOutside::verbatim(as_written(url))),
            })
        })
        .collect()
}

/// This forge's words for how a job came out. **A conclusion with no name here
/// is a failure**, never a pass: see [`CiState`].
pub(crate) fn state_of(status: &str, conclusion: &str) -> CiState {
    match conclusion {
        "success" | "neutral" | "skipped" => CiState::Passed,
        "" if status != "completed" => CiState::Pending,
        _ => CiState::Failed,
    }
}

/// The end of one job's log, with the forge's per-line timestamps taken off so
/// a runner's own summary lines read as the runner printed them.
pub(crate) fn log(in_repo: &str, run: &CiRun) -> Option<FromOutside> {
    let id = run.handle.as_written();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let path = format!("repos/{{owner}}/{{repo}}/actions/jobs/{id}/logs");
    let mut child = Command::new(FORGE)
        .args(["api", &path])
        .current_dir(in_repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let kept = tail(child.stdout.take()?, LOG_TAIL);
    let done = child.wait().ok()?;
    done.success()
        .then(|| FromOutside::verbatim(without_timestamps(&String::from_utf8_lossy(&kept))))
}

/// The last `keep` bytes of a stream, holding no more than twice that at once.
pub(crate) fn tail(mut from: impl Read, keep: usize) -> Vec<u8> {
    let (mut held, mut chunk) = (Vec::new(), vec![0u8; 64 * 1024]);
    while let Ok(n) = from.read(&mut chunk) {
        if n == 0 {
            break;
        }
        held.extend_from_slice(&chunk[..n]);
        if held.len() > keep * 2 {
            held.drain(..held.len() - keep);
        }
    }
    let at = held.len().saturating_sub(keep);
    held.split_off(at)
}

/// `2026-10-06T10:00:00.1234567Z text` is `text`. A line that does not start
/// that way is left alone.
pub(crate) fn without_timestamps(log: &str) -> String {
    let mut out = String::with_capacity(log.len());
    for line in log.lines() {
        let line = line.trim_start_matches('\u{feff}');
        out.push_str(stamped(line).unwrap_or(line));
        out.push('\n');
    }
    out
}

fn stamped(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let shaped = bytes.len() > 21
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[10] == b'T';
    let end = line.find("Z ").filter(|at| *at <= 32)?;
    shaped.then(|| &line[end + 2..])
}

/// The pull request that put `commit` on the base: the forge's own mapping
/// where it names one whose merge commit this is, otherwise what a merge
/// commit's subject says.
pub(crate) fn merged_by(in_repo: &str, commit: &str) -> Option<MergedPull> {
    if !is_a_commit(commit) {
        return None;
    }
    let path = format!("repos/{{owner}}/{{repo}}/commits/{commit}/pulls");
    let jq = format!(
        ".[] | select(.merge_commit_sha == \"{commit}\") | \
         [(.number | tostring), (.html_url // \"\"), (.head.ref // \"\")] | @tsv"
    );
    if let Some(said) = asked(in_repo, &["api", &path, "--jq", &jq]) {
        if let Some(pull) = said.lines().find_map(pull_of_line) {
            return Some(pull);
        }
    }
    let path = format!("repos/{{owner}}/{{repo}}/commits/{commit}");
    let subject = asked(
        in_repo,
        &[
            "api",
            &path,
            "--jq",
            ".commit.message | split(\"\\n\") | .[0]",
        ],
    )?;
    named_in(subject.trim())
}

fn pull_of_line(line: &str) -> Option<MergedPull> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let url = field.next().unwrap_or_default();
    let branch = field.next().unwrap_or_default();
    Some(MergedPull {
        number,
        url: (!url.is_empty()).then(|| FromOutside::verbatim(as_written(url))),
        branch: (!branch.is_empty()).then(|| FromOutside::verbatim(as_written(branch))),
    })
}

/// A merge commit's subject: `Merge pull request #12 from owner/branch`, or a
/// squash's `title (#12)`. Anything else names no pull request.
pub(crate) fn named_in(subject: &str) -> Option<MergedPull> {
    let (number, branch) = match subject.strip_prefix("Merge pull request #") {
        Some(rest) => {
            let (number, from) = rest.split_once(' ')?;
            let branch = from.strip_prefix("from ")?.split_once('/')?.1;
            (number, Some(branch))
        }
        None => {
            let inside = subject.strip_suffix(')')?.rsplit_once("(#")?.1;
            (inside, None)
        }
    };
    Some(MergedPull {
        number: number.parse().ok()?,
        url: None,
        branch: branch.map(FromOutside::verbatim),
    })
}

/// `gh`'s listing: each pull request's checks ride on it as `name US status US
/// conclusion`, joined by `RS`. A status context has no `name`, `status` or
/// `conclusion`, so its `context` and `state` stand in.
const OPEN_PULLS: &str = "\
    .[] | [(.number | tostring), (.title // \"\"), (.headRefName // \"\"), (.url // \"\"), \
    (.author.login // \"\"), \
    ([.statusCheckRollup[]? | [(.name // .context // \"\"), (.status // \"\"), \
    (.conclusion // .state // \"\")] | join(\"\\u001f\")] | join(\"\\u001e\")), \
    (.headRefOid // \"\")] | @tsv";

/// The check whose result is a pull request's `ci`, where it has one.
const GATE: &str = "ci";

/// Every open pull request, `None` where the forge would not answer.
pub(crate) fn open_pulls(in_repo: &str) -> Option<OpenPulls> {
    let said = asked(
        in_repo,
        &[
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,headRefName,url,author,statusCheckRollup,headRefOid",
            "--jq",
            OPEN_PULLS,
        ],
    )?;
    Some(said.lines().filter_map(open_pull_of_line).collect())
}

/// Put one commit status on `commit`: `gh api` fills `{owner}/{repo}` from the
/// repository it runs in. **Only a commit and a context that cannot be read as an
/// option**, so a word from a branch name never becomes a flag.
pub(crate) fn publish_status(in_repo: &str, status: &CommitStatus) -> Result<(), String> {
    let CommitStatus {
        commit,
        context,
        state,
        description,
    } = status;
    if !is_a_commit(commit.as_str()) || context.is_empty() || context.starts_with('-') {
        return Err(format!("{commit} / {context} is not a status to publish"));
    }
    let state = match *state {
        StatusState::Pending => "pending",
        StatusState::Success => "success",
    };
    let route = format!("repos/{{owner}}/{{repo}}/statuses/{commit}");
    let run = run_in(
        in_repo,
        FORGE,
        &[
            "api",
            &route,
            "-f",
            &format!("state={state}"),
            "-f",
            &format!("context={context}"),
            "-f",
            &format!("description={description}"),
        ],
    )
    .map_err(|why| why.to_string())?;
    match run.status.success() {
        true => Ok(()),
        false => Err(crate::delivery::said(&run)),
    }
}

const MERGED_PULLS: &str = "\
    .[] | [(.number | tostring), (.title // \"\"), (.headRefName // \"\"), (.url // \"\"), \
    (.author.login // \"\"), (.mergedAt // \"\"), (.mergeCommit.oid // \"\")] | @tsv";

/// The newest `limit` pull requests merged into `base`, newest first. `None`
/// where the forge would not answer.
pub(crate) fn recently_merged(
    in_repo: &str,
    base: &str,
    limit: usize,
) -> Option<RecentlyMergedPulls> {
    if base.is_empty() || base.starts_with('-') {
        return None;
    }
    let limit = limit.to_string();
    let said = asked(
        in_repo,
        &[
            "pr",
            "list",
            "--state",
            "merged",
            "--base",
            base,
            "--limit",
            &limit,
            "--json",
            "number,title,headRefName,url,author,mergedAt,mergeCommit",
            "--jq",
            MERGED_PULLS,
        ],
    )?;
    let mut merged: Vec<RecentlyMerged> = said.lines().filter_map(merged_of_line).collect();
    merged.sort_by(|a, b| b.merged_at.as_written().cmp(a.merged_at.as_written()));
    Some(merged)
}

fn merged_of_line(line: &str) -> Option<RecentlyMerged> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let title = field.next().unwrap_or_default();
    let branch = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let author = field.next().unwrap_or_default();
    let merged_at = field.next().unwrap_or_default();
    let commit = field.next().unwrap_or_default();
    (!merged_at.is_empty()).then(|| RecentlyMerged {
        number,
        title: FromOutside::verbatim(as_written(title)),
        branch: FromOutside::verbatim(as_written(branch)),
        url: FromOutside::verbatim(as_written(url)),
        author: (!author.is_empty()).then(|| FromOutside::verbatim(as_written(author))),
        merged_at: FromOutside::verbatim(as_written(merged_at)),
        commit: is_a_commit(commit).then(|| FromOutside::verbatim(commit)),
    })
}

fn open_pull_of_line(line: &str) -> Option<OpenPull> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let title = field.next().unwrap_or_default();
    let branch = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let author = field.next().unwrap_or_default();
    let rollup = field.next().unwrap_or_default();
    let head = field.next().unwrap_or_default();
    let checks: Vec<(String, CiState)> = rollup
        .split('\u{1e}')
        .filter(|one| !one.is_empty())
        .map(|one| {
            let mut part = one.split('\u{1f}');
            let name = part.next().unwrap_or_default();
            let status = part.next().unwrap_or_default().to_ascii_lowercase();
            let conclusion = part.next().unwrap_or_default().to_ascii_lowercase();
            (as_written(name), check_state_of(&status, &conclusion))
        })
        .collect();
    let deciding: Vec<&(String, CiState)> = match checks.iter().any(|(name, _)| name == GATE) {
        true => checks.iter().filter(|(name, _)| name == GATE).collect(),
        false => checks.iter().collect(),
    };
    let ci = [CiState::Failed, CiState::Pending, CiState::Passed]
        .into_iter()
        .find(|state| deciding.iter().any(|(_, one)| one == state));
    Some(OpenPull {
        number,
        title: FromOutside::verbatim(as_written(title)),
        branch: FromOutside::verbatim(as_written(branch)),
        url: FromOutside::verbatim(as_written(url)),
        author: (!author.is_empty()).then(|| FromOutside::verbatim(as_written(author))),
        ci,
        head: is_a_commit(head).then(|| head.to_string()),
        failing: checks
            .iter()
            .filter(|(_, state)| *state == CiState::Failed)
            .map(|(name, _)| FromOutside::verbatim(name.as_str()))
            .collect(),
    })
}

/// How one check on a pull request came out, in the words `gh` lists: a
/// check run's `status` and `conclusion`, or a status context's `state` alone.
/// **A word with no name here is a failure**, [`state_of`]'s direction.
fn check_state_of(status: &str, conclusion: &str) -> CiState {
    match (status, conclusion) {
        (_, "success" | "neutral" | "skipped") => CiState::Passed,
        (_, "pending" | "expected") => CiState::Pending,
        ("", "") => CiState::Pending,
        (status, "") if status != "completed" => CiState::Pending,
        _ => CiState::Failed,
    }
}

fn is_a_commit(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.chars().all(|c| c.is_ascii_hexdigit())
}

fn asked(in_repo: &str, args: &[&str]) -> Option<String> {
    let run = run_in(in_repo, FORGE, args).ok()?;
    run.status
        .success()
        .then(|| String::from_utf8_lossy(&run.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_come_back_named_as_the_forge_names_them() {
        let lines = "11\tci\tcompleted\tfailure\thttps://forge.invalid/r/actions/runs/1/job/11\n\
                     12\tlint\tcompleted\tsuccess\t\n\
                     13\tdeploy\tin_progress\t\t\n";
        let runs = folded_runs(lines);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].name.as_written(), "ci");
        assert_eq!(runs[0].state, CiState::Failed);
        assert_eq!(
            runs[0].log_url.as_ref().map(|url| url.as_written()),
            Some("https://forge.invalid/r/actions/runs/1/job/11")
        );
        assert_eq!(runs[1].state, CiState::Passed);
        assert!(runs[1].log_url.is_none());
        assert_eq!(runs[2].state, CiState::Pending);
    }

    #[test]
    fn a_conclusion_with_no_name_here_is_never_a_pass() {
        assert_eq!(state_of("completed", "something_new"), CiState::Failed);
        assert_eq!(state_of("completed", "cancelled"), CiState::Failed);
        assert_eq!(state_of("completed", ""), CiState::Failed);
        assert_eq!(state_of("queued", ""), CiState::Pending);
    }

    #[test]
    fn the_forges_timestamps_come_off_every_line_and_nothing_else_does() {
        let log = "2026-10-06T10:00:00.1234567Z        FAIL [   0.016s] (1/2) nt tests::a\n\
                   no stamp on this one\n";
        assert_eq!(
            without_timestamps(log),
            "       FAIL [   0.016s] (1/2) nt tests::a\nno stamp on this one\n"
        );
    }

    #[test]
    fn only_the_end_of_a_long_log_is_kept() {
        let long: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
        let kept = tail(&long[..], 1000);
        assert_eq!(kept.len(), 1000);
        assert_eq!(kept, long[long.len() - 1000..]);
        assert_eq!(tail(&b"short"[..], 1000), b"short");
    }

    #[test]
    fn a_merge_commits_subject_names_its_pull_request() {
        let merge = named_in("Merge pull request #1812 from Mele-Labs/armada/cache").unwrap();
        assert_eq!(merge.number, 1812);
        assert_eq!(merge.branch.unwrap().as_written(), "armada/cache");
        assert_eq!(named_in("Fix the thing (#77)").unwrap().number, 77);
        assert!(named_in("Fix the thing").is_none());
        assert!(named_in("Merge branch 'x' into main").is_none());
    }

    #[test]
    fn a_merged_pull_request_is_read_with_its_time_and_merge_commit() {
        let commit = "c".repeat(40);
        let line = format!("1839\tAdd it\tarmada/it\thttps://forge.invalid/pull/1839\tnick\t2026-10-06T21:00:00Z\t{commit}");
        let pull = merged_of_line(&line).unwrap();
        assert_eq!(pull.number, 1839);
        assert_eq!(pull.merged_at.as_written(), "2026-10-06T21:00:00Z");
        assert_eq!(pull.commit.unwrap().as_written(), commit);
        assert_eq!(pull.author.unwrap().as_written(), "nick");
        let bare = merged_of_line("7\tt\tb\tu\t\t2026-10-06T21:00:00Z\t").unwrap();
        assert!(bare.author.is_none() && bare.commit.is_none());
        assert!(merged_of_line("8\tt\tb\tu\t\t\t").is_none(), "not merged");
    }

    #[test]
    fn only_a_commit_shaped_word_is_ever_put_in_a_path() {
        assert!(is_a_commit(&"a".repeat(40)));
        assert!(!is_a_commit("main; rm -rf"));
        assert!(!is_a_commit(""));
    }

    #[test]
    fn a_pull_requests_head_is_kept_only_where_it_is_a_commit() {
        let sha = "b".repeat(40);
        let line = format!("12\tt\tb\tu\tnick\t\t{sha}");
        assert_eq!(open_pull_of_line(&line).unwrap().head, Some(sha));
        assert_eq!(
            open_pull_of_line("12\tt\tb\tu\tnick\t\tnot").unwrap().head,
            None
        );
        assert_eq!(open_pull_of_line("12\tt\tb\tu\tnick\t").unwrap().head, None);
    }

    #[test]
    fn a_status_is_refused_for_a_commit_or_context_that_could_be_an_option() {
        for (commit, context) in [
            ("main", "needs"),
            (&"a".repeat(40), "-f"),
            (&"a".repeat(40), ""),
        ] {
            let status = CommitStatus {
                commit: commit.to_string(),
                context: context.to_string(),
                state: StatusState::Success,
                description: "x".to_string(),
            };
            assert!(publish_status(".", &status).is_err());
        }
    }

    #[test]
    fn a_pull_requests_ci_is_the_check_named_ci_where_there_is_one() {
        let line = "12\tAdd a cache\tarmada/cache\thttps://forge.invalid/pull/12\tnick\t\
                    ci\u{1f}COMPLETED\u{1f}SUCCESS\u{1e}desktop_test\u{1f}COMPLETED\u{1f}FAILURE";
        let pull = open_pull_of_line(line).unwrap();
        assert_eq!(pull.number, 12);
        assert_eq!(pull.title.as_written(), "Add a cache");
        assert_eq!(pull.branch.as_written(), "armada/cache");
        assert_eq!(pull.author.unwrap().as_written(), "nick");
        assert_eq!(pull.ci, Some(CiState::Passed));
        let failing: Vec<&str> = pull.failing.iter().map(|one| one.as_written()).collect();
        assert_eq!(failing, ["desktop_test"]);
    }

    #[test]
    fn with_no_check_named_ci_every_check_decides() {
        let running =
            "13\tt\tb\tu\t\tlint\u{1f}COMPLETED\u{1f}SUCCESS\u{1e}build\u{1f}IN_PROGRESS\u{1f}";
        assert_eq!(
            open_pull_of_line(running).unwrap().ci,
            Some(CiState::Pending)
        );
        let failed = "14\tt\tb\tu\t\tlint\u{1f}COMPLETED\u{1f}SUCCESS\u{1e}build\u{1f}COMPLETED\u{1f}TIMED_OUT";
        assert_eq!(open_pull_of_line(failed).unwrap().ci, Some(CiState::Failed));
    }

    #[test]
    fn a_pull_request_nothing_ran_on_has_no_ci_and_a_status_context_is_read_by_its_state() {
        let none = open_pull_of_line("15\tt\tb\tu\t\t").unwrap();
        assert_eq!(none.ci, None);
        assert!(none.author.is_none());
        let context = "16\tt\tb\tu\ta\tbuild\u{1f}\u{1f}PENDING\u{1e}ci\u{1f}\u{1f}FAILURE";
        let pull = open_pull_of_line(context).unwrap();
        assert_eq!(pull.ci, Some(CiState::Failed));
        assert_eq!(pull.failing.len(), 1);
    }
}
