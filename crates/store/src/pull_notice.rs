//! What Fleet has told the owner of a pull request, and which pull requests the merge queue held
//! when it last looked. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **One row a (pull request, commit, trigger, recipient)**, which is what makes a notice
//! once-only: the insert is refused when the row is there, so a restart reads the same answer.
//! The recipient is plain text (`session:<id>` or `job:<id>`) and no foreign key, as in
//! `main_fix`: a Job `armada clean` forgets leaves its row.

use core_model::{JobId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

impl Store {
    /// Keep that `recipient` was told of `trigger` on `pull` at `head`. **`false` where it
    /// already was**, and nothing is written.
    pub fn keep_pull_notice(
        &mut self,
        repository: &str,
        pull: u64,
        head: &str,
        trigger: &str,
        recipient: &str,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "INSERT INTO pull_notices (repository, pull, head, cause, recipient, noted_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT DO NOTHING",
                (
                    repository,
                    pull as i64,
                    head,
                    trigger,
                    recipient,
                    at.as_str(),
                ),
            )
            .map_err(fault("keeping a pull request notice"))?;
        Ok(kept == 1)
    }

    /// Whether `recipient` was told of `trigger` on `pull` at `head`.
    pub fn pull_notice_kept(
        &self,
        repository: &str,
        pull: u64,
        head: &str,
        trigger: &str,
        recipient: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM pull_notices WHERE repository = ?1 AND pull = ?2 \
                 AND head = ?3 AND cause = ?4 AND recipient = ?5)",
                (repository, pull as i64, head, trigger, recipient),
                |row| row.get::<_, bool>(0),
            )
            .map_err(fault("reading a pull request notice"))
            .map_err(WriteError::Database)
    }

    /// The pull requests the merge queue held when last read.
    pub fn pulls_seen_queued(&self, repository: &str) -> Result<Vec<u64>, WriteError> {
        let doing = "reading the merge queue last seen";
        let mut asking = self
            .conn
            .prepare("SELECT pull FROM pull_queue_seen WHERE repository = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = asking
            .query_map((repository,), |row| row.get::<_, i64>(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows.into_iter().map(|number| number as u64).collect())
    }

    /// Replace the pull requests the merge queue held with `queued`.
    pub fn keep_pulls_queued(
        &mut self,
        repository: &str,
        queued: &[u64],
    ) -> Result<(), WriteError> {
        let doing = "keeping the merge queue last seen";
        let tx = self
            .conn
            .transaction()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM pull_queue_seen WHERE repository = ?1",
            (repository,),
        )
        .map_err(fault(doing))
        .map_err(WriteError::Database)?;
        for number in queued {
            tx.execute(
                "INSERT INTO pull_queue_seen (repository, pull) VALUES (?1, ?2)",
                (repository, *number as i64),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The Jobs whose worktree branch this is, newest first. **A branch name alone cannot name a
    /// repository**, so a caller that serves more than one keeps only its own.
    pub fn jobs_on_branch(&self, branch: &str) -> Result<Vec<JobId>, LoadJobError> {
        let reading = fault("finding the Jobs on a branch");
        let mut asking = self
            .conn
            .prepare("SELECT job_id FROM jobs WHERE branch = ?1 ORDER BY job_id DESC")
            .map_err(reading)
            .map_err(|cause| LoadJobError::Unreadable(crate::error::RowError::Database(cause)))?;
        asking
            .query_map((branch,), |row| {
                Ok(JobId::carried(Ulid::carried(row.get::<_, String>(0)?)))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("finding the Jobs on a branch"))
            .map_err(|cause| LoadJobError::Unreadable(crate::error::RowError::Database(cause)))
    }
}
