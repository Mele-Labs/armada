//! What a Job asked the Operator with `raise_conflict` and `ask_held`, kept until it is answered.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The two kinds of ask, as the table names them.
pub mod kind {
    pub const CONFLICT: &str = "conflict";
    pub const HELD: &str = "held";
}

/// An ask while it stands, and once it is answered.
pub mod standing {
    pub const OPEN: &str = "open";
    pub const ANSWERED: &str = "answered";
}

/// One ask, as it was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJobAsk {
    /// `conflict:<a>:<b>` or `held:<repository>:<pull or branch>`.
    pub id: String,
    /// Which time the same thing was asked: 0 the first.
    pub round: u32,
    pub kind: String,
    /// The Job that asked first.
    pub session_id: String,
    /// Its title when it asked.
    pub job_title: String,
    /// The Job a conflict names.
    pub other_session_id: Option<String>,
    pub other_title: Option<String>,
    pub manifest_id: String,
    pub files: Vec<String>,
    pub wire: Vec<String>,
    pub mine: String,
    pub theirs: String,
    pub breaks: String,
    pub pull: Option<u64>,
    pub branch: Option<String>,
    /// The pull request's title, where one was read.
    pub title: Option<String>,
    /// `passed`, `pending` or `failed`, where the forge said.
    pub checks: Option<String>,
    /// What else raised the same ask while it stood, oldest first.
    pub also_raised_by: Vec<String>,
    /// It is on the sender's Needs you, or in Your turn.
    pub raised: bool,
    pub at: String,
    pub state: String,
    pub answered_at: Option<String>,
    pub answer: Option<String>,
}

/// What putting an ask to the store did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobAskPut {
    /// A new row, this round.
    Opened(u32),
    /// An open ask about the same thing took it as one more who raised it.
    Folded,
}

const COLUMNS: &str = "id, round, kind, session_id, job_title, other_session_id, other_title, manifest_id, files, wire, mine, theirs, breaks, pull, branch, title, checks, also_raised_by, raised, at, state, answered_at, answer";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn lines(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptJobAsk> {
    Ok(KeptJobAsk {
        id: row.get(0)?,
        round: row.get(1)?,
        kind: row.get(2)?,
        session_id: row.get(3)?,
        job_title: row.get(4)?,
        other_session_id: row.get(5)?,
        other_title: row.get(6)?,
        manifest_id: row.get(7)?,
        files: lines(row.get(8)?),
        wire: lines(row.get(9)?),
        mine: row.get(10)?,
        theirs: row.get(11)?,
        breaks: row.get(12)?,
        pull: row.get(13)?,
        branch: row.get(14)?,
        title: row.get(15)?,
        checks: row.get(16)?,
        also_raised_by: lines(row.get(17)?),
        raised: row.get::<_, i64>(18)? != 0,
        at: row.get(19)?,
        state: row.get(20)?,
        answered_at: row.get(21)?,
        answer: row.get(22)?,
    })
}

impl Store {
    /// Put `ask`, whose `round` and `state` are the store's to set. **An open ask about the same thing takes it
    /// as one more who raised it** (`also` names it), and nothing new is kept.
    pub fn put_job_ask(&mut self, ask: &KeptJobAsk, also: &str) -> Result<JobAskPut, WriteError> {
        let doing = "keeping what a Job asked the Operator";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let open: Option<(u32, String)> = tx
            .query_row("SELECT round, also_raised_by FROM job_asks WHERE id = ?1 AND state = ?2", params![ask.id, standing::OPEN], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .map_err(failed(doing))?;
        if let Some((round, raised_by)) = open {
            let mut by = lines(raised_by);
            by.push(also.to_string());
            let by = serde_json::to_string(&by).unwrap_or_else(|_| String::from("[]"));
            tx.execute("UPDATE job_asks SET also_raised_by = ?3 WHERE id = ?1 AND round = ?2", params![ask.id, round, by]).map_err(failed(doing))?;
            tx.commit().map_err(failed(doing))?;
            return Ok(JobAskPut::Folded);
        }
        let round: u32 = tx.query_row("SELECT COUNT(*) FROM job_asks WHERE id = ?1", [&ask.id], |row| row.get(0)).map_err(failed(doing))?;
        let json = |lines: &[String]| serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"));
        tx.execute(
            &format!("INSERT INTO job_asks ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, NULL, NULL)"),
            params![
                ask.id,
                round,
                ask.kind,
                ask.session_id,
                ask.job_title,
                ask.other_session_id,
                ask.other_title,
                ask.manifest_id,
                json(&ask.files),
                json(&ask.wire),
                ask.mine,
                ask.theirs,
                ask.breaks,
                ask.pull,
                ask.branch,
                ask.title,
                ask.checks,
                json(&ask.also_raised_by),
                i64::from(ask.raised),
                ask.at,
                standing::OPEN,
            ],
        )
        .map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))?;
        Ok(JobAskPut::Opened(round))
    }

    /// The ask `id` names that still stands.
    pub fn open_job_ask(&self, id: &str) -> Result<Option<KeptJobAsk>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_asks WHERE id = ?1 AND state = ?2"), params![id, standing::OPEN], kept)
            .optional()
            .map_err(failed("reading what a Job asked the Operator"))
    }

    /// The newest ask `id` names, answered or not.
    pub fn latest_job_ask(&self, id: &str) -> Result<Option<KeptJobAsk>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_asks WHERE id = ?1 ORDER BY round DESC LIMIT 1"), [id], kept)
            .optional()
            .map_err(failed("reading what a Job asked the Operator"))
    }

    /// The asks `session` made that stand and are on its Needs you or in Your turn, oldest first.
    pub fn raised_job_asks_of(&self, session: &str) -> Result<Vec<KeptJobAsk>, WriteError> {
        let doing = "reading what a Job asked the Operator";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_asks WHERE session_id = ?1 AND state = ?2 AND raised = 1 ORDER BY at, id"))
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![session, standing::OPEN], kept).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Every open ask nothing has raised yet, oldest first: what the Operator is handed, or what a Needs you waits for.
    pub fn unraised_job_asks(&self) -> Result<Vec<KeptJobAsk>, WriteError> {
        let doing = "reading what a Job asked the Operator";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_asks WHERE state = ?1 AND raised = 0 ORDER BY at, id"))
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![standing::OPEN], kept).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Put the open ask `id` on the sender's Needs you. Whether it moved.
    pub fn raise_job_ask(&mut self, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE job_asks SET raised = 1 WHERE id = ?1 AND state = ?2 AND raised = 0", params![id, standing::OPEN])
            .map(|moved| moved > 0)
            .map_err(failed("raising what a Job asked the Operator"))
    }

    /// Answer the open ask `id` with `answer` at `at`. **The first answer wins**: whether this one moved it.
    pub fn answer_job_ask(&mut self, id: &str, answer: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE job_asks SET state = ?4, answered_at = ?3, answer = ?2 WHERE id = ?1 AND state = ?5",
                params![id, answer, at, standing::ANSWERED, standing::OPEN],
            )
            .map(|moved| moved > 0)
            .map_err(failed("answering what a Job asked the Operator"))
    }

    /// When pull request `pull` of repository `manifest` merged, as the timeline kept it. Absent where it kept no merge.
    pub fn pull_merged_at(&self, manifest: &str, pull: u64) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT MIN(at) FROM timeline_events WHERE kind = 'pr.merged' AND subject_kind = 'pull_request' AND subject_id = ?1 AND repo = ?2",
                params![pull.to_string(), manifest],
                |row| row.get(0),
            )
            .map_err(failed("reading when a pull request merged"))
    }
}
