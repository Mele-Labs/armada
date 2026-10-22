//! The conversations on a pull request's diff, and resolving one.
//!
//! One `api graphql` read, reduced by `--jq` to tagged `@tsv` records the way
//! `crate::pull_request_detail` does: a `thread` record, then a `tc` record for each thing said on
//! it. The forge's words (`LEFT`, `isOutdated`) stop in [`threads_folded`].

use adapter_traits::{DiffSide, ReviewThreadFacts, ThreadSaid};

use crate::delivery::{read_in, run_in, said, FORGE};
use crate::under_review::as_written;

pub(crate) const QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    reviewThreads(first:100){nodes{id isResolved isOutdated path line diffSide \
    comments(first:50){nodes{id author{login} createdAt body}}}}}}}";

/// `// ""` on every field so a `null` cannot stop jq mid-stream. A thread whose line is `null`
/// prints no line, and so does an outdated one: the line it names has changed under it.
const REDUCTION: &str = "\
    .data.repository.pullRequest.reviewThreads.nodes[]? | \
    ([\"thread\", (.id // \"\"), ((.isResolved // false) | tostring), \
      ((.isOutdated // false) | tostring), (.path // \"\"), ((.line // \"\") | tostring), \
      (.diffSide // \"\")], \
     (.comments.nodes[]? | \
      [\"tc\", (.id // \"\"), (.author.login // \"\"), (.createdAt // \"\"), (.body // \"\")])) \
    | @tsv";

/// The author the forge shows for an account that no longer exists.
pub(crate) const GHOST: &str = "ghost";

/// The first 100 threads. `None` is the forge's silence. **No threads is `Some(vec![])`**, which
/// is an answer, where `crate::delivery::asked_lines` would call it silence.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> Option<Vec<ReviewThreadFacts>> {
    let number = format!("number={pull_request}");
    let query = format!("query={QUERY}");
    let run = read_in(
        in_repo,
        FORGE,
        &[
            "api",
            "graphql",
            "-F",
            "owner={owner}",
            "-F",
            "name={repo}",
            "-F",
            &number,
            "-f",
            &query,
            "--jq",
            REDUCTION,
        ],
    )
    .ok()?;
    if !run.status.success() {
        return None;
    }
    let lines: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    Some(threads_folded(&lines))
}

/// The records the reduction printed, as threads. Split from [`read`] so a test hands it what jq
/// prints and needs no forge. A thread with nothing said on it is dropped: it has no first
/// comment to draw, and the forge does not keep one.
pub(crate) fn threads_folded(lines: &[String]) -> Vec<ReviewThreadFacts> {
    let mut threads: Vec<ReviewThreadFacts> = Vec::new();
    for line in lines {
        let mut field = line.split('\t').map(as_written);
        let tag = field.next().unwrap_or_default();
        let mut next = || field.next().unwrap_or_default();
        match tag.as_str() {
            "thread" => {
                let (id, resolved, outdated, path, line, side) =
                    (next(), next(), next(), next(), next(), next());
                threads.push(ReviewThreadFacts {
                    id,
                    path,
                    line: line
                        .parse::<u64>()
                        .ok()
                        .filter(|_| outdated != "true"),
                    side: match side.as_str() {
                        "LEFT" => DiffSide::Left,
                        _ => DiffSide::Right,
                    },
                    resolved: resolved == "true",
                    comments: Vec::new(),
                });
            }
            "tc" => {
                let (id, author, at, body) = (next(), next(), next(), next());
                if let Some(thread) = threads.last_mut() {
                    thread.comments.push(ThreadSaid {
                        id,
                        author: if author.is_empty() { GHOST.to_string() } else { author },
                        at,
                        body,
                    });
                }
            }
            _ => {}
        }
    }
    threads.retain(|thread| !thread.comments.is_empty());
    threads
}

const RESOLVE: &str = "mutation($thread:ID!){resolveReviewThread(input:{threadId:$thread})\
    {thread{isResolved}}}";
const UNRESOLVE: &str = "mutation($thread:ID!){unresolveReviewThread(input:{threadId:$thread})\
    {thread{isResolved}}}";

/// Resolve or reopen a thread. **A write to the forge**, so the refusal is its own sentence and
/// nothing here retries. Answers whether the thread is now resolved, as the forge says.
pub(crate) fn resolve(in_repo: &str, thread: &str, resolved: bool) -> Result<bool, String> {
    let (mutation, picked) = match resolved {
        true => (RESOLVE, ".data.resolveReviewThread.thread.isResolved"),
        false => (UNRESOLVE, ".data.unresolveReviewThread.thread.isResolved"),
    };
    let query = format!("query={mutation}");
    let id = format!("thread={thread}");
    let run = run_in(
        in_repo,
        FORGE,
        &["api", "graphql", "-f", &id, "-f", &query, "--jq", picked],
    )
    .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    if !run.status.success() {
        return Err(said(&run));
    }
    match crate::delivery::last_line(&run).as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("the forge did not say whether the thread is resolved".to_string()),
    }
}
