//! Noticing that the issue a Job came from was edited after Fleet read it.
//! Spike 022, answer 5, the owner's call of 1 Oct 2026: **the Job keeps the
//! words it froze, and says the issue has moved since.**
//!
//! | | |
//! |---|---|
//! | The set | Every Job in flight whose request linked an issue |
//! | Forge cost | One issue read an interval, however many such Jobs there are |
//! | How fresh | Each such Job is asked once every interval times their number |
//! | Pull requests | Unchanged: `crate::noticing` keeps its own cursor and call |
//!
//! **A cursor of its own on the same interval.** Sharing the pull requests'
//! would slow each of their readings by the number of issue-linked Jobs, a
//! cadence that is the owner's (`docs/concepts/fleet.md`).

use adapter_traits::{AgentHarness, Delivery, IssueAddress, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, IssueSource, JobId, Level, Timestamp};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;

/// Where the issue rotation stands, and when it last ran. **In memory**, for
/// `crate::noticing::Sweep`'s reason: a position in a list that has since
/// changed is worth nothing after a restart.
#[derive(Clone, Debug, Default)]
pub(crate) struct IssueSweep {
    last: Option<Timestamp>,
    next: usize,
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
    /// Ask the forge when one issue-linked Job's issue was last edited, if it
    /// is time to, and keep it where that is news.
    ///
    /// **`None` is nearly every turn**: too soon, no such Job, a forge that
    /// will not answer, or an answer that is not news. A forge that will not
    /// answer is held, never raised, for `notice_a_merge`'s reason.
    pub(crate) async fn notice_an_issue(&self) -> Result<Option<JobId>, Adrift> {
        let Some((job, source)) = self.issue_due().await? else {
            return Ok(None);
        };
        let address = IssueAddress::at(&source.reference, &source.url);
        let Some(call) = self.links().edited(&address) else {
            return Ok(None);
        };
        let Ok(said) = crate::proposal::resolved(&call).await else {
            return Ok(None);
        };
        let Some(moved) = source.edited(&Timestamp::from_rfc3339(said.trim())) else {
            return Ok(None);
        };
        self.store()
            .lock()
            .await
            .record_issue_source(&job, &moved)
            .map_err(Adrift::Writing)?;
        self.noted_issue_moved(&job, &moved);
        Ok(Some(job))
    }

    /// The one issue-linked Job to ask about this turn, or why there is not
    /// one. `crate::noticing`'s `due_to_ask`, over its own set and cursor.
    async fn issue_due(&self) -> Result<Option<(JobId, IssueSource)>, Adrift> {
        let now = self.now();
        let mut sweep = self.issue_sweeping().lock().await;
        if let Some(last) = sweep.last.as_ref() {
            if elapsed(last, &now) < self.noticing().interval() {
                return Ok(None);
            }
        }
        // Stamped whether or not there is anything to ask, for
        // `due_to_ask`'s reason: an empty set is not a query each turn.
        sweep.last = Some(now);
        let waiting = self
            .store()
            .lock()
            .await
            .issue_sources_in_flight()
            .map_err(Adrift::Reading)?;
        if waiting.is_empty() {
            return Ok(None);
        }
        let at = sweep.next % waiting.len();
        sweep.next = at + 1;
        Ok(waiting.into_iter().nth(at))
    }

    /// The line on the Job's own log that says so, once per edit noticed.
    fn noted_issue_moved(&self, job: &JobId, moved: &IssueSource) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the issue this job's criteria were read from has been edited since; the job keeps \
             the words it froze",
        )
        .in_job(job.as_ulid().clone())
        .with_field("issue", FieldValue::Str(moved.reference.clone()))
        .with_field("url", FieldValue::Str(moved.url.clone()));
        if let Some(at) = &moved.moved_at {
            envelope = envelope.with_field("edited_at", FieldValue::Str(at.as_str().to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }
}
