//! Every comment on a pull request, for the file Fleet keeps in the holding Job's scratch folder.
//!
//! One `api graphql` read, reduced by `--jq` to tagged `@tsv` records as
//! `crate::pull_request_threads` does: `c` for the conversation's, `r` for a review's body, then
//! a `thread` record and a `tc` for each comment on it. **Never `gh pr view`**, which gives no
//! edit time and cannot tell a bot, and a notice is decided on both.

use adapter_traits::{OnFile, PullComment};

use crate::delivery::{read_in, FORGE};
use crate::pull_request_detail::{by_a_bot, edited};
use crate::under_review::as_written;

/// Comments and reviews from the end, as the page keeps the latest.
pub(crate) const QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){\
    repository(owner:$owner,name:$name){pullRequest(number:$number){\
    comments(last:100){nodes{id url author{__typename login} createdAt lastEditedAt body}} \
    reviews(last:100){nodes{id url author{__typename login} submittedAt createdAt lastEditedAt body}} \
    reviewThreads(first:100){nodes{isResolved isOutdated path line \
    comments(first:50){nodes{id url author{__typename login} createdAt lastEditedAt body}}}}}}}";

/// `// ""` on every field so a `null` cannot stop jq mid-stream. A review not yet submitted has
/// no `submittedAt`, so its `createdAt` stands in.
const REDUCTION: &str = "\
    .data.repository.pullRequest as $pull | \
    ($pull.comments.nodes[]? | \
      [\"c\", (.id // \"\"), (.author.login // \"\"), (.author.__typename // \"\"), (.createdAt // \"\"), \
       (.lastEditedAt // \"\"), (.url // \"\"), (.body // \"\")] | @tsv), \
    ($pull.reviews.nodes[]? | \
      [\"r\", (.id // \"\"), (.author.login // \"\"), (.author.__typename // \"\"), \
       (.submittedAt // .createdAt // \"\"), (.lastEditedAt // \"\"), (.url // \"\"), (.body // \"\")] | @tsv), \
    ($pull.reviewThreads.nodes[]? | \
      ([\"thread\", ((.isResolved // false) | tostring), ((.isOutdated // false) | tostring), \
        (.path // \"\"), ((.line // \"\") | tostring)], \
       (.comments.nodes[]? | \
        [\"tc\", (.id // \"\"), (.author.login // \"\"), (.author.__typename // \"\"), (.createdAt // \"\"), \
         (.lastEditedAt // \"\"), (.url // \"\"), (.body // \"\")])) | @tsv)";

/// `None` is the forge's silence. **No comments is `Some(vec![])`**, which is an answer.
pub(crate) fn read(in_repo: &str, number: u64) -> Option<Vec<PullComment>> {
    let number = format!("number={number}");
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
    Some(comments_folded(&lines))
}

/// The records the reduction printed, as comments. Split from [`read`] so a test hands it what jq
/// prints and needs no forge. A review with no body says nothing to file, and is dropped.
pub(crate) fn comments_folded(lines: &[String]) -> Vec<PullComment> {
    let mut out = Vec::new();
    let mut on: Option<OnFile> = None;
    for line in lines {
        let mut field = line.split('\t').map(as_written);
        let tag = field.next().unwrap_or_default();
        let mut next = || field.next().unwrap_or_default();
        if tag == "thread" {
            let (resolved, outdated, path, line) = (next(), next(), next(), next());
            on = Some(OnFile {
                path,
                line: line.parse::<u64>().ok().filter(|_| outdated != "true"),
                resolved: resolved == "true",
            });
            continue;
        }
        let file = match tag.as_str() {
            "c" | "r" => None,
            "tc" => on.clone(),
            _ => continue,
        };
        let (id, author, kind, at, edited_at, url, body) =
            (next(), next(), next(), next(), next(), next(), next());
        if tag == "r" && body.trim().is_empty() {
            continue;
        }
        out.push(PullComment {
            id,
            bot: by_a_bot(&kind, &author),
            author: if author.is_empty() { crate::pull_request_threads::GHOST.to_string() } else { author },
            at,
            edited_at: edited(edited_at),
            url,
            body,
            file,
        });
    }
    out
}
