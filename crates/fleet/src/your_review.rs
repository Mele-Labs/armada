//! His own latest review of a pull request and what came since it: the page's *Your review* bar,
//! and the review each row of the lists carries. `docs/concepts/pull-review.md`, *Your review*.
//!
//! **A review Fleet just wrote is served until the forge shows one as new.** The forge can answer
//! the read right after a write without it, so Fleet keeps what it wrote and the newer of the two
//! wins.

use std::collections::{BTreeMap, HashMap};

use adapter_traits::{
    AgentHarness, Delivery, LatestReview, PullRequestPageFacts, PullRequestWrite, ReviewSaid, Said, Vcs, WorkProduct,
};
use core_model::Timestamp;
use ipc::{ManifestId, MyReviewSaid, PullMyReview, PullRequestPage, PullReviewRow, PullSinceMyReview};
use store::KeptWrittenReview;

use crate::daemon::Fleet;
use crate::pull_review_memory::my_review_of;

/// Who said something and when, and whether an app said it.
struct Spoken {
    author: String,
    at: String,
    bot: bool,
}

impl Spoken {
    /// A person, not an app: the forge says which, and a login ending `[bot]` is an app's either way.
    fn by_a_person(&self) -> bool {
        !self.bot && !self.author.to_ascii_lowercase().ends_with("[bot]")
    }
}

/// What the page's read says about reviews, kept apart from the page so the bar can be worked out
/// once Fleet knows who he is.
pub(crate) struct ReviewedOn {
    latest: BTreeMap<String, LatestReview>,
    /// The commits' short names, oldest first.
    commits: Vec<String>,
    /// Commits past the last one read.
    commits_cut: usize,
    /// Each review thread's comments.
    threads: Vec<Vec<Spoken>>,
    /// What was said on the conversation, with what it said.
    conversation: Vec<(Spoken, String)>,
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
                .map(|thread| {
                    thread.comments.iter().map(|it| Spoken { author: it.author.clone(), at: it.at.clone(), bot: it.bot }).collect()
                })
                .collect(),
            conversation: facts
                .conversation
                .iter()
                .map(|said| match said {
                    Said::Comment { author, at, bot, body, .. } | Said::Review { author, at, bot, body, .. } => {
                        (Spoken { author: author.clone(), at: at.clone(), bot: *bot }, body.clone())
                    }
                })
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

/// Whether `body` mentions `login` as the forge links one: `@login`, ending where a login cannot go on.
fn mentions(body: &str, login: &str) -> bool {
    let (body, at) = (body.to_ascii_lowercase(), format!("@{}", login.to_ascii_lowercase()));
    body.match_indices(&at).any(|(start, _)| {
        let after = body[start + at.len()..].chars().next();
        !after.is_some_and(|next| next.is_ascii_alphanumeric() || next == '-')
    })
}

/// What came after `mine`: the commits after the one it was written on, and what a person other
/// than him said after it, on the review threads he wrote on or on the conversation naming him.
/// **An app's comment is no reply.**
pub(crate) fn since_of(on: &ReviewedOn, logins: &[String], mine: &PullMyReview) -> PullSinceMyReview {
    let commits = mine.commit.as_deref().and_then(|commit| {
        let at = on.commits.iter().position(|sha| !sha.is_empty() && commit.starts_with(sha.as_str()))?;
        Some((on.commits.len() - at - 1 + on.commits_cut) as u64)
    });
    let replies = instant(mine).map(|after| {
        let a_reply = |it: &Spoken| {
            it.by_a_person()
                && !is_his(&it.author, logins)
                && Timestamp::from_rfc3339(it.at.clone()).epoch_millis().is_some_and(|at| at > after)
        };
        let on_threads = on
            .threads
            .iter()
            .filter(|thread| thread.iter().any(|it| is_his(&it.author, logins)))
            .flatten()
            .filter(|it| a_reply(it))
            .count();
        let naming_him = on
            .conversation
            .iter()
            .filter(|(it, body)| a_reply(it) && logins.iter().any(|login| mentions(body, login)))
            .count();
        (on_threads + naming_him) as u64
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
