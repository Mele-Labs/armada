//! Who took a Job over, and how the pilot ended. `docs/concepts/pilot.md`.
//!
//! **One row a Job, replaced when it is taken over again.** The Job's own log
//! and transitions keep every earlier pilot; this row is the current one and
//! the last one's exit, which is what a surface reads.
//!
//! **The mark is a row with no `piloted_at`**, and it names the run of Fleet
//! that set it. A Fleet that died between marking and moving leaves a mark no
//! later run honours, so a Drone's unbidden pull cannot be let through by a
//! mark nobody is waiting on.

use core_model::{JobId, PilotReason, Timestamp};

use crate::error::{fault, DatabaseFault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// What a Drone said it was stuck on, as `escape_hatch` carried it.
///
/// **Not Evidence**: Evidence is proof tied to an advance gate and this states
/// that no proof is coming. Nothing here is read as a claim about the work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Narrative {
    /// What the step was meant to produce.
    pub trying_to: String,
    /// The specific thing preventing it.
    pub blocked_by: String,
    /// What was attempted, and what each attempt produced.
    pub tried: String,
}

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
    /// The Session the person is piloting from, where they said.
    pub session: Option<String>,
    pub marked_run: String,
    pub marked_at: Timestamp,
    /// Absent while only the mark stands.
    pub piloted_at: Option<Timestamp>,
    pub narrative: Option<Narrative>,
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

const COLUMNS: &str = "job_id, reason, session_id, marked_run, marked_at, piloted_at, trying_to, \
                       blocked_by, tried, exit_how, ended_at, note";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptPilot> {
    let unknown = |at: usize, what: String| {
        rusqlite::Error::InvalidColumnType(at, what, rusqlite::types::Type::Text)
    };
    let reason: String = row.get(1)?;
    let exit: Option<String> = row.get(9)?;
    let said: [Option<String>; 3] = [row.get(6)?, row.get(7)?, row.get(8)?];
    Ok(KeptPilot {
        job: JobId::carried(core_model::Ulid::carried(row.get::<_, String>(0)?)),
        reason: PilotReason::from_wire(&reason)
            .ok_or_else(|| unknown(1, format!("a reason {reason} this build does not know")))?,
        session: row.get(2)?,
        marked_run: row.get(3)?,
        marked_at: Timestamp::from_rfc3339(row.get::<_, String>(4)?),
        piloted_at: row.get::<_, Option<String>>(5)?.map(Timestamp::from_rfc3339),
        narrative: match said {
            [None, None, None] => None,
            [trying_to, blocked_by, tried] => Some(Narrative {
                trying_to: trying_to.unwrap_or_default(),
                blocked_by: blocked_by.unwrap_or_default(),
                tried: tried.unwrap_or_default(),
            }),
        },
        exit: match exit {
            None => None,
            Some(text) => Some(
                PilotExit::from_column(&text)
                    .ok_or_else(|| unknown(9, format!("an exit {text} this build does not know")))?,
            ),
        },
        ended_at: row
            .get::<_, Option<String>>(10)?
            .map(Timestamp::from_rfc3339),
        note: row.get(11)?,
    })
}

impl Store {
    /// Mark a Job for the handoff, replacing whatever pilot it last had.
    pub fn mark_for_pilot(
        &mut self,
        job: &JobId,
        reason: PilotReason,
        session: Option<&str>,
        run: &str,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO job_pilots (job_id, reason, session_id, marked_run, \
                 marked_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                (job.as_str(), reason.as_wire(), session, run, at.as_str()),
            )
            .map_err(fault("marking a job for a pilot"))?;
        Ok(())
    }

    /// Forget the Job's pilot at whatever stage it was. **For a take over that
    /// failed**, so a mark outlives no pull.
    pub fn drop_pilot(&mut self, job: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM job_pilots WHERE job_id = ?1", (job.as_str(),))
            .map_err(fault("dropping a pilot"))?;
        Ok(())
    }

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

    /// The Job moved to `piloted`: stamp it, and keep the narrative the Drone
    /// brought where it brought one. **Returns whether a mark was there to
    /// stamp.**
    pub fn pilot_began(
        &mut self,
        job: &JobId,
        at: &Timestamp,
        narrative: Option<&Narrative>,
    ) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_pilots SET piloted_at = ?2, trying_to = ?3, blocked_by = ?4, \
                 tried = ?5 WHERE job_id = ?1 AND piloted_at IS NULL",
                (
                    job.as_str(),
                    at.as_str(),
                    narrative.map(|said| said.trying_to.as_str()),
                    narrative.map(|said| said.blocked_by.as_str()),
                    narrative.map(|said| said.tried.as_str()),
                ),
            )
            .map_err(fault("stamping a pilot"))?;
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
