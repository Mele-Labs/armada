//! Which Job is working on a repository's red main, and which one fixed it.
//! `docs/concepts/fleet.md`, *When main goes red*.
//!
//! **One row a Job per red**, keyed by the red's `red_at`, which a red keeps
//! until a green. The key is what makes taking a red once-only: a second take
//! of the same red by the same Job is refused by the insert.
//!
//! **The Job is plain text and no foreign key**, as in `main_ci`: a Job
//! `armada clean` forgets leaves its row, and a reader that cannot load it
//! passes over it.

use core_model::{JobId, Timestamp, Ulid};

use crate::error::{fault, DatabaseFault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// Version 115, additive: who took each red main, for the hub's `fixing` and a
/// Job's own mark.
pub(crate) const V115: &str = r#"
CREATE TABLE main_ci_fixes (
    repository   TEXT NOT NULL,
    red_at       TEXT NOT NULL,
    job_id       TEXT NOT NULL,
    how          TEXT NOT NULL CHECK (how IN ('took', 'dispatched', 'sent_back')),
    check_name   TEXT NOT NULL,
    test         TEXT,
    merge_number INTEGER CHECK (merge_number IS NULL OR merge_number > 0),
    taken_at     TEXT NOT NULL,
    ended_at     TEXT,
    fixed_in     INTEGER CHECK (fixed_in IS NULL OR fixed_in > 0),
    PRIMARY KEY (repository, red_at, job_id)
) STRICT;

CREATE INDEX main_ci_fixes_by_job ON main_ci_fixes (job_id, taken_at);
"#;

/// How a Job came to take a red.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakenHow {
    /// The Job whose pull request turned main red, sent back by Fleet.
    Took,
    /// A new Job a person dispatched for it.
    Dispatched,
    /// An earlier Job a person sent the work back to.
    SentBack,
}

impl TakenHow {
    fn as_str(self) -> &'static str {
        match self {
            TakenHow::Took => "took",
            TakenHow::Dispatched => "dispatched",
            TakenHow::SentBack => "sent_back",
        }
    }

    fn from_column(text: &str) -> Option<TakenHow> {
        [TakenHow::Took, TakenHow::Dispatched, TakenHow::SentBack]
            .into_iter()
            .find(|how| how.as_str() == text)
    }
}

/// One Job's part in one red.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainFix {
    pub repository: String,
    /// The red it took, by when main first read red.
    pub red_at: Timestamp,
    pub job: JobId,
    pub how: TakenHow,
    /// The Check, or the CI job's own name where none maps.
    pub check: String,
    pub test: Option<String>,
    /// The pull request that turned main red.
    pub merge: Option<u64>,
    pub taken_at: Timestamp,
    /// When main went green, or the red otherwise ended. `None` while it is on.
    pub ended_at: Option<Timestamp>,
    /// The pull request of this Job that put main green. Only the Job that did.
    pub fixed_in: Option<u64>,
}

const COLUMNS: &str = "repository, red_at, job_id, how, check_name, test, merge_number, \
                       taken_at, ended_at, fixed_in";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<MainFix> {
    let how: String = row.get(3)?;
    Ok(MainFix {
        repository: row.get(0)?,
        red_at: Timestamp::from_rfc3339(row.get::<_, String>(1)?),
        job: JobId::carried(Ulid::carried(row.get::<_, String>(2)?)),
        how: TakenHow::from_column(&how).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(
                3,
                format!("a way {how} this build does not know"),
                rusqlite::types::Type::Text,
            )
        })?,
        check: row.get(4)?,
        test: row.get(5)?,
        merge: row.get::<_, Option<i64>>(6)?.map(|number| number as u64),
        taken_at: Timestamp::from_rfc3339(row.get::<_, String>(7)?),
        ended_at: row
            .get::<_, Option<String>>(8)?
            .map(Timestamp::from_rfc3339),
        fixed_in: row.get::<_, Option<i64>>(9)?.map(|number| number as u64),
    })
}

impl Store {
    /// Keep a Job's take of a red. **`false` where this Job already took this
    /// red**, and nothing is written.
    pub fn keep_main_fix(&mut self, fix: &MainFix) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "INSERT INTO main_ci_fixes (repository, red_at, job_id, how, check_name, test, \
                 merge_number, taken_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (repository, red_at, job_id) DO NOTHING",
                (
                    &fix.repository,
                    fix.red_at.as_str(),
                    fix.job.as_str(),
                    fix.how.as_str(),
                    &fix.check,
                    &fix.test,
                    fix.merge.map(|number| number as i64),
                    fix.taken_at.as_str(),
                ),
            )
            .map_err(fault("keeping a take of main's red"))?;
        Ok(kept == 1)
    }

    /// Every take of one red, newest first.
    pub fn main_fixes_of(
        &self,
        repository: &str,
        red_at: &Timestamp,
    ) -> Result<Vec<MainFix>, LoadJobError> {
        self.fixes_where(
            "WHERE repository = ?1 AND red_at = ?2 ORDER BY taken_at DESC, job_id DESC",
            &[&repository, &red_at.as_str()],
        )
    }

    /// The newest take this Job has made of any red.
    pub fn main_fix_of_job(&self, job: &JobId) -> Result<Option<MainFix>, LoadJobError> {
        Ok(self
            .fixes_where(
                "WHERE job_id = ?1 ORDER BY taken_at DESC LIMIT 1",
                &[&job.as_str()],
            )?
            .into_iter()
            .next())
    }

    /// Whether this Job has a take still on, of any red.
    pub fn is_fixing_main(&self, job: &JobId) -> Result<bool, LoadJobError> {
        Ok(!self
            .fixes_where(
                "WHERE job_id = ?1 AND ended_at IS NULL LIMIT 1",
                &[&job.as_str()],
            )?
            .is_empty())
    }

    /// End every take still on in a repository, because main went green.
    /// `fixed` is the Job whose pull request put it there and that pull
    /// request's number, where there was one: only its row says it fixed main.
    pub fn end_main_fixes(
        &mut self,
        repository: &str,
        at: &Timestamp,
        fixed: Option<(&JobId, u64)>,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("ending takes of main's red"))?;
        if let Some((job, number)) = fixed {
            tx.execute(
                "UPDATE main_ci_fixes SET fixed_in = ?3 \
                 WHERE repository = ?1 AND job_id = ?2 AND ended_at IS NULL",
                (repository, job.as_str(), number as i64),
            )
            .map_err(fault("ending takes of main's red"))?;
        }
        tx.execute(
            "UPDATE main_ci_fixes SET ended_at = ?2 WHERE repository = ?1 AND ended_at IS NULL",
            (repository, at.as_str()),
        )
        .map_err(fault("ending takes of main's red"))?;
        tx.commit().map_err(fault("ending takes of main's red"))?;
        Ok(())
    }

    fn fixes_where(
        &self,
        clause: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<MainFix>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM main_ci_fixes {clause}"))
            .map_err(fault("reading takes of main's red"))
            .map_err(database)?;
        let rows = asking
            .query_map(params, read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("reading takes of main's red"))
            .map_err(database)?;
        Ok(rows)
    }
}

fn database(cause: DatabaseFault) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(cause))
}
