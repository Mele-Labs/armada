//! What a Session's rail has done and what its judges said, kept. `docs/concepts/session.md`,
//! *The rail*, *The judges*.
//!
//! **Strings and numbers, not the wire's types**: this crate does not know `ipc`. The step is an index
//! into the Session's workflow; the lists of steps are kept as comma-separated indexes; a concern's
//! cites are one per line, `region`, `path`, `from`, `to` separated by tabs.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What the rail has done for a Session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptRail {
    /// The step the Session last stood on, as the timeline was told.
    pub step: Option<u32>,
    /// Steps Fleet has nudged the agent about.
    pub nudged: Vec<u32>,
    /// Steps a boundary judge has read.
    pub judged: Vec<u32>,
    /// Steps Fleet told the agent it had stopped on, where only its own act ends them.
    pub stalled: Vec<u32>,
    /// Each step the Session began, in order, with the time. A step it went back to appears again.
    pub visits: Vec<(u32, String)>,
}

/// How a Session was started, as it stood when its process first ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptStarted {
    /// The workflow's name as it read then. `None` is a custom Session.
    pub workflow: Option<String>,
    /// `auto_land`, `open_pr` or `keep_local`, where one was chosen by then.
    pub delivery: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// `ask`, `auto`, `accept_edits` or `plan`.
    pub mode: String,
}

/// Where a concern points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCite {
    pub region: String,
    pub path: Option<String>,
    pub from_line: u32,
    pub to_line: u32,
}

/// One concern a judge raised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptConcern {
    /// Counted from one in the Session; the wire's item id is `judge:<n>`.
    pub n: u32,
    pub text: String,
    pub since: String,
    /// The step it came from, and its label as it read then. Absent for the tile.
    pub from_step: Option<(u32, String)>,
    pub cites: Vec<KeptCite>,
    /// What the judge said should be seen, what it saw instead and what that does. `text` is these
    /// joined, which is what the agent is sent. Absent on a concern kept before the parts were.
    pub expected: Option<String>,
    pub produced: Option<String>,
    pub consequence: Option<String>,
    /// `sent` or `dismissed`, and when. Absent while it waits.
    pub settled: Option<(String, String)>,
}

/// The last reading's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptReading {
    pub at: String,
    pub concerns: u32,
}

/// Settled concerns kept per Session, newest last. The thread draws each as a line.
pub const SETTLED_KEPT: usize = 40;

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a Job's rail")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a Job's rail")(why))
}

fn listed(steps: &[u32]) -> String {
    steps.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

fn unlisted(text: &str) -> Vec<u32> {
    text.split(',').filter_map(|one| one.trim().parse().ok()).collect()
}

fn visits_text(visits: &[(u32, String)]) -> String {
    visits.iter().map(|(step, since)| format!("{step}\t{since}")).collect::<Vec<_>>().join("\n")
}

fn visits_of(text: &str) -> Vec<(u32, String)> {
    text.lines().filter_map(|line| line.split_once('\t').and_then(|(step, since)| Some((step.parse().ok()?, since.to_string())))).collect()
}

fn cites_text(cites: &[KeptCite]) -> String {
    cites
        .iter()
        .map(|one| {
            format!("{}\t{}\t{}\t{}", one.region, one.path.as_deref().unwrap_or(""), one.from_line, one.to_line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cites_of(text: &str) -> Vec<KeptCite> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let region = parts.next()?.to_string();
            let path = parts.next().filter(|path| !path.is_empty()).map(str::to_string);
            let from_line = parts.next()?.parse().ok()?;
            let to_line = parts.next()?.parse().ok()?;
            Some(KeptCite { region, path, from_line, to_line })
        })
        .collect()
}

const CONCERN: &str = "n, text, since, from_step, from_label, cites, how, settled_at, expected, produced, consequence";

fn part(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<Option<String>> {
    Ok(Some(row.get::<_, String>(at)?).filter(|said| !said.is_empty()))
}

fn concern(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptConcern> {
    let from: Option<u32> = row.get(3)?;
    let label: String = row.get(4)?;
    let cites: String = row.get(5)?;
    let how: Option<String> = row.get(6)?;
    let at: Option<String> = row.get(7)?;
    Ok(KeptConcern {
        n: row.get(0)?,
        text: row.get(1)?,
        since: row.get(2)?,
        from_step: from.map(|step| (step, label)),
        cites: cites_of(&cites),
        expected: part(row, 8)?,
        produced: part(row, 9)?,
        consequence: part(row, 10)?,
        settled: how.zip(at),
    })
}

impl Store {
    /// What the rail has done for the Session. A Session it has done nothing for reads as default.
    pub fn session_rail(&self, session_id: &str) -> Result<KeptRail, WriteError> {
        let found = self
            .conn
            .query_row("SELECT step, nudged, judged, visits, stalled FROM session_rail WHERE session_id = ?1", [session_id], |row| {
                let (nudged, judged, visits, stalled): (String, String, String, String) =
                    (row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?);
                Ok(KeptRail {
                    step: row.get(0)?,
                    nudged: unlisted(&nudged),
                    judged: unlisted(&judged),
                    stalled: unlisted(&stalled),
                    visits: visits_of(&visits),
                })
            })
            .optional()
            .map_err(reading)?;
        Ok(found.unwrap_or_default())
    }

    pub fn keep_session_rail(&mut self, session_id: &str, rail: &KeptRail) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_rail (session_id, step, nudged, judged, visits, stalled) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (session_id) DO UPDATE SET step = ?2, nudged = ?3, judged = ?4, visits = ?5, stalled = ?6",
                params![
                    session_id,
                    rail.step,
                    listed(&rail.nudged),
                    listed(&rail.judged),
                    visits_text(&rail.visits),
                    listed(&rail.stalled)
                ],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// How the Session was started, where it has begun.
    pub fn session_started(&self, session_id: &str) -> Result<Option<KeptStarted>, WriteError> {
        self.conn
            .query_row("SELECT workflow, delivery, model, effort, mode FROM session_started WHERE session_id = ?1", [session_id], |row| {
                Ok(KeptStarted { workflow: row.get(0)?, delivery: row.get(1)?, model: row.get(2)?, effort: row.get(3)?, mode: row.get(4)? })
            })
            .optional()
            .map_err(reading)
    }

    /// Keep how the Session was started. **The first write stands**: a Session that starts a process
    /// again keeps the start it had.
    pub fn keep_session_started(&mut self, session_id: &str, started: &KeptStarted) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO session_started (session_id, workflow, delivery, model, effort, mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, started.workflow, started.delivery, started.model, started.effort, started.mode],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// Every concern of the Session, oldest first, waiting and settled.
    pub fn session_concerns(&self, session_id: &str) -> Result<Vec<KeptConcern>, WriteError> {
        let mut read = self
            .conn
            .prepare(&format!("SELECT {CONCERN} FROM session_concerns WHERE session_id = ?1 ORDER BY n"))
            .map_err(reading)?;
        let found = read
            .query_map([session_id], concern)
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading);
        found
    }

    /// Keep a new concern, numbered after the Session's last. Answers its number.
    pub fn keep_session_concern(&mut self, session_id: &str, new: &KeptConcern) -> Result<u32, WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        let n: u32 = kept
            .query_row("SELECT COALESCE(MAX(n), 0) + 1 FROM session_concerns WHERE session_id = ?1", [session_id], |row| row.get(0))
            .map_err(keeping)?;
        kept.execute(
            "INSERT INTO session_concerns (session_id, n, text, since, from_step, from_label, cites, expected, produced, consequence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                session_id,
                n,
                new.text,
                new.since,
                new.from_step.as_ref().map(|(step, _)| step),
                new.from_step.as_ref().map(|(_, label)| label.as_str()).unwrap_or(""),
                cites_text(&new.cites),
                new.expected.as_deref().unwrap_or(""),
                new.produced.as_deref().unwrap_or(""),
                new.consequence.as_deref().unwrap_or("")
            ],
        )
        .map_err(keeping)?;
        // Only settled ones are let go of, oldest first: a waiting concern is never trimmed away.
        kept.execute(
            "DELETE FROM session_concerns WHERE session_id = ?1 AND how IS NOT NULL AND n <= (
                 SELECT n FROM session_concerns WHERE session_id = ?1 AND how IS NOT NULL
                 ORDER BY n DESC LIMIT 1 OFFSET ?2)",
            params![session_id, SETTLED_KEPT as i64],
        )
        .map_err(keeping)?;
        kept.commit().map_err(keeping)?;
        Ok(n)
    }

    /// Settle concern `n` as `how` (`sent` or `dismissed`) at `at`, if it is waiting. Whether it was.
    pub fn settle_session_concern(&mut self, session_id: &str, n: u32, how: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE session_concerns SET how = ?3, settled_at = ?4
                 WHERE session_id = ?1 AND n = ?2 AND how IS NULL",
                params![session_id, n, how, at],
            )
            .map(|changed| changed == 1)
            .map_err(keeping)
    }

    pub fn session_judge_reading(&self, session_id: &str) -> Result<Option<KeptReading>, WriteError> {
        self.conn
            .query_row("SELECT at, concerns FROM session_judge_reading WHERE session_id = ?1", [session_id], |row| {
                Ok(KeptReading { at: row.get(0)?, concerns: row.get(1)? })
            })
            .optional()
            .map_err(reading)
    }

    pub fn keep_session_judge_reading(&mut self, session_id: &str, outcome: &KeptReading) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_judge_reading (session_id, at, concerns) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id) DO UPDATE SET at = ?2, concerns = ?3",
                params![session_id, outcome.at, outcome.concerns],
            )
            .map(|_| ())
            .map_err(keeping)
    }

    /// Whether the Session has run `armada check`: a Check run it asked for exists.
    pub fn session_has_run_a_check(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT EXISTS (SELECT 1 FROM session_check_runs WHERE session_id = ?1)", [session_id], |row| {
                row.get(0)
            })
            .map_err(reading)
    }
}
