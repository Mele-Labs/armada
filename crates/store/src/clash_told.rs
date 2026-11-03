//! Each write-clash notice Fleet sent a Job, so a restart, which refills the clash table from the
//! worktrees, does not send it again. `docs/concepts/fleet.md`, *Write clashes*.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Whether `session` was sent these words already.
    pub fn clash_told_before(&self, session: &str, said: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM clash_told WHERE session_id = ?1 AND said = ?2)",
                (session, said),
                |row| row.get::<_, bool>(0),
            )
            .map_err(fault("reading whether a Job was told of a clash"))
            .map_err(WriteError::Database)
    }

    /// Keep that `session` was sent these words.
    pub fn keep_clash_told(&mut self, session: &str, said: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO clash_told (session_id, said, told_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                (session, said, at.as_str()),
            )
            .map(|_| ())
            .map_err(fault("keeping that a Job was told of a clash"))
            .map_err(WriteError::Database)
    }

    /// The files `session` was already told it conflicts on with `peer`.
    pub fn clash_paths_told(&self, session: &str, peer: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading which files a Job was told it clashes on";
        let mut statement = self
            .conn
            .prepare("SELECT path FROM clash_told_paths WHERE session_id = ?1 AND peer_id = ?2")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map((session, peer), |row| row.get::<_, String>(0))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep that `session` was told it conflicts with `peer` on these files.
    pub fn keep_clash_paths_told(
        &mut self,
        session: &str,
        peer: &str,
        paths: &[String],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        for path in paths {
            self.conn
                .execute(
                    "INSERT INTO clash_told_paths (session_id, peer_id, path, told_at) VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT DO NOTHING",
                    (session, peer, path, at.as_str()),
                )
                .map_err(fault("keeping which files a Job was told it clashes on"))
                .map_err(WriteError::Database)?;
        }
        Ok(())
    }
}
