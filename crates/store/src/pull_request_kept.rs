//! A pull request's title and comment count, kept past the merge.
//!
//! **Written down because the sweep stops asking.** Fleet's rotation reads the
//! forge about every open pull request, and what it read lived only in memory:
//! erased the moment the pull request settled, and on every restart. A Job
//! whose pull request merged had no title to show, and one that merged before
//! the rotation first reached it never had one at all.

use core_model::JobId;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 96 — the pull request's title and how many comments it carries.
///
/// **Null, and no backfill.** A null title is a pull request no read has named
/// since this column arrived; a null count is one the sweep never read open.
/// Neither is zero, and nothing here guesses one.
pub(crate) const V96: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_pr_title TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pr_comments INTEGER;
"#;

/// What the record holds about a Job's pull request beyond its address.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptPullRequest {
    /// The title, as the last read of it said.
    pub title: Option<String>,
    /// How many comments the sweep last counted on it while it was open.
    pub comments: Option<u32>,
}

impl Store {
    /// Write the title Fleet just opened a pull request with, and forget any
    /// count: **a new pull request starts unread**, and a redispatched Job's
    /// last count belongs to the pull request it opened before.
    pub fn record_pull_request_opened(
        &mut self,
        job_id: &JobId,
        title: &str,
    ) -> Result<(), WriteError> {
        self.pull_request_updated(
            job_id,
            "UPDATE jobs SET delivery_pr_title = ?2, delivery_pr_comments = NULL \
             WHERE job_id = ?1",
            (job_id.as_str(), title),
        )
    }

    /// Write what a read of the forge said. **`None` keeps what was there**:
    /// a forge that did not answer one of the two is not a pull request that
    /// lost its title or its comments.
    pub fn record_pull_request_read(
        &mut self,
        job_id: &JobId,
        title: Option<&str>,
        comments: Option<u32>,
    ) -> Result<(), WriteError> {
        self.pull_request_updated(
            job_id,
            "UPDATE jobs SET delivery_pr_title = COALESCE(?2, delivery_pr_title), \
             delivery_pr_comments = COALESCE(?3, delivery_pr_comments) WHERE job_id = ?1",
            (job_id.as_str(), title, comments.map(i64::from)),
        )
    }

    /// What the record holds. Both absent for a Job the file does not hold, as
    /// [`delivery_for`](Store::delivery_for) reads one as nothing to say.
    pub fn kept_pull_request_for(
        &self,
        job_id: &JobId,
    ) -> Result<KeptPullRequest, crate::error::LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_pr_title, delivery_pr_comments FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    let comments: Option<i64> = row.get(1)?;
                    Ok(KeptPullRequest {
                        title: row.get(0)?,
                        comments: comments.and_then(|count| u32::try_from(count).ok()),
                    })
                },
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(KeptPullRequest::default()),
                other => Err(crate::error::LoadJobError::Unreadable(
                    crate::error::RowError::Database(fault(
                        "reading a pull request's title and comments",
                    )(other)),
                )),
            })
    }

    fn pull_request_updated(
        &mut self,
        job_id: &JobId,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(sql, params)
            .map_err(fault("recording a pull request's title and comments"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }
}
