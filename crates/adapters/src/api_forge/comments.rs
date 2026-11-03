//! Every comment on a pull request over GitHub's API: `gh`'s query, read into the comments
//! `crate::pull_request_comments::comments_folded` makes from `gh`'s records.

use adapter_traits::{OnFile, PullComment};
use serde::Deserialize;

use super::connection::Named;
use super::page::InRepository;
use super::{author_of, Login, Nodes};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::pull_request_threads::GHOST;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    comments: Option<Nodes<Said>>,
    reviews: Option<Nodes<Said>>,
    review_threads: Option<Nodes<Thread>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    is_resolved: Option<bool>,
    is_outdated: Option<bool>,
    path: Option<String>,
    line: Option<u64>,
    comments: Option<Nodes<Said>>,
}

/// A comment, a review and a comment on the diff, as the query asks for each.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Said {
    id: Option<String>,
    url: Option<String>,
    author: Option<Login>,
    created_at: Option<String>,
    submitted_at: Option<String>,
    last_edited_at: Option<String>,
    body: Option<String>,
}

impl Said {
    fn comment(self, file: Option<OnFile>) -> PullComment {
        let (author, bot) = author_of(self.author);
        PullComment {
            id: self.id.unwrap_or_default(),
            author: if author.is_empty() { GHOST.to_string() } else { author },
            bot,
            at: self.submitted_at.or(self.created_at).unwrap_or_default(),
            edited_at: self.last_edited_at,
            url: self.url.unwrap_or_default(),
            body: self.body.unwrap_or_default(),
            file,
        }
    }
}

/// `Ok(None)` where there is no such pull request.
pub(super) fn read(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<Vec<PullComment>>, ApiFailure> {
    let named = Named::of(repository);
    let found: InRepository<Pull> =
        api.graphql(crate::pull_request_comments::QUERY, &named.asked(number))?;
    Ok(found.pull().map(comments_of))
}

fn comments_of(pull: Pull) -> Vec<PullComment> {
    let mut out: Vec<PullComment> = Nodes::all(pull.comments).map(|said| said.comment(None)).collect();
    out.extend(
        Nodes::all(pull.reviews)
            .filter(|said| said.body.as_deref().is_some_and(|body| !body.trim().is_empty()))
            .map(|said| said.comment(None)),
    );
    for thread in Nodes::all(pull.review_threads) {
        let on = OnFile {
            path: thread.path.unwrap_or_default(),
            line: thread.line.filter(|_| thread.is_outdated != Some(true)),
            resolved: thread.is_resolved == Some(true),
        };
        out.extend(Nodes::all(thread.comments).map(|said| said.comment(Some(on.clone()))));
    }
    out
}
