//! The conversations on a pull request's diff over GitHub's API: `gh`'s query, read into the
//! threads `crate::pull_request_threads::threads_folded` makes from `gh`'s records.

use adapter_traits::{DiffSide, ReviewThreadFacts, ThreadSaid};
use serde::Deserialize;

use super::{login_of, Login, Nodes};
use crate::pull_request_threads::GHOST;

/// `pullRequest` as the threads query asks for it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Threads {
    review_threads: Option<Nodes<Thread>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    id: Option<String>,
    is_resolved: Option<bool>,
    is_outdated: Option<bool>,
    path: Option<String>,
    line: Option<u64>,
    diff_side: Option<String>,
    comments: Option<Nodes<Comment>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comment {
    id: Option<String>,
    author: Option<Login>,
    created_at: Option<String>,
    body: Option<String>,
}

/// An outdated thread names no line, the line having changed under it; a thread with nothing
/// said on it is dropped; a gone author is the forge's ghost.
pub(crate) fn threads(pull: Option<Threads>) -> Vec<ReviewThreadFacts> {
    Nodes::all(pull.and_then(|it| it.review_threads))
        .map(|thread| ReviewThreadFacts {
            id: thread.id.unwrap_or_default(),
            path: thread.path.unwrap_or_default(),
            line: thread.line.filter(|_| thread.is_outdated != Some(true)),
            side: match thread.diff_side.as_deref() {
                Some("LEFT") => DiffSide::Left,
                _ => DiffSide::Right,
            },
            resolved: thread.is_resolved == Some(true),
            comments: Nodes::all(thread.comments)
                .map(|said| {
                    let author = login_of(said.author);
                    ThreadSaid {
                        id: said.id.unwrap_or_default(),
                        author: if author.is_empty() { GHOST.to_string() } else { author },
                        at: said.created_at.unwrap_or_default(),
                        body: said.body.unwrap_or_default(),
                    }
                })
                .collect(),
        })
        .filter(|thread| !thread.comments.is_empty())
        .collect()
}
