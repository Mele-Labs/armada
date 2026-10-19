//! How a repository runs, as Fleet learnt it from its Jobs: the facts, where each has been
//! written, the place a repository's facts go, and the deny list of facts a person dropped.
//! `docs/concepts/kit.md`, *What a repository runs*.
//!
//! **Kinds and places are the words the wire spells them in**, and nothing here knows what one
//! means. A fact is keyed by its workspace, kind and command, so a Job and an import that saw the
//! same line are one row.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One remembered run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRun {
    pub workspace: String,
    pub kind: String,
    pub command: String,
    pub port: Option<u16>,
    pub succeeded: bool,
    /// Milliseconds since the epoch.
    pub last_ran: u64,
    pub source: String,
    /// The place that has taken it.
    pub kept_in: Option<String>,
    /// What it waits on where `kept_in` is empty.
    pub why: Option<String>,
}

const COLUMNS: &str = "workspace, kind, command, port, succeeded, last_ran, source, kept_in, why";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptRun> {
    Ok(KeptRun {
        workspace: row.get(0)?,
        kind: row.get(1)?,
        command: row.get(2)?,
        port: row.get::<_, Option<i64>>(3)?.and_then(|port| u16::try_from(port).ok()),
        succeeded: row.get::<_, i64>(4)? != 0,
        last_ran: row.get::<_, i64>(5)?.max(0) as u64,
        source: row.get(6)?,
        kept_in: row.get(7)?,
        why: row.get(8)?,
    })
}

impl Store {
    /// The place chosen for a repository and the folder `synced` writes under. `None` is no choice.
    pub fn run_memory_place(&self, repository: &str) -> Result<Option<(String, Option<String>)>, WriteError> {
        self.conn
            .query_row(
                "SELECT place, synced_folder FROM run_memory_places WHERE repository = ?1",
                [repository],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(fault("reading where a repository's runs are kept"))
            .map_err(WriteError::Database)
    }

    /// Choose where a repository's runs are written, replacing an earlier choice. Facts already
    /// learnt stand as not yet written there, so the next write brings them.
    pub fn set_run_memory_place(
        &mut self,
        repository: &str,
        place: &str,
        synced_folder: Option<&str>,
    ) -> Result<(), WriteError> {
        let doing = fault("choosing where a repository's runs are kept");
        self.conn
            .execute(
                "INSERT INTO run_memory_places (repository, place, synced_folder) VALUES (?1, ?2, ?3) \
                 ON CONFLICT (repository) DO UPDATE SET place = ?2, synced_folder = ?3",
                params![repository, place, synced_folder],
            )
            .map_err(doing)
            .map_err(WriteError::Database)?;
        self.conn
            .execute(
                "UPDATE run_facts SET kept_in = NULL, why = 'the place was changed' WHERE repository = ?1",
                [repository],
            )
            .map(|_| ())
            .map_err(fault("choosing where a repository's runs are kept"))
            .map_err(WriteError::Database)
    }

    /// Every remembered run of a repository, by workspace, kind and command.
    pub fn run_facts(&self, repository: &str) -> Result<Vec<KeptRun>, WriteError> {
        let doing = fault("reading the runs a repository remembers");
        let mut asked = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM run_facts WHERE repository = ?1 ORDER BY workspace, kind, command"
            ))
            .map_err(doing)
            .map_err(WriteError::Database)?;
        let rows = asked
            .query_map([repository], kept)
            .map_err(fault("reading the runs a repository remembers"))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading a run a repository remembers"))
            .map_err(WriteError::Database)
    }

    /// Merge `runs` into what a repository remembers. **A dropped line is skipped**, and for the
    /// others the newer run wins, keeping the older run's port where the newer saw none. A row that
    /// changes stands as not yet written; one a run did not change keeps where it was written.
    pub fn merge_run_facts(&mut self, repository: &str, runs: &[KeptRun]) -> Result<(), WriteError> {
        for run in runs {
            let doing = "merging a run into what a repository remembers";
            let dropped = self
                .conn
                .query_row(
                    "SELECT 1 FROM run_facts_dropped WHERE repository = ?1 AND workspace = ?2 AND kind = ?3 AND command = ?4",
                    params![repository, run.workspace, run.kind, run.command],
                    |_| Ok(()),
                )
                .optional()
                .map_err(fault(doing))
                .map_err(WriteError::Database)?
                .is_some();
            if dropped {
                continue;
            }
            let held = self
                .conn
                .query_row(
                    &format!("SELECT {COLUMNS} FROM run_facts WHERE repository = ?1 AND workspace = ?2 AND kind = ?3 AND command = ?4"),
                    params![repository, run.workspace, run.kind, run.command],
                    kept,
                )
                .optional()
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
            let merged = match held {
                Some(old) if old.last_ran >= run.last_ran => continue,
                Some(old) => KeptRun { port: run.port.or(old.port), kept_in: None, why: None, ..run.clone() },
                None => KeptRun { kept_in: None, why: None, ..run.clone() },
            };
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO run_facts (repository, workspace, kind, command, port, succeeded, last_ran, source, kept_in, why) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL)",
                    params![
                        repository,
                        merged.workspace,
                        merged.kind,
                        merged.command,
                        merged.port.map(i64::from),
                        i64::from(merged.succeeded),
                        merged.last_ran as i64,
                        merged.source,
                    ],
                )
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
        }
        Ok(())
    }

    /// Say where one run stands: written to `kept_in`, or waiting for `why`.
    pub fn stand_run_fact(
        &mut self,
        repository: &str,
        run: (&str, &str, &str),
        kept_in: Option<&str>,
        why: Option<&str>,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE run_facts SET kept_in = ?5, why = ?6 \
                 WHERE repository = ?1 AND workspace = ?2 AND kind = ?3 AND command = ?4",
                params![repository, run.0, run.1, run.2, kept_in, why],
            )
            .map(|_| ())
            .map_err(fault("saying where a run is kept"))
            .map_err(WriteError::Database)
    }

    /// Drop a run: it goes onto the deny list and out of the facts, and is never merged again.
    pub fn drop_run_fact(
        &mut self,
        repository: &str,
        run: (&str, &str, &str),
        at: &str,
    ) -> Result<(), WriteError> {
        let doing = "dropping a run a repository remembers";
        self.conn
            .execute(
                "INSERT OR IGNORE INTO run_facts_dropped (repository, workspace, kind, command, dropped_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![repository, run.0, run.1, run.2, at],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        self.conn
            .execute(
                "DELETE FROM run_facts WHERE repository = ?1 AND workspace = ?2 AND kind = ?3 AND command = ?4",
                params![repository, run.0, run.1, run.2],
            )
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The runs a person dropped for a repository: workspace, kind, command.
    pub fn dropped_run_facts(&self, repository: &str) -> Result<Vec<(String, String, String)>, WriteError> {
        let mut asked = self
            .conn
            .prepare("SELECT workspace, kind, command FROM run_facts_dropped WHERE repository = ?1 ORDER BY dropped_at, rowid")
            .map_err(fault("reading the runs a repository dropped"))
            .map_err(WriteError::Database)?;
        let rows = asked
            .query_map([repository], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(fault("reading the runs a repository dropped"))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading a run a repository dropped"))
            .map_err(WriteError::Database)
    }
}
