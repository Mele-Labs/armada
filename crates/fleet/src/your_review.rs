//! His own latest review of a pull request and what came since it: the page's *Your review* bar,
//! and the review each row of the lists carries. `docs/concepts/pull-review.md`, *Your review*.
//!
//! **A review Fleet just wrote is served until the forge shows one as new.** The forge can answer
//! the read right after a write without it, so Fleet keeps what it wrote and the newer of the two
//! wins.

use std::collections::{BTreeMap, HashMap};

use adapter_traits::{
    AgentHarness, Delivery, LatestReview, PullRequestPageFacts, PullRequestWrite, ReviewSaid, Vcs, WorkProduct,
};
use core_model::Timestamp;
use ipc::{ManifestId, MyReviewSaid, PullMyReview, PullRequestPage, PullReviewRow, PullSinceMyReview};
use store::KeptWrittenReview;

use crate::daemon::Fleet;
use crate::pull_review_memory::my_review_of;

/// What the page's read says about reviews, kept apart from the page so the bar can be worked out
/// once Fleet knows who he is.
pub(crate) struct ReviewedOn {
    latest: BTreeMap<String, LatestReview>,
    /// The commits' short names, oldest first.
    commits: Vec<String>,
    /// Commits past the last one read.
    commits_cut: usize,
    /// Each review thread's comments, as author and time.
    threads: Vec<Vec<(String, String)>>,
}

impl ReviewedOn {
    pub(crate) fn of(facts: &PullRequestPageFacts) -> Self {
        Self {
            latest: facts.latest_reviews.clone(),
            commits: facts.commits.iter().map(|it| it.sha.clone()).collect(),
            commits_cut: facts.omitted.commits,
            threads: facts
                .threads
                .iter()
                .map(|thread| thread.comments.iter().map(|it| (it.author.clone(), it.at.clone())).collect())
                .collect(),
        }
    }
}

fn word_of(said: MyReviewSaid) -> &'static str {
    match said {
        MyReviewSaid::Approved => "approved",
        MyReviewSaid::ChangesRequested => "changes_requested",
        MyReviewSaid::Commented => "commented",
    }
}

fn said_of(word: &str) -> Option<MyReviewSaid> {
    match word {
        "approved" => Some(MyReviewSaid::Approved),
        "changes_requested" => Some(MyReviewSaid::ChangesRequested),
        "commented" => Some(MyReviewSaid::Commented),
        _ => None,
    }
}

fn review_of(kept: &KeptWrittenReview) -> Option<PullMyReview> {
    Some(PullMyReview {
        said: said_of(&kept.said)?,
        commit: kept.commit.clone(),
        submitted_at: Some(kept.submitted_at.clone()),
    })
}

/// The review a write made, where it made one: a comment with no line comments is not a review.
pub(crate) fn written_said(write: &PullRequestWrite) -> Option<MyReviewSaid> {
    match write {
        PullRequestWrite::Comment { .. } => None,
        PullRequestWrite::Approve { .. } => Some(MyReviewSaid::Approved),
        PullRequestWrite::RequestChanges { .. } => Some(MyReviewSaid::ChangesRequested),
        PullRequestWrite::Review { verdict, .. } => Some(match verdict {
            ReviewSaid::Approved => MyReviewSaid::Approved,
            ReviewSaid::ChangesRequested => MyReviewSaid::ChangesRequested,
            ReviewSaid::Commented => MyReviewSaid::Commented,
        }),
    }
}

fn instant(review: &PullMyReview) -> Option<i64> {
    Timestamp::from_rfc3339(review.submitted_at.clone()?).epoch_millis()
}

/// The forge's review, unless the one Fleet wrote was submitted after it.
pub(crate) fn newer_of(forge: Option<PullMyReview>, written: Option<PullMyReview>) -> Option<PullMyReview> {
    match (forge, written) {
        (Some(forge), Some(written)) if instant(&forge) < instant(&written) => Some(written),
        (Some(forge), _) => Some(forge),
        (None, written) => written,
    }
}

fn is_his(login: &str, logins: &[String]) -> bool {
    logins.iter().any(|mine| mine.eq_ignore_ascii_case(login))
}

/// What came after `mine`: the commits after the one it was written on, and the comments someone
/// else wrote after it on the threads he wrote on.
pub(crate) fn since_of(on: &ReviewedOn, logins: &[String], mine: &PullMyReview) -> PullSinceMyReview {
    let commits = mine.commit.as_deref().and_then(|commit| {
        let at = on.commits.iter().position(|sha| !sha.is_empty() && commit.starts_with(sha.as_str()))?;
        Some((on.commits.len() - at - 1 + on.commits_cut) as u64)
    });
    let replies = instant(mine).map(|after| {
        on.threads
            .iter()
            .filter(|thread| thread.iter().any(|(author, _)| is_his(author, logins)))
            .flatten()
            .filter(|(author, at)| {
                !is_his(author, logins) && Timestamp::from_rfc3339(at.clone()).epoch_millis().is_some_and(|at| at > after)
            })
            .count() as u64
    });
    PullSinceMyReview { commits, replies }
}

/// His latest review on the page and what came since, where he has one.
pub(crate) fn your_review_of(
    on: &ReviewedOn,
    logins: &[String],
    written: Option<PullMyReview>,
) -> Option<(PullMyReview, PullSinceMyReview)> {
    let forge = on.latest.iter().find(|(login, _)| is_his(login, logins)).map(|(_, it)| my_review_of(it.clone()));
    let mine = newer_of(forge, written)?;
    let since = since_of(on, logins, &mine);
    Some((mine, since))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every review Fleet wrote, by Manifest, repository lowercase and number. Unreadable is none.
    async fn written_reviews(&self) -> HashMap<(String, String, u64), PullMyReview> {
        let kept = self.store().lock().await.written_reviews().unwrap_or_default();
        kept.iter()
            .filter_map(|it| Some(((it.manifest_id.clone(), it.repository.to_lowercase(), it.number), review_of(it)?)))
            .collect()
    }

    /// The page's *Your review* bar. Where Fleet does not know who he is, there is none.
    pub(crate) async fn your_review_onto(&self, page: &mut PullRequestPage, on: &ReviewedOn, root: &str) {
        let mine = self.mine_in(root).await;
        if mine.logins().is_empty() {
            return;
        }
        let slug = self.walk_slug(root, &page.manifest_id).await.to_lowercase();
        let written = self.written_reviews().await.remove(&(page.manifest_id.as_str().to_string(), slug, page.number));
        (page.my_review, page.since_my_review) = your_review_of(on, mine.logins(), written).unzip();
    }

    /// Each row's review, the one Fleet wrote where that is newer than the forge's.
    pub(crate) async fn written_onto(&self, rows: &mut [PullReviewRow]) {
        let mut written = self.written_reviews().await;
        for row in rows {
            let key = (row.manifest_id.as_str().to_string(), row.repository.to_lowercase(), row.number);
            row.my_review = newer_of(row.my_review.take(), written.remove(&key));
        }
    }

    /// Keep the review a write just made, at the commit it was written on: the one the person read,
    /// else the head the forge shows now. Answered with what was kept.
    pub(crate) async fn review_written(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
        said: MyReviewSaid,
        commit: Option<String>,
    ) -> Option<PullMyReview> {
        let kept = KeptWrittenReview {
            manifest_id: manifest_id.as_str().to_string(),
            repository: self.walk_slug(root, manifest_id).await.to_lowercase(),
            number,
            said: word_of(said).to_string(),
            commit,
            submitted_at: self.clock().now().as_str().to_string(),
        };
        // **Best effort**: the write was taken, and an unkept one leaves the forge's read to say it.
        self.store().lock().await.keep_written_review(&kept).ok()?;
        review_of(&kept)
    }
}
