//! How a taken-over Job's pilot ended. `docs/concepts/pilot.md`.
//!
//! **One row a Job.** The Job's own log and transitions keep every earlier
//! pilot; this row is the current one and the last one's exit, which is what a
//! surface reads. Nothing writes a pilot any more, so only a Job taken over
//! before that has one; the table and its Narrative and Session columns stay
//! and are not read.

use core_model::{JobId, PilotReason, Timestamp};

use crate::error::{fault, DatabaseFault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// How a pilot ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotExit {
    /// The step's gates ran against the worktree, or a fresh Drone took it.
    Submitted,
    /// A person said it is done. **Never verified**, and rendered apart.
    Attested,
    /// The work landed outside the Job.
    Superseded,
}

impl PilotExit {
    pub fn as_wire(self) -> &'static str {
        match self {
            PilotExit::Submitted => "submitted",
            PilotExit::Attested => "attested",
            PilotExit::Superseded => "superseded",
        }
    }

    fn from_column(text: &str) -> Option<PilotExit> {
        [
            PilotExit::Submitted,
            PilotExit::Attested,
            PilotExit::Superseded,
        ]
        .into_iter()
        .find(|exit| exit.as_wire() == text)
    }
}

/// One Job's pilot, current or last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPilot {
    pub job: JobId,
    pub reason: PilotReason,
    /// Absent where the row was only ever a mark.
    pub piloted_at: Option<Timestamp>,
    pub exit: Option<PilotExit>,
    pub ended_at: Option<Timestamp>,
    /// What the person said when they attested or superseded.
    pub note: Option<String>,
}

impl KeptPilot {
    /// The pilot is under way: the Job moved and nothing has ended it.
    pub fn standing(&self) -> bool {
        self.piloted_at.is_some() && self.ended_at.is_none()
    }
}

const COLUMNS: &str = "job_id, reason, piloted_at, exit_how, ended_at, note";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptPilot> {
    let unknown = |at: usize, what: String| {
        rusqlite::Error::InvalidColumnType(at, what, rusqlite::types::Type::Text)
    };
    let reason: String = row.get(1)?;
    let exit: Option<String> = row.get(3)?;
    Ok(KeptPilot {
        job: JobId::carried(core_model::Ulid::carried(row.get::<_, String>(0)?)),
        reason: PilotReason::from_wire(&reason)
            .ok_or_else(|| unknown(1, format!("a reason {reason} this build does not know")))?,
        piloted_at: row
            .get::<_, Option<String>>(2)?
            .map(Timestamp::from_rfc3339),
        exit: match exit {
            None => None,
            Some(text) => {
                Some(PilotExit::from_column(&text).ok_or_else(|| {
                    unknown(3, format!("an exit {text} this build does not know"))
                })?)
            }
        },
        ended_at: row
            .get::<_, Option<String>>(4)?
            .map(Timestamp::from_rfc3339),
        note: row.get(5)?,
    })
}

impl Store {
    /// Take an exit back, for a move that failed after the pilot was stamped
    /// ended. **Returns whether one was ended.**
    pub fn pilot_reopened(&mut self, job: &JobId) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_pilots SET exit_how = NULL, ended_at = NULL, note = NULL \
                 WHERE job_id = ?1 AND ended_at IS NOT NULL",
                (job.as_str(),),
            )
            .map_err(fault("reopening a pilot"))?;
        Ok(changed == 1)
    }

    /// End the pilot under way. **Returns whether one was.**
    pub fn pilot_ended(
        &mut self,
        job: &JobId,
        exit: PilotExit,
        at: &Timestamp,
        note: Option<&str>,
    ) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_pilots SET exit_how = ?2, ended_at = ?3, note = ?4 \
                 WHERE job_id = ?1 AND piloted_at IS NOT NULL AND ended_at IS NULL",
                (job.as_str(), exit.as_wire(), at.as_str(), note),
            )
            .map_err(fault("ending a pilot"))?;
        Ok(changed == 1)
    }

    /// The Job's current or last pilot, and its mark where nothing moved yet.
    pub fn pilot_of(&self, job: &JobId) -> Result<Option<KeptPilot>, LoadJobError> {
        let doing = "reading a pilot";
        let mut asking = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM job_pilots WHERE job_id = ?1"
            ))
            .map_err(fault(doing))
            .map_err(database)?;
        let mut rows = asking
            .query_map((job.as_str(),), read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(database)?;
        Ok(rows.pop())
    }
}

fn database(cause: DatabaseFault) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(cause))
}
