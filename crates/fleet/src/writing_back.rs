//! What Fleet writes on the issue a Job came from: that the Job started, the
//! pull request it opened, and a label that follows it. #91, the owner's call
//! of 11 Oct 2026; `docs/concepts/tracker.md`.
//!
//! **A pass, not a hook on each transition.** Every interval it reads each Job
//! that came from an issue, compares what its issue should say with what the
//! store says was written, and writes the difference. A refused write is
//! kept, said on the Job's log and tried again later; **it never fails, holds
//! or slows a Job**, because nothing a Job does waits on this.
//!
//! **Only Armada's own labels move**, one at a time, and only one Armada set.
//! A label somebody set by hand under the same name is the one cost the owner
//! accepted.

use std::time::Duration;

use adapter_traits::{
    AgentHarness, Delivery, IssueAddress, NotTracked, Review, Vcs, WorkProduct,
};
use core_model::{Component, Envelope, FieldValue, Job, JobStatus, Level};
use store::{WriteBackOwed, WrittenBack};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;

/// While a Job works the issue.
pub(crate) const IN_PROGRESS: &str = "armada: in progress";
/// While a Job waits on a person: escalated, or its retries spent.
pub(crate) const SET_ASIDE: &str = "armada: set aside";

/// How long a refused write waits before it is tried again: long enough that a
/// tracker refusing every write is not asked every few seconds.
pub(crate) const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);

/// Armada's two labels, by what the store keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Label {
    InProgress,
    SetAside,
}

impl Label {
    fn stored(self) -> &'static str {
        match self {
            Label::InProgress => "in_progress",
            Label::SetAside => "set_aside",
        }
    }

    fn from_stored(word: &str) -> Option<Label> {
        match word {
            "in_progress" => Some(Label::InProgress),
            "set_aside" => Some(Label::SetAside),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Label::InProgress => IN_PROGRESS,
            Label::SetAside => SET_ASIDE,
        }
    }
}

/// One comment owed, by what it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Said {
    Started(String),
    PullRequest { url: String, text: String },
}

impl Said {
    pub(crate) fn text(&self) -> &str {
        match self {
            Said::Started(text) | Said::PullRequest { text, .. } => text,
        }
    }

    /// The store's row once this comment is on the issue.
    fn kept_in(&self, so_far: &mut WrittenBack) {
        match self {
            Said::Started(_) => so_far.started = true,
            Said::PullRequest { url, .. } => so_far.pull_request = Some(url.clone()),
        }
    }
}

/// What one issue is owed, and what the store says once it is all written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Owed {
    pub comments: Vec<Said>,
    pub add: Option<Label>,
    pub remove: Option<Label>,
    pub next: WrittenBack,
}

impl Owed {
    fn is_nothing(&self, kept: &WrittenBack) -> bool {
        self.comments.is_empty()
            && self.add.is_none()
            && self.remove.is_none()
            && self.next == *kept
    }
}

/// The label a Job's status puts on its issue. `None` inside is none of
/// Armada's; the outer `None` is not started, which moves nothing.
fn label_for(status: JobStatus) -> Option<Option<Label>> {
    match status {
        JobStatus::Proposing | JobStatus::AwaitingApproval | JobStatus::Queued => None,
        JobStatus::Escalated | JobStatus::AwaitingRepair => Some(Some(Label::SetAside)),
        status if status.is_terminal() => Some(None),
        _ => Some(Some(Label::InProgress)),
    }
}

/// What the issue `row` names is owed now. **Pure**, so every status is a
/// case a test names.
pub(crate) fn owed(row: &WriteBackOwed) -> Owed {
    let kept = &row.kept;
    let mut next = WrittenBack {
        failed: None,
        failed_at: None,
        ..kept.clone()
    };
    let mut comments = Vec::new();
    let Some(status) = JobStatus::from_wire(&row.status) else {
        return Owed { comments, add: None, remove: None, next: kept.clone() };
    };
    let wanted = label_for(status);
    let working = !status.is_terminal() && wanted.is_some();
    if working && !kept.started {
        comments.push(Said::Started(format!(
            "Armada started job {} on this issue: {}",
            row.number,
            row.title.trim()
        )));
        next.started = true;
    }
    if let Some(url) = row.pull_request.as_deref() {
        if kept.pull_request.as_deref() != Some(url) {
            comments.push(Said::PullRequest {
                url: url.to_string(),
                text: format!("Job {} opened a pull request for this issue: {url}", row.number),
            });
            next.pull_request = Some(url.to_string());
        }
    }
    let had = kept.label.as_deref().and_then(Label::from_stored);
    let (add, remove) = match wanted {
        Some(want) if want != had => {
            next.label = want.map(|label| label.stored().to_string());
            (want, had)
        }
        _ => (None, None),
    };
    next.settled = status.is_terminal();
    Owed {
        comments,
        add,
        remove,
        next,
    }
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
    /// Write back on every issue that is owed it, if a noticing interval has
    /// passed since the last pass. `crate::notice_loop` rides it.
    pub(crate) async fn write_back_when_due(&self) -> Result<usize, Adrift> {
        let now = self.now();
        {
            let mut last = self.tracking().written_back.lock().await;
            if let Some(at) = last.as_ref() {
                if elapsed(at, &now) < self.noticing().interval() {
                    return Ok(0);
                }
            }
            *last = Some(now);
        }
        self.write_back().await
    }

    /// One pass over every unsettled Job that came from an issue. Answers how
    /// many issues were written on.
    pub(crate) async fn write_back(&self) -> Result<usize, Adrift> {
        let rows = self
            .store()
            .lock()
            .await
            .issue_writebacks_open()
            .map_err(Adrift::Reading)?;
        let now = self.now();
        let mut written = 0;
        for row in rows {
            if let Some(at) = row.kept.failed_at.as_ref() {
                if elapsed(at, &now) < RETRY_AFTER {
                    continue;
                }
            }
            let owed = owed(&row);
            if owed.is_nothing(&row.kept) {
                continue;
            }
            let wrote = !owed.comments.is_empty() || owed.add.is_some() || owed.remove.is_some();
            let kept = match self.written_on_the_issue(&row, owed).await {
                Ok(next) => next,
                Err((next, not)) => {
                    self.noted_write_back_refused(&row, &not);
                    WrittenBack {
                        failed: Some(format!("{} did not happen: {}", not.doing, not.said)),
                        failed_at: Some(self.now()),
                        ..next
                    }
                }
            };
            self.store()
                .lock()
                .await
                .record_written_back(&row.job_id, &kept)
                .map_err(Adrift::Writing)?;
            written += usize::from(wrote && kept.failed.is_none());
        }
        Ok(written)
    }

    /// Write what is owed, in order, off the async thread. On a refusal,
    /// answers what had been written before it, so nothing is said twice.
    async fn written_on_the_issue(
        &self,
        row: &WriteBackOwed,
        owed: Owed,
    ) -> Result<WrittenBack, (WrittenBack, NotTracked)> {
        let tracker = self.tracking().tracker();
        let issue = IssueAddress::at(&row.source.reference, &row.source.url);
        let before = row.kept.clone();
        tokio::task::spawn_blocking(move || {
            let mut so_far = WrittenBack {
                failed: None,
                failed_at: None,
                ..before
            };
            for said in &owed.comments {
                tracker
                    .comment(&issue, said.text())
                    .map_err(|not| (so_far.clone(), not))?;
                said.kept_in(&mut so_far);
            }
            let add: Vec<&str> = owed.add.iter().map(|label| label.name()).collect();
            let remove: Vec<&str> = owed.remove.iter().map(|label| label.name()).collect();
            if !add.is_empty() || !remove.is_empty() {
                tracker
                    .relabel(&issue, &add, &remove)
                    .map_err(|not| (so_far.clone(), not))?;
            }
            Ok(owed.next)
        })
        .await
        .unwrap_or_else(|panicked| {
            Err((
                row.kept.clone(),
                NotTracked::of("writing back on the issue", panicked.to_string()),
            ))
        })
    }

    /// `review` with the line that closes the Job's issue when it merges, in
    /// the tracker's own words. **Unchanged** where the Job came from no issue,
    /// or the store or tracker cannot say: a pull request never waits on this.
    pub(crate) async fn closing_its_issue(&self, job: &Job, review: Review) -> Review {
        let source = self.store().lock().await.issue_source(job.id());
        let Ok(Some(source)) = source else {
            return review;
        };
        let issue = IssueAddress::at(&source.reference, &source.url);
        match self.tracking().tracker().closing_line(&issue) {
            Some(line) => Review::assembled(
                review.title(),
                format!("{}\n\n{line}\n", review.body().trim_end()),
            ),
            None => review,
        }
    }

    /// The line on the Job's own log, once per refused pass.
    fn noted_write_back_refused(&self, row: &WriteBackOwed, not: &NotTracked) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the issue this job came from could not be written on; the job goes on, and Fleet \
             tries again in ten minutes",
        )
        .in_job(row.job_id.as_ulid().clone())
        .with_field("issue", FieldValue::Str(row.source.reference.clone()))
        .with_field("url", FieldValue::Str(row.source.url.clone()))
        .with_field("cause", FieldValue::Str(format!("{} did not happen: {}", not.doing, not.said)));
        self.noted_in_the_log(&row.job_id, &envelope);
    }
}
