//! The merge line Fleet keeps for each repository it serves: who is waiting,
//! who holds the turn, and how many a turn takes. `docs/capabilities/merge-line.md`.
//!
//! **Everything a restart must not lose is a row.** An entry's place is its
//! id, which is never reused; its outcome is written onto it and it stays as
//! the record. The turn is a row too, so a Fleet that dies mid-gate leaves it
//! for the next to find. Nothing here decides whether a holder is still alive:
//! that is a question about a process, and `fleet` asks it.
//!
//! Over 500 lines because the three tables are one line's state and are read
//! and written together; a split would move the count and leave the coupling.

use core_model::{JobId, Timestamp, Ulid};
use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 106 — the merge line, its turn, and the size a turn takes.
///
/// **One waiting entry per Job**, by a partial unique index: a press made again
/// after a restart finds the entry it left and keeps its place. **One turn per
/// repository**, by the primary key. Entries point at `jobs`, so
/// `forget_job` takes a forgotten Job's with it.
pub(crate) const V106: &str = r#"
CREATE TABLE merge_line_entries (
    entry_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    repository   TEXT NOT NULL,
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    nonce        TEXT NOT NULL,
    actor        TEXT NOT NULL,
    pull_request TEXT NOT NULL,
    queued_at    TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'waiting'
                 CHECK (state IN ('waiting', 'landed', 'refused', 'stopped')),
    kind         TEXT,
    said         TEXT,
    merge_commit TEXT,
    base         TEXT,
    blamed       TEXT CHECK (blamed IN ('branch', 'base')),
    ended_at     TEXT,
    finished     INTEGER NOT NULL DEFAULT 0 CHECK (finished IN (0, 1)),
    held_back    TEXT NOT NULL DEFAULT 'none'
                 CHECK (held_back IN ('clash_member', 'clash_main', 'kept_place_after_red',
                                      'joined_after_turn_began', 'none')),
    next_position INTEGER,
    CHECK ((state = 'waiting') = (ended_at IS NULL))
) STRICT;

CREATE UNIQUE INDEX merge_line_one_waiting_a_job
    ON merge_line_entries (job_id) WHERE state = 'waiting';
CREATE INDEX merge_line_by_repository
    ON merge_line_entries (repository, state, entry_id);

CREATE TABLE merge_line_turns (
    repository TEXT PRIMARY KEY,
    run        TEXT NOT NULL,
    pid        INTEGER NOT NULL,
    started    TEXT NOT NULL,
    taken_at   TEXT NOT NULL
) STRICT;

CREATE TABLE merge_line_sizes (
    repository TEXT PRIMARY KEY,
    size       INTEGER NOT NULL CHECK (size >= 1),
    reason     TEXT NOT NULL,
    at         TEXT NOT NULL
) STRICT;
"#;

/// Where an entry stands. Only `Waiting` is in line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineState {
    Waiting,
    Landed,
    /// The merge was refused and the Job is where it was.
    Refused,
    /// Taken out of line without being tried: the Job had left its gate.
    Stopped,
}

impl LineState {
    fn word(self) -> &'static str {
        match self {
            LineState::Waiting => "waiting",
            LineState::Landed => "landed",
            LineState::Refused => "refused",
            LineState::Stopped => "stopped",
        }
    }

    fn read(word: &str) -> Option<LineState> {
        [
            LineState::Waiting,
            LineState::Landed,
            LineState::Refused,
            LineState::Stopped,
        ]
        .into_iter()
        .find(|state| state.word() == word)
    }
}

/// Why a waiting entry is not in the turn that is running. A closed set, kept
/// so the reason outlives the turn that made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldBack {
    /// Left out because it clashes with another member of the batch.
    ClashMember,
    /// Left out because it clashes with the base.
    ClashMain,
    /// Went red, and kept its place.
    KeptPlaceAfterRed,
    /// Joined after the running turn began.
    JoinedAfterTurnBegan,
    /// Just queued behind.
    None,
}

impl HeldBack {
    fn word(self) -> &'static str {
        match self {
            HeldBack::ClashMember => "clash_member",
            HeldBack::ClashMain => "clash_main",
            HeldBack::KeptPlaceAfterRed => "kept_place_after_red",
            HeldBack::JoinedAfterTurnBegan => "joined_after_turn_began",
            HeldBack::None => "none",
        }
    }

    fn read(word: &str) -> Option<HeldBack> {
        [
            HeldBack::ClashMember,
            HeldBack::ClashMain,
            HeldBack::KeptPlaceAfterRed,
            HeldBack::JoinedAfterTurnBegan,
            HeldBack::None,
        ]
        .into_iter()
        .find(|reason| reason.word() == word)
    }
}

/// Whose failure a red Check was, once the base was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blame {
    Branch,
    Base,
}

impl Blame {
    fn word(self) -> &'static str {
        match self {
            Blame::Branch => "branch",
            Blame::Base => "base",
        }
    }
}

/// One Job's place in a repository's line, and what came of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineEntry {
    /// Its place: ascending, and never reused.
    pub id: i64,
    pub repository: String,
    pub job_id: JobId,
    /// Minted per entry, so an outcome is written only to the entry it was
    /// taken for.
    pub nonce: String,
    /// Who pressed, as `Actor::as_wire` spells it.
    pub actor: String,
    pub pull_request: String,
    pub queued_at: Timestamp,
    pub state: LineState,
    /// `NotMerged::kind` where the merge was refused.
    pub kind: Option<String>,
    pub said: Option<String>,
    pub merge_commit: Option<String>,
    pub base: Option<String>,
    pub blamed: Option<Blame>,
    pub ended_at: Option<Timestamp>,
    /// Whether the Job was moved on after the landing. Set last, so a landing
    /// a Fleet died before finishing is finished by the next.
    pub finished: bool,
    pub held_back: HeldBack,
    /// 1-based, in the order the next turn takes the line: the running batch
    /// first, then place order. `None` once the entry has left the line.
    pub next_position: Option<u32>,
}

/// What an entry came to, written when it leaves the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub state: LineState,
    pub kind: Option<String>,
    pub said: Option<String>,
    pub merge_commit: Option<String>,
    pub base: Option<String>,
    pub blamed: Option<Blame>,
}

/// Which process holds a repository's turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnHolder {
    /// The Fleet run, `Fleet::run_id`.
    pub run: String,
    pub pid: u32,
    /// What `ps` said that pid started at, so a recycled pid is not taken for
    /// the holder.
    pub started: String,
}

/// How many entries a turn takes, and why it is that many.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineSize {
    pub size: u32,
    pub reason: String,
    pub at: Timestamp,
}

impl From<rusqlite::Error> for WriteError {
    fn from(cause: rusqlite::Error) -> WriteError {
        WriteError::Database(fault("keeping the merge line")(cause))
    }
}

const COLUMNS: &str = "entry_id, repository, job_id, nonce, actor, pull_request, queued_at, \
                       state, kind, said, merge_commit, base, blamed, ended_at, finished, held_back, \
                       next_position";

fn entry_of(row: &Row<'_>) -> rusqlite::Result<LineEntry> {
    let bad = |what: &str| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            format!("a merge line entry with {what} this build does not know").into(),
        )
    };
    let state: String = row.get("state")?;
    let blamed: Option<String> = row.get("blamed")?;
    let ended_at: Option<String> = row.get("ended_at")?;
    Ok(LineEntry {
        id: row.get("entry_id")?,
        repository: row.get("repository")?,
        job_id: JobId::carried(Ulid::carried(row.get::<_, String>("job_id")?)),
        nonce: row.get("nonce")?,
        actor: row.get("actor")?,
        pull_request: row.get("pull_request")?,
        queued_at: Timestamp::from_rfc3339(row.get::<_, String>("queued_at")?),
        state: LineState::read(&state).ok_or_else(|| bad("a state"))?,
        kind: row.get("kind")?,
        said: row.get("said")?,
        merge_commit: row.get("merge_commit")?,
        base: row.get("base")?,
        blamed: match blamed.as_deref() {
            None => None,
            Some("branch") => Some(Blame::Branch),
            Some("base") => Some(Blame::Base),
            Some(_) => return Err(bad("a blame")),
        },
        ended_at: ended_at.map(Timestamp::from_rfc3339),
        finished: row.get::<_, i64>("finished")? == 1,
        held_back: HeldBack::read(&row.get::<_, String>("held_back")?)
            .ok_or_else(|| bad("a reason"))?,
        next_position: row
            .get::<_, Option<i64>>("next_position")?
            .and_then(|at| u32::try_from(at).ok()),
    })
}

impl Store {
    /// Put a Job in a repository's line, or return the entry it already has
    /// waiting there. **A second press keeps the place the first took.**
    pub fn join_the_line(
        &mut self,
        repository: &str,
        job_id: &JobId,
        nonce: &str,
        actor: &str,
        pull_request: &str,
        at: &Timestamp,
    ) -> Result<LineEntry, WriteError> {
        let doing = fault("putting a Job in the merge line");
        let tx = self.conn.transaction().map_err(doing)?;
        let waiting = tx
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM merge_line_entries \
                     WHERE job_id = ?1 AND state = 'waiting'"
                ),
                (job_id.as_str(),),
                entry_of,
            )
            .optional()
            .map_err(fault("reading the merge line"))?;
        let entry = match waiting {
            Some(entry) => entry,
            None => {
                let turning = holder_in(&tx, repository)?.is_some();
                let held_back = if turning {
                    HeldBack::JoinedAfterTurnBegan
                } else {
                    HeldBack::None
                };
                tx.execute(
                    "INSERT INTO merge_line_entries \
                     (repository, job_id, nonce, actor, pull_request, queued_at, held_back) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    (
                        repository,
                        job_id.as_str(),
                        nonce,
                        actor,
                        pull_request,
                        at.as_str(),
                        held_back.word(),
                    ),
                )
                .map_err(fault("putting a Job in the merge line"))?;
                let id = tx.last_insert_rowid();
                tx.query_row(
                    &format!("SELECT {COLUMNS} FROM merge_line_entries WHERE entry_id = ?1"),
                    (id,),
                    entry_of,
                )
                .map_err(fault("reading the merge line"))?
            }
        };
        tx.commit()
            .map_err(fault("putting a Job in the merge line"))?;
        Ok(entry)
    }

    /// The entries waiting in a repository's line, in place order.
    pub fn waiting_in_line(&self, repository: &str) -> Result<Vec<LineEntry>, WriteError> {
        self.entries_where(
            "repository = ?1 AND state = 'waiting' ORDER BY entry_id",
            repository,
        )
    }

    /// Landings whose Job was not yet moved on, in place order.
    pub fn landings_not_finished(&self, repository: &str) -> Result<Vec<LineEntry>, WriteError> {
        self.entries_where(
            "repository = ?1 AND state = 'landed' AND finished = 0 ORDER BY entry_id",
            repository,
        )
    }

    fn entries_where(&self, clause: &str, repository: &str) -> Result<Vec<LineEntry>, WriteError> {
        let doing = fault("reading the merge line");
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM merge_line_entries WHERE {clause}"
            ))
            .map_err(doing)?;
        let rows = statement
            .query_map((repository,), entry_of)
            .map_err(fault("reading the merge line"))?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading the merge line"))
            .map_err(WriteError::Database)
    }

    /// One entry, by place.
    pub fn line_entry(&self, id: i64) -> Result<Option<LineEntry>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM merge_line_entries WHERE entry_id = ?1"),
                (id,),
                entry_of,
            )
            .optional()
            .map_err(fault("reading the merge line"))
            .map_err(WriteError::Database)
    }

    /// Every repository with an entry waiting or a landing not finished.
    pub fn lines_with_work(&self) -> Result<Vec<String>, WriteError> {
        let doing = fault("reading the merge line");
        let mut statement = self
            .conn
            .prepare(
                "SELECT DISTINCT repository FROM merge_line_entries \
                 WHERE state = 'waiting' OR (state = 'landed' AND finished = 0) \
                 ORDER BY repository",
            )
            .map_err(doing)?;
        let rows = statement
            .query_map((), |row| row.get::<_, String>(0))
            .map_err(fault("reading the merge line"))?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading the merge line"))
            .map_err(WriteError::Database)
    }

    /// Take an entry out of line with what came of it. **Only while it is
    /// still waiting with this nonce**; `false` is an entry that was ended
    /// already, or is another's.
    pub fn end_line_entry(
        &mut self,
        id: i64,
        nonce: &str,
        ended: &Ended,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        debug_assert!(ended.state != LineState::Waiting);
        let changed = self
            .conn
            .execute(
                "UPDATE merge_line_entries SET state = ?3, kind = ?4, said = ?5, \
                 merge_commit = ?6, base = ?7, blamed = ?8, ended_at = ?9, next_position = NULL \
                 WHERE entry_id = ?1 AND nonce = ?2 AND state = 'waiting'",
                (
                    id,
                    nonce,
                    ended.state.word(),
                    &ended.kind,
                    &ended.said,
                    &ended.merge_commit,
                    &ended.base,
                    ended.blamed.map(Blame::word),
                    at.as_str(),
                ),
            )
            .map_err(fault("ending a merge line entry"))
            .map_err(WriteError::Database)?;
        Ok(changed == 1)
    }

    /// Record, for each waiting entry of a repository, where the next turn
    /// takes it and why it is not in the running turn. `running` are the ids in
    /// the turn now, first; the rest follow in place order. A running entry is
    /// held back for no reason; a waiting one keeps the reason it was given
    /// unless `reasons` names another.
    pub fn order_the_line(
        &mut self,
        repository: &str,
        running: &[i64],
        reasons: &[(i64, HeldBack)],
    ) -> Result<(), WriteError> {
        let tx = self.conn.transaction()?;
        let mut waiting: Vec<i64> = {
            let mut statement = tx.prepare(
                "SELECT entry_id FROM merge_line_entries \
                 WHERE repository = ?1 AND state = 'waiting' ORDER BY entry_id",
            )?;
            let rows = statement.query_map((repository,), |row| row.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        waiting.sort_by_key(|id| (!running.contains(id), *id));
        for (at, id) in waiting.iter().enumerate() {
            let reason = match (
                running.contains(id),
                reasons.iter().find(|(one, _)| one == id),
            ) {
                (true, _) => Some(HeldBack::None),
                (false, Some((_, reason))) => Some(*reason),
                (false, None) => None,
            };
            tx.execute(
                "UPDATE merge_line_entries SET next_position = ?2, \
                 held_back = COALESCE(?3, held_back) WHERE entry_id = ?1",
                (
                    id,
                    i64::try_from(at + 1).unwrap_or(i64::MAX),
                    reason.map(HeldBack::word),
                ),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Mark a landing as moved on. `false` where it already was.
    pub fn finish_line_entry(&mut self, id: i64) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE merge_line_entries SET finished = 1 \
                 WHERE entry_id = ?1 AND state = 'landed' AND finished = 0",
                (id,),
            )
            .map_err(fault("finishing a merge line entry"))
            .map_err(WriteError::Database)?;
        Ok(changed == 1)
    }

    /// Take a repository's turn. `None` is taken; `Some` is who already holds
    /// it, and nothing changed.
    pub fn hold_turn(
        &mut self,
        repository: &str,
        holder: &TurnHolder,
        at: &Timestamp,
    ) -> Result<Option<TurnHolder>, WriteError> {
        let doing = fault("taking the merge line's turn");
        let tx = self.conn.transaction().map_err(doing)?;
        let held = holder_in(&tx, repository)?;
        if held.is_none() {
            tx.execute(
                "INSERT INTO merge_line_turns (repository, run, pid, started, taken_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    repository,
                    &holder.run,
                    i64::from(holder.pid),
                    &holder.started,
                    at.as_str(),
                ),
            )
            .map_err(fault("taking the merge line's turn"))?;
        }
        tx.commit().map_err(fault("taking the merge line's turn"))?;
        Ok(held)
    }

    /// Who holds a repository's turn, if anybody.
    pub fn turn_held(&self, repository: &str) -> Result<Option<TurnHolder>, WriteError> {
        holder_in(&self.conn, repository)
    }

    /// Take the turn from the run that held it, because that process is gone.
    /// `false` where it is no longer that run's, which means somebody else
    /// took it first.
    pub fn take_over_turn(
        &mut self,
        repository: &str,
        from_run: &str,
        holder: &TurnHolder,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE merge_line_turns SET run = ?3, pid = ?4, started = ?5, taken_at = ?6 \
                 WHERE repository = ?1 AND run = ?2",
                (
                    repository,
                    from_run,
                    &holder.run,
                    i64::from(holder.pid),
                    &holder.started,
                    at.as_str(),
                ),
            )
            .map_err(fault("taking over the merge line's turn"))
            .map_err(WriteError::Database)?;
        Ok(changed == 1)
    }

    /// Give the turn up. Only the run that holds it can.
    pub fn release_turn(&mut self, repository: &str, run: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM merge_line_turns WHERE repository = ?1 AND run = ?2",
                (repository, run),
            )
            .map_err(fault("giving up the merge line's turn"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// How many a turn takes in this repository, where it has been kept.
    pub fn line_size(&self, repository: &str) -> Result<Option<LineSize>, WriteError> {
        self.conn
            .query_row(
                "SELECT size, reason, at FROM merge_line_sizes WHERE repository = ?1",
                (repository,),
                |row| {
                    Ok(LineSize {
                        size: u32::try_from(row.get::<_, i64>(0)?).unwrap_or(1),
                        reason: row.get(1)?,
                        at: Timestamp::from_rfc3339(row.get::<_, String>(2)?),
                    })
                },
            )
            .optional()
            .map_err(fault("reading the merge line's size"))
            .map_err(WriteError::Database)
    }

    /// Keep how many a turn takes, written whole.
    pub fn keep_line_size(
        &mut self,
        repository: &str,
        size: u32,
        reason: &str,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO merge_line_sizes (repository, size, reason, at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (repository) DO UPDATE SET size = ?2, reason = ?3, at = ?4",
                (repository, i64::from(size.max(1)), reason, at.as_str()),
            )
            .map_err(fault("keeping the merge line's size"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}

fn holder_in(
    conn: &rusqlite::Connection,
    repository: &str,
) -> Result<Option<TurnHolder>, WriteError> {
    conn.query_row(
        "SELECT run, pid, started FROM merge_line_turns WHERE repository = ?1",
        (repository,),
        |row| {
            Ok(TurnHolder {
                run: row.get(0)?,
                pid: u32::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
                started: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(fault("reading the merge line's turn"))
    .map_err(WriteError::Database)
}
