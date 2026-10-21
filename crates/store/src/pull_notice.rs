//! What Fleet has told the owner of a pull request, and which pull requests the merge queue held
//! when it last looked. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.
//!
//! **One row a (pull request, commit, trigger, recipient)**, which is what makes a notice
//! once-only: the insert is refused when the row is there, so a restart reads the same answer.
//! The recipient is plain text (`session:<id>`) and no foreign key, as in `main_fix`.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
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

    /// Forget every `trigger` notice on `pull`, at any commit: what was told is no longer news
    /// once the thing it told of is undone, as a queue entry undoes a drop.
    pub fn forget_pull_notices(
        &mut self,
        repository: &str,
        pull: u64,
        trigger: &str,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM pull_notices WHERE repository = ?1 AND pull = ?2 AND cause = ?3",
                (repository, pull as i64, trigger),
            )
            .map_err(fault("forgetting a pull request notice"))
            .map_err(WriteError::Database)?;
        Ok(())
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

}
