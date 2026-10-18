//! A repository's Autopilot and what it picked. #2092, `docs/concepts/autopilot.md`.
//!
//! **Words, not enums.** The states are Fleet's vocabulary (`fleet::autopilot`); this keeps what
//! it was handed, as `sleep_night` does, and Fleet refuses a word it does not know when it reads.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One repository's Autopilot. `run_id` names this start, and its picks carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAutopilot {
    pub manifest_id: String,
    pub run_id: String,
    pub instruction: String,
    pub at_once: u32,
    pub until: Option<String>,
    pub spend_cap_micros: Option<u64>,
    pub session_cap: Option<u32>,
    /// `running`, `paused` or `stopped`.
    pub state: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    /// `until`, `spend` or `sessions`: the cap that stopped it. Absent where a person did, or nothing has.
    pub capped_by: Option<String>,
}

/// One pick. `started_session` is the Session it started, where it started one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPick {
    pub pick_id: String,
    pub run_id: String,
    pub manifest_id: String,
    /// The tracker's own name for the issue: `#12`, `ABC-12`.
    pub issue_reference: String,
    pub issue_title: String,
    pub issue_url: String,
    pub workflow_id: Option<String>,
    pub started_session: Option<String>,
    /// `started`, `set_aside`, `withdrawn` or `dismissed`.
    pub state: String,
    pub reason: String,
    pub at: String,
}

const AUTOPILOT: &str =
    "manifest_id, run_id, instruction, at_once, until, spend_cap_micros, job_cap, \
                         state, started_at, ended_at, capped_by";
const PICK: &str =
    "pick_id, run_id, manifest_id, issue_number, issue_title, issue_url, workflow_id, \
                    started_session, state, reason, at, issue_reference";

fn autopilot_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptAutopilot> {
    Ok(KeptAutopilot {
        manifest_id: row.get(0)?,
        run_id: row.get(1)?,
        instruction: row.get(2)?,
        at_once: row.get(3)?,
        until: row.get(4)?,
        spend_cap_micros: row
            .get::<_, Option<i64>>(5)?
            .map(|micros| micros.max(0) as u64),
        session_cap: row.get(6)?,
        state: row.get(7)?,
        started_at: row.get(8)?,
        ended_at: row.get(9)?,
        capped_by: row.get(10)?,
    })
}

fn pick_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptPick> {
    Ok(KeptPick {
        pick_id: row.get(0)?,
        run_id: row.get(1)?,
        manifest_id: row.get(2)?,
        // A pick from before the column has no reference and reads as its number.
        issue_reference: match row.get::<_, String>(11)? {
            kept if !kept.is_empty() => kept,
            _ => format!("#{}", row.get::<_, i64>(3)?.max(0)),
        },
        issue_title: row.get(4)?,
        issue_url: row.get(5)?,
        workflow_id: row.get(6)?,
        started_session: row.get(7)?,
        state: row.get(8)?,
        reason: row.get(9)?,
        at: row.get(10)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Every repository's Autopilot, stopped ones included, by repository.
    pub fn autopilots(&self) -> Result<Vec<KeptAutopilot>, WriteError> {
        let doing = "reading every Autopilot";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {AUTOPILOT} FROM autopilots ORDER BY manifest_id"
            ))
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([], autopilot_of)
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    pub fn autopilot(&self, manifest_id: &str) -> Result<Option<KeptAutopilot>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {AUTOPILOT} FROM autopilots WHERE manifest_id = ?1"),
                [manifest_id],
                autopilot_of,
            )
            .optional()
            .map_err(failed("reading a repository's Autopilot"))
    }

    /// Keep a new start, replacing whatever the repository held. **Fleet refuses one that is not
    /// stopped before this is reached**; the store does not know what stopped means.
    pub fn start_autopilot(&mut self, kept: &KeptAutopilot) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO autopilots ({AUTOPILOT})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
                ),
                params![
                    kept.manifest_id,
                    kept.run_id,
                    kept.instruction,
                    kept.at_once,
                    kept.until,
                    kept.spend_cap_micros.map(|micros| micros as i64),
                    kept.session_cap,
                    kept.state,
                    kept.started_at,
                    kept.ended_at,
                    kept.capped_by,
                ],
            )
            .map(|_| ())
            .map_err(failed("starting an Autopilot"))
    }

    /// Move one run to `state`. **False where the repository holds no such run**, so an act racing
    /// a new start does not move the new one.
    pub fn move_autopilot(
        &mut self,
        run_id: &str,
        state: &str,
        ended_at: Option<&str>,
        capped_by: Option<&str>,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE autopilots SET state = ?2, ended_at = ?3, capped_by = ?4 WHERE run_id = ?1",
                params![run_id, state, ended_at, capped_by],
            )
            .map(|changed| changed > 0)
            .map_err(failed("moving an Autopilot"))
    }

    /// Change one run's instruction, at once and caps, and nothing else: its state, `capped_by`
    /// and picks stay. **False where the repository holds no such run**, as
    /// [`move_autopilot`](Store::move_autopilot) answers.
    pub fn update_autopilot(&mut self, kept: &KeptAutopilot) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE autopilots SET instruction = ?2, at_once = ?3, until = ?4,
                     spend_cap_micros = ?5, job_cap = ?6
                 WHERE run_id = ?1",
                params![
                    kept.run_id,
                    kept.instruction,
                    kept.at_once,
                    kept.until,
                    kept.spend_cap_micros.map(|micros| micros as i64),
                    kept.session_cap,
                ],
            )
            .map(|changed| changed > 0)
            .map_err(failed("changing an Autopilot's settings"))
    }

    /// One run's picks, oldest first: a pick id is a ULID, minted in order.
    pub fn autopilot_picks(&self, run_id: &str) -> Result<Vec<KeptPick>, WriteError> {
        let doing = "reading an Autopilot's picks";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {PICK} FROM autopilot_picks WHERE run_id = ?1 ORDER BY pick_id"
            ))
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([run_id], pick_of)
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Every pick, of any run, that started a Session, oldest first. A run ends at Stop and a
    /// later Start opens another, while the Session the first started may still be live.
    pub fn autopilot_picks_that_started_a_session(&self) -> Result<Vec<KeptPick>, WriteError> {
        let doing = "reading the picks that started a Job";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {PICK} FROM autopilot_picks WHERE started_session IS NOT NULL
                 ORDER BY pick_id"
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map([], pick_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    pub fn add_autopilot_pick(&mut self, pick: &KeptPick) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO autopilot_picks ({PICK})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
                ),
                params![
                    pick.pick_id,
                    pick.run_id,
                    pick.manifest_id,
                    pick.issue_reference.trim_start_matches('#').parse::<i64>().unwrap_or(0),
                    pick.issue_title,
                    pick.issue_url,
                    pick.workflow_id,
                    pick.started_session,
                    pick.state,
                    pick.reason,
                    pick.at,
                    pick.issue_reference,
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping an Autopilot's pick"))
    }

    /// Move one pick from `from` to `to`, naming the Session it started where it started one. **False
    /// where the pick is not at `from`**, which is how two presses on one pick start one Session.
    pub fn move_autopilot_pick(
        &mut self,
        pick_id: &str,
        from: &str,
        to: &str,
        started_session: Option<&str>,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE autopilot_picks SET state = ?3, started_session = COALESCE(?4, started_session)
                 WHERE pick_id = ?1 AND state = ?2",
                params![pick_id, from, to, started_session],
            )
            .map(|changed| changed > 0)
            .map_err(failed("moving an Autopilot's pick"))
    }
}
