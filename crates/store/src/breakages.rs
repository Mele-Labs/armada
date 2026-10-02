//! A test broken on main, and the Job drafted to fix it. #999.
//!
//! One row per repository, Check and test, owned by the fix: forgetting the fix
//! removes its claims, and Fleet gives them back when the fix settles.

use core_model::{
    Breakage, BreakageClaim, FixWaiter, JobId, LandedHold, ManifestId, RepoPath, Timestamp, Ulid,
};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 68 — which Job is fixing which test broken on main.
///
/// **`reported_by` is not a reference.** The reporting Job can be forgotten
/// while the fix is still worked, and a second key to `jobs` would make that
/// forget fail at commit. `job_id` is the fix, which is what `forget_job` keys on.
pub(crate) const V68: &str = r#"
CREATE TABLE job_breakage_claims (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    failure     TEXT NOT NULL,
    reported_by TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (repository, check_name, test)
) STRICT;
"#;

/// Version 69 — which Jobs failed on a claimed test, pointed at its fix. #1001.
///
/// **`fix` is not a reference**, for `reported_by`'s reason one table up.
/// `job_id` is the waiting Job, so forgetting it removes its pointers.
pub(crate) const V69: &str = r#"
CREATE TABLE job_fix_waiters (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    fix         TEXT NOT NULL,
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (job_id, repository, check_name, test)
) STRICT;
"#;

/// Version 94 — the test's files on a claim, and what a landed fix still
/// holds off a Job until its copy takes the fix. #1673.
///
/// **Two tables beside the claim rather than a column on it**, for
/// `crate::plan`'s reason: a list is rows. `job_id` is the fix on the first
/// and the held Job on the second, so each goes with the Job that owns it.
pub(crate) const V94: &str = r#"
CREATE TABLE job_breakage_claim_files (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    path        TEXT NOT NULL,
    PRIMARY KEY (repository, check_name, test, path)
) STRICT;

CREATE TABLE job_landed_holds (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    fix     TEXT NOT NULL,
    test    TEXT NOT NULL,
    path    TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, fix, test, path)
) STRICT;
"#;

impl Store {
    /// Claim a breakage for its fix. `false` where the same test in the same
    /// repository is already claimed, so a second report drafts nothing.
    ///
    /// **One statement decides**, so two claims cannot both land; the files
    /// go in beside it, in the same transaction, only where it did.
    pub fn claim_breakage(
        &mut self,
        claim: &BreakageClaim,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a claim"))
            .map_err(WriteError::Database)?;
        let claimed = tx
            .execute(
                "INSERT INTO job_breakage_claims
                   (job_id, repository, check_name, test, failure, reported_by, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (repository, check_name, test) DO NOTHING",
                rusqlite::params![
                    claim.fix.as_str(),
                    claim.repository.as_str(),
                    claim.breakage.check,
                    claim.breakage.test,
                    claim.breakage.failure,
                    claim.reported_by.as_str(),
                    at.as_str(),
                ],
            )
            .map(|claimed| claimed == 1)
            .map_err(fault("claiming a breakage"))
            .map_err(WriteError::Database)?;
        if claimed {
            for path in &claim.files {
                tx.execute(
                    "INSERT OR IGNORE INTO job_breakage_claim_files
                       (job_id, repository, check_name, test, path)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        claim.fix.as_str(),
                        claim.repository.as_str(),
                        claim.breakage.check,
                        claim.breakage.test,
                        path.as_str(),
                    ],
                )
                .map_err(fault("naming a claimed test's files"))
                .map_err(WriteError::Database)?;
            }
        }
        tx.commit()
            .map_err(fault("committing a claim"))
            .map_err(WriteError::Database)?;
        Ok(claimed)
    }

    /// The claim on one test in one repository, where there is one.
    pub fn breakage_claimed(
        &self,
        repository: &ManifestId,
        check: &str,
        test: &str,
    ) -> Result<Option<BreakageClaim>, LoadJobError> {
        Ok(self
            .breakage_claims(
                "WHERE repository = ?1 AND check_name = ?2 AND test = ?3",
                rusqlite::params![repository.as_str(), check, test],
            )?
            .into_iter()
            .next())
    }

    /// Every breakage one fix Job claims, oldest first.
    pub fn breakages_claimed_by(&self, fix: &JobId) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE job_id = ?1 ORDER BY at, check_name, test",
            rusqlite::params![fix.as_str()],
        )
    }

    /// Every claimed fix for a test one Job's Drone reported, oldest first.
    pub fn breakages_reported_by(
        &self,
        reporter: &JobId,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE reported_by = ?1 ORDER BY at, check_name, test",
            rusqlite::params![reporter.as_str()],
        )
    }

    /// Every standing claim in one repository, oldest first.
    pub fn breakages_claimed_in(
        &self,
        repository: &ManifestId,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE repository = ?1 ORDER BY at, check_name, test",
            rusqlite::params![repository.as_str()],
        )
    }

    /// Point a Job at the fix claiming a test it failed on. `false` where it
    /// already was, so it is told once.
    pub fn point_at_fix(&mut self, waiter: &FixWaiter, at: &Timestamp) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_fix_waiters (job_id, fix, repository, check_name, test, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (job_id, repository, check_name, test) DO NOTHING",
                rusqlite::params![
                    waiter.waiting.as_str(),
                    waiter.fix.as_str(),
                    waiter.repository.as_str(),
                    waiter.check,
                    waiter.test,
                    at.as_str(),
                ],
            )
            .map(|pointed| pointed == 1)
            .map_err(fault("pointing a Job at a fix"))
            .map_err(WriteError::Database)
    }

    /// Every Job pointed at one fix, oldest first.
    pub fn waiting_on_fix(&self, fix: &JobId) -> Result<Vec<FixWaiter>, LoadJobError> {
        self.fix_waiters(
            "WHERE fix = ?1 ORDER BY at, job_id",
            rusqlite::params![fix.as_str()],
        )
    }

    /// Every fix one Job is pointed at, oldest first.
    pub fn fixes_waited_on_by(&self, job: &JobId) -> Result<Vec<FixWaiter>, LoadJobError> {
        self.fix_waiters(
            "WHERE job_id = ?1 ORDER BY at, check_name, test",
            rusqlite::params![job.as_str()],
        )
    }

    /// Drop every pointer at a fix, once the Jobs waiting on it have been told.
    pub fn release_waiters(&mut self, fix: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_fix_waiters WHERE fix = ?1",
                (fix.as_str(),),
            )
            .map(|_| ())
            .map_err(fault("releasing the Jobs waiting on a fix"))
            .map_err(WriteError::Database)
    }

    fn fix_waiters(
        &self,
        filter: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<FixWaiter>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading fix waiters")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT job_id, fix, repository, check_name, test FROM job_fix_waiters {filter}"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params, |row| {
                Ok(FixWaiter {
                    waiting: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    fix: JobId::carried(Ulid::carried(row.get::<_, String>(1)?)),
                    repository: ManifestId::carried(Ulid::carried(row.get::<_, String>(2)?)),
                    check: row.get(3)?,
                    test: row.get(4)?,
                })
            })
            .map_err(unreadable)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(unreadable)
    }

    /// Give back every claim a fix Job holds, once it has ended.
    pub fn release_breakages(&mut self, fix: &JobId) -> Result<(), WriteError> {
        for table in ["job_breakage_claim_files", "job_breakage_claims"] {
            self.conn
                .execute(
                    &format!("DELETE FROM {table} WHERE job_id = ?1"),
                    (fix.as_str(),),
                )
                .map_err(fault("releasing a fix's breakages"))
                .map_err(WriteError::Database)?;
        }
        Ok(())
    }

    /// Hold these paths off a Job until its copy takes the fix that landed.
    pub fn hold_until_caught_up(
        &mut self,
        hold: &LandedHold,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        for path in &hold.paths {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO job_landed_holds (job_id, fix, test, path, at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        hold.held.as_str(),
                        hold.fix.as_str(),
                        hold.test,
                        path.as_str(),
                        at.as_str(),
                    ],
                )
                .map_err(fault("holding a landed fix's files"))
                .map_err(WriteError::Database)?;
        }
        Ok(())
    }

    /// What landed fixes still hold off one Job, one entry per fix and test.
    pub fn landed_holds_on(&self, held: &JobId) -> Result<Vec<LandedHold>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading landed holds")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT fix, test, path FROM job_landed_holds
                 WHERE job_id = ?1 ORDER BY at, fix, test, path",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map((held.as_str(),), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(unreadable)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)?;
        let mut holds: Vec<LandedHold> = Vec::new();
        for (fix, test, path) in rows {
            let fix = JobId::carried(Ulid::carried(fix));
            match holds
                .iter_mut()
                .find(|hold| hold.fix == fix && hold.test == test)
            {
                Some(hold) => hold.paths.push(RepoPath::new(&path)),
                None => holds.push(LandedHold {
                    held: held.clone(),
                    fix,
                    test,
                    paths: vec![RepoPath::new(&path)],
                }),
            }
        }
        Ok(holds)
    }

    /// Give back everything landed fixes held off one Job, once its copy has
    /// taken them.
    pub fn release_landed_holds(&mut self, held: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_landed_holds WHERE job_id = ?1",
                (held.as_str(),),
            )
            .map(|_| ())
            .map_err(fault("releasing landed holds"))
            .map_err(WriteError::Database)
    }

    /// The files named on one claim, in the order they were stored.
    fn claim_files(
        &self,
        repository: &ManifestId,
        check: &str,
        test: &str,
    ) -> Result<Vec<RepoPath>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading a claim's files")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT path FROM job_breakage_claim_files
                 WHERE repository = ?1 AND check_name = ?2 AND test = ?3 ORDER BY rowid",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![repository.as_str(), check, test], |row| {
                row.get::<_, String>(0)
            })
            .map_err(unreadable)?;
        rows.map(|path| path.map(|path| RepoPath::new(&path)))
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)
    }

    fn breakage_claims(
        &self,
        filter: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading breakage claims")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT job_id, repository, check_name, test, failure, reported_by
                 FROM job_breakage_claims {filter}"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params, |row| {
                Ok(BreakageClaim {
                    fix: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    repository: ManifestId::carried(Ulid::carried(row.get::<_, String>(1)?)),
                    breakage: Breakage {
                        check: row.get(2)?,
                        test: row.get(3)?,
                        failure: row.get(4)?,
                    },
                    reported_by: JobId::carried(Ulid::carried(row.get::<_, String>(5)?)),
                    files: Vec::new(),
                })
            })
            .map_err(unreadable)?;
        let mut claims = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)?;
        drop(statement);
        for claim in &mut claims {
            claim.files = self.claim_files(
                &claim.repository,
                &claim.breakage.check,
                &claim.breakage.test,
            )?;
        }
        Ok(claims)
    }
}
