//! What a Job's rules have seen, what they planned, and what the last look at the Job showed.
//!
//! **A plan is kept in one write with the pairs that made it**, so a crash between the two can
//! neither lose an act nor plan it twice: a rule that ran has its acts, and an act is there
//! because its rule ran. Text in, text out, as the rest of the store.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Where an act stands. **Spelled as the table keeps it.**
pub mod standing {
    /// Its course is to act, and it has not been done.
    pub const PLANNED: &str = "planned";
    /// Its course is to ask, and the owner has not said.
    pub const ASKED: &str = "asked";
    /// It failed past its retries and the rule says to ask: the owner has not said.
    pub const STUCK: &str = "stuck";
    pub const DONE: &str = "done";
    pub const SKIPPED: &str = "skipped";
    /// It failed past its retries and the rule says to tell the agent, which was.
    pub const TOLD: &str = "told";
    /// Planned and recorded, and nothing here does it.
    pub const NOT_CARRIED: &str = "not_carried";
}

/// One act a rule planned, as it was recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRuleAct {
    pub id: i64,
    pub session_id: String,
    pub rule_id: String,
    /// The occurrence that set it off, as `OccurrenceKey` writes it out.
    pub occurrence: String,
    /// Where it stands in the rule's Do, from the first.
    pub position: u32,
    /// `(lifecycle, key)` of the instance it is done to. Absent for an act done to none.
    pub on: Option<(String, String)>,
    /// `(lifecycle, key)` of the instance whose entering a state set it off, where one did.
    pub entered: Option<(String, String)>,
    /// `acts` or `asks`.
    pub course: String,
    pub state: String,
    /// How many tries were begun.
    pub attempts: u32,
    /// The one line the owner reads.
    pub text: String,
    /// What failed, or how it was settled.
    pub said: String,
    pub raised_at: String,
    pub settled_at: String,
}

/// An act to keep.
pub struct NewRuleAct<'a> {
    pub rule_id: &'a str,
    pub position: u32,
    pub on: Option<(&'a str, &'a str)>,
    pub entered: Option<(&'a str, &'a str)>,
    pub course: &'a str,
    pub state: &'a str,
    pub text: &'a str,
    pub said: &'a str,
}

/// What the last look at a Job showed, for the events found by comparing two looks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptSeen {
    /// The id of the step the Job was on. Absent when it was on none.
    pub step: Option<String>,
    pub pull_request_opened: bool,
    pub landed: bool,
    pub ended: bool,
    /// How many events were found, so each is told from the ones before it.
    pub events: u32,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

const COLUMNS: &str = "id, session_id, rule_id, occurrence, position, on_lifecycle, on_key, entered_lifecycle, \
                       entered_key, course, state, attempts, text, said, raised_at, settled_at";

fn pair(lifecycle: String, key: String) -> Option<(String, String)> {
    (!lifecycle.is_empty()).then_some((lifecycle, key))
}

fn kept(row: &Row<'_>) -> rusqlite::Result<KeptRuleAct> {
    Ok(KeptRuleAct {
        id: row.get(0)?,
        session_id: row.get(1)?,
        rule_id: row.get(2)?,
        occurrence: row.get(3)?,
        position: row.get(4)?,
        on: pair(row.get(5)?, row.get(6)?),
        entered: pair(row.get(7)?, row.get(8)?),
        course: row.get(9)?,
        state: row.get(10)?,
        attempts: row.get(11)?,
        text: row.get(12)?,
        said: row.get(13)?,
        raised_at: row.get(14)?,
        settled_at: row.get(15)?,
    })
}

impl Store {
    /// The rules of the Job that have seen `occurrence`.
    pub fn rules_that_ran(&self, session_id: &str, occurrence: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading which rules have seen an occurrence";
        self.conn
            .prepare("SELECT rule_id FROM rule_ran WHERE session_id = ?1 AND occurrence = ?2 ORDER BY rule_id")
            .map_err(failing(doing))?
            .query_map(params![session_id, occurrence], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failing(doing))
    }

    /// Keep that `ran` saw `occurrence`, and the acts they planned for it, in one write. The id of
    /// each act kept, in order. **An act already there is not kept again**, and its id is its own.
    pub fn keep_rule_plan(
        &mut self,
        session_id: &str,
        occurrence: &str,
        ran: &[&str],
        acts: &[NewRuleAct<'_>],
        at: &str,
    ) -> Result<Vec<i64>, WriteError> {
        let doing = "keeping what a rule planned";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        for rule in ran {
            tx.execute(
                "INSERT OR IGNORE INTO rule_ran (session_id, rule_id, occurrence) VALUES (?1, ?2, ?3)",
                params![session_id, rule, occurrence],
            )
            .map_err(failing(doing))?;
        }
        let mut ids = Vec::new();
        for act in acts {
            let (on_lifecycle, on_key) = act.on.unwrap_or_default();
            let (entered_lifecycle, entered_key) = act.entered.unwrap_or_default();
            tx.execute(
                "INSERT OR IGNORE INTO rule_acts
                   (session_id, rule_id, occurrence, position, on_lifecycle, on_key, entered_lifecycle, entered_key,
                    course, state, text, said, raised_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    session_id,
                    act.rule_id,
                    occurrence,
                    act.position,
                    on_lifecycle,
                    on_key,
                    entered_lifecycle,
                    entered_key,
                    act.course,
                    act.state,
                    act.text,
                    act.said,
                    at
                ],
            )
            .map_err(failing(doing))?;
            let id = tx
                .query_row(
                    "SELECT id FROM rule_acts WHERE session_id = ?1 AND rule_id = ?2 AND occurrence = ?3
                       AND position = ?4 AND on_lifecycle = ?5 AND on_key = ?6",
                    params![session_id, act.rule_id, occurrence, act.position, on_lifecycle, on_key],
                    |row| row.get(0),
                )
                .map_err(failing(doing))?;
            ids.push(id);
        }
        tx.commit().map_err(failing(doing))?;
        Ok(ids)
    }

    /// Every act planned for the Job, oldest first.
    pub fn rule_acts_of(&self, session_id: &str) -> Result<Vec<KeptRuleAct>, WriteError> {
        let doing = "reading the acts a job's rules planned";
        self.conn
            .prepare(&format!("SELECT {COLUMNS} FROM rule_acts WHERE session_id = ?1 ORDER BY id"))
            .map_err(failing(doing))?
            .query_map(params![session_id], kept)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))
    }

    pub fn rule_act(&self, id: i64) -> Result<Option<KeptRuleAct>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM rule_acts WHERE id = ?1"), params![id], kept)
            .optional()
            .map_err(failing("reading an act a rule planned"))
    }

    /// The acts of live Jobs that are planned to be done and have not been: the ones a Fleet that
    /// stopped between planning and doing still owes.
    pub fn rule_acts_owed(&self) -> Result<Vec<KeptRuleAct>, WriteError> {
        let doing = "reading the acts rules still owe";
        self.conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM rule_acts a JOIN sessions s ON s.id = a.session_id
                 WHERE a.state = 'planned' AND s.state = 'live' ORDER BY a.id"
            ))
            .map_err(failing(doing))?
            .query_map([], kept)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))
    }

    /// Begin a try, and say how many have been begun. **Counted before it is made**, so a try a
    /// crash cut short is one of the rule's retries and not a free one.
    pub fn begin_rule_attempt(&mut self, id: i64) -> Result<u32, WriteError> {
        let doing = "counting a try at an act a rule planned";
        self.conn
            .execute("UPDATE rule_acts SET attempts = attempts + 1 WHERE id = ?1", params![id])
            .map_err(failing(doing))?;
        self.conn
            .query_row("SELECT attempts FROM rule_acts WHERE id = ?1", params![id], |row| row.get(0))
            .map_err(failing(doing))
    }

    /// Move an act from one of `from` to `to`, saying why. False where it stood somewhere else,
    /// which is how two settlings of one act cannot both win.
    pub fn move_rule_act(&mut self, id: i64, from: &[&str], to: &str, said: &str, at: &str) -> Result<bool, WriteError> {
        let doing = "settling an act a rule planned";
        let settled_at = if matches!(to, standing::DONE | standing::SKIPPED | standing::TOLD | standing::NOT_CARRIED) { at } else { "" };
        for held in from {
            let changed = self
                .conn
                .execute(
                    "UPDATE rule_acts SET state = ?2, said = ?3, settled_at = ?4 WHERE id = ?1 AND state = ?5",
                    params![id, to, said, settled_at, held],
                )
                .map_err(failing(doing))?;
            if changed > 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Start an act over: tries counted from none, and its words as they were. For the owner's
    /// Try again on one that failed past its retries.
    pub fn restart_rule_act(&mut self, id: i64, to: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE rule_acts SET state = ?2, attempts = 0, said = '', settled_at = '' WHERE id = ?1 AND state = 'stuck'",
                params![id, to],
            )
            .map(|changed| changed > 0)
            .map_err(failing("starting an act a rule planned over"))
    }

    /// What the last look at the Job showed. `None` before the first.
    pub fn job_seen(&self, session_id: &str) -> Result<Option<KeptSeen>, WriteError> {
        self.conn
            .query_row(
                "SELECT step, pull_request_opened, landed, ended, events FROM rule_job_seen WHERE session_id = ?1",
                params![session_id],
                |row| {
                    let step: String = row.get(0)?;
                    Ok(KeptSeen {
                        step: Some(step).filter(|step| !step.is_empty()),
                        pull_request_opened: row.get(1)?,
                        landed: row.get(2)?,
                        ended: row.get(3)?,
                        events: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(failing("reading what the last look at a job showed"))
    }

    pub fn keep_job_seen(&mut self, session_id: &str, seen: &KeptSeen) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO rule_job_seen (session_id, step, pull_request_opened, landed, ended, events)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (session_id) DO UPDATE SET step = ?2, pull_request_opened = ?3, landed = ?4, ended = ?5, events = ?6",
                params![session_id, seen.step.as_deref().unwrap_or_default(), seen.pull_request_opened, seen.landed, seen.ended, seen.events],
            )
            .map(|_| ())
            .map_err(failing("keeping what the last look at a job showed"))
    }
}
