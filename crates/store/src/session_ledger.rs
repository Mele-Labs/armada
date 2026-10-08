//! The session ledger: every agent session a person runs, and what each holds.
//! `docs/concepts/session.md`.
//!
//! **Two tables, and the second is not the first's child.** `sessions` is one row
//! a session. `ledger_attachments` is what a holder has taken or done, keyed by
//! `(holder_kind, holder_id)` with no foreign key, so a Job or a need can hold
//! rows too without a table of its own.
//!
//! **`kind` is open text and `state` is not.** A new kind of attachment is a new
//! word and no migration; a released slot and a spent need both read as a
//! state, so every kind is asked the same way.

use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Whether a session is still running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Live,
    Ended,
}

impl SessionState {
    fn as_text(self) -> &'static str {
        match self {
            SessionState::Live => "live",
            SessionState::Ended => "ended",
        }
    }

    fn of(text: &str) -> SessionState {
        if text == "ended" {
            SessionState::Ended
        } else {
            SessionState::Live
        }
    }
}

/// The figures a session last reported. Each is absent until reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionFigures {
    pub context_tokens: Option<u64>,
    pub context_window: Option<u64>,
    pub cost_micros: Option<u64>,
}

/// One session, as the ledger holds it. Instants are RFC3339, stamped by Fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSession {
    /// The id its harness knows it by.
    pub id: String,
    pub harness: String,
    /// What started it: `terminal` today, and `bridge` for one Bridge hosts.
    pub origin: String,
    /// The repository it stands in, where Fleet serves one.
    pub manifest_id: Option<String>,
    pub cwd: String,
    pub title: Option<String>,
    pub state: SessionState,
    pub started_at: String,
    pub last_seen_at: String,
    pub last_turn_at: Option<String>,
    pub ended_at: Option<String>,
    pub end_reason: Option<String>,
    pub figures: SessionFigures,
    /// The version of the `armada` mod a terminal session last reported. Absent from an older mod.
    pub mod_version: Option<String>,
}

/// What a row of the ledger is held by. Closed, because a holder is something
/// Fleet knows the kind of; an attachment's own kind is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderKind {
    Session,
    Job,
}

impl HolderKind {
    pub fn as_text(self) -> &'static str {
        match self {
            HolderKind::Session => "session",
            HolderKind::Job => "job",
        }
    }

    fn of(text: &str) -> Option<HolderKind> {
        match text {
            "session" => Some(HolderKind::Session),
            "job" => Some(HolderKind::Job),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    pub kind: HolderKind,
    pub id: String,
}

impl Holder {
    pub fn job(id: impl Into<String>) -> Holder {
        Holder {
            kind: HolderKind::Job,
            id: id.into(),
        }
    }

    pub fn session(id: impl Into<String>) -> Holder {
        Holder {
            kind: HolderKind::Session,
            id: id.into(),
        }
    }
}

/// Where an attachment stands. **One vocabulary for every kind**: a slot let go
/// and a need whose branch landed are different words and the same question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentState {
    /// In force: the slot is held, the pull request is open.
    Standing,
    /// Done with, having been used: a merged pull request, a need whose branch
    /// landed.
    Spent,
    /// Let go before it was used: a released slot, a session that ended in it.
    GivenBack,
}

impl AttachmentState {
    pub fn as_text(self) -> &'static str {
        match self {
            AttachmentState::Standing => "standing",
            AttachmentState::Spent => "spent",
            AttachmentState::GivenBack => "given_back",
        }
    }

    fn of(text: &str) -> AttachmentState {
        match text {
            "spent" => AttachmentState::Spent,
            "given_back" => AttachmentState::GivenBack,
            _ => AttachmentState::Standing,
        }
    }
}

/// One thing a holder took or did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAttachment {
    pub holder: Holder,
    /// Open: `slot`, `branch`, `pr`, `job`, `studio`, `subagent`, `message`, `artifact`, and
    /// whatever a later holder needs.
    pub kind: String,
    /// The repository a slot or branch is of; empty where there is none.
    pub manifest_id: String,
    pub target: String,
    pub state: AttachmentState,
    pub detail: BTreeMap<String, String>,
    pub since: String,
    pub changed_at: String,
}

/// What `find_sessions` is asked.
#[derive(Clone, Debug, Default)]
pub struct SessionSearch<'a> {
    pub manifest_id: Option<&'a str>,
    pub state: Option<SessionState>,
    /// A title, a branch, a pull request number, a Job id or a slot.
    pub text: Option<&'a str>,
}

const SESSION_COLUMNS: &str = "id, harness, origin, manifest_id, cwd, title, state, started_at, \
     last_seen_at, last_turn_at, ended_at, end_reason, usage, mod_version";

const ATTACHMENT_COLUMNS: &str =
    "holder_kind, holder_id, kind, manifest_id, target, state, detail, since, changed_at";

fn kept_session(row: &Row<'_>) -> rusqlite::Result<KeptSession> {
    let figures: Option<String> = row.get(12)?;
    let figure = |name: &str| -> Option<u64> {
        let parsed: serde_json::Value = serde_json::from_str(figures.as_deref()?).ok()?;
        parsed.get(name)?.as_u64()
    };
    Ok(KeptSession {
        id: row.get(0)?,
        harness: row.get(1)?,
        origin: row.get(2)?,
        manifest_id: row.get(3)?,
        cwd: row.get(4)?,
        title: row.get(5)?,
        state: SessionState::of(&row.get::<_, String>(6)?),
        started_at: row.get(7)?,
        last_seen_at: row.get(8)?,
        last_turn_at: row.get(9)?,
        ended_at: row.get(10)?,
        end_reason: row.get(11)?,
        figures: SessionFigures {
            context_tokens: figure("context_tokens"),
            context_window: figure("context_window"),
            cost_micros: figure("cost_micros"),
        },
        mod_version: row.get(13)?,
    })
}

fn kept_attachment(row: &Row<'_>) -> rusqlite::Result<Option<KeptAttachment>> {
    let Some(kind) = HolderKind::of(&row.get::<_, String>(0)?) else {
        // A holder kind this build does not know was written by a newer one.
        return Ok(None);
    };
    let detail: String = row.get(6)?;
    Ok(Some(KeptAttachment {
        holder: Holder {
            kind,
            id: row.get(1)?,
        },
        kind: row.get(2)?,
        manifest_id: row.get(3)?,
        target: row.get(4)?,
        state: AttachmentState::of(&row.get::<_, String>(5)?),
        detail: serde_json::from_str(&detail).unwrap_or_default(),
        since: row.get(7)?,
        changed_at: row.get(8)?,
    }))
}

fn figures_text(figures: &SessionFigures) -> Option<String> {
    let mut kept = serde_json::Map::new();
    for (name, value) in [
        ("context_tokens", figures.context_tokens),
        ("context_window", figures.context_window),
        ("cost_micros", figures.cost_micros),
    ] {
        if let Some(value) = value {
            kept.insert(name.into(), value.into());
        }
    }
    (!kept.is_empty()).then(|| serde_json::Value::Object(kept).to_string())
}

/// A search word with the characters `LIKE` reads as its own escaped.
fn literally(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl Store {
    /// Keep `session`, replacing the row of the same id.
    pub fn keep_session(&mut self, session: &KeptSession) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO sessions (id, harness, origin, manifest_id, cwd, title, \
                 state, started_at, last_seen_at, last_turn_at, ended_at, end_reason, usage, mod_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    session.id,
                    session.harness,
                    session.origin,
                    session.manifest_id,
                    session.cwd,
                    session.title,
                    session.state.as_text(),
                    session.started_at,
                    session.last_seen_at,
                    session.last_turn_at,
                    session.ended_at,
                    session.end_reason,
                    figures_text(&session.figures),
                    session.mod_version,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a session"))
            .map_err(WriteError::Database)
    }

    pub fn session(&self, id: &str) -> Result<Option<KeptSession>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {SESSION_COLUMNS} FROM sessions WHERE id = ?1"),
                [id],
                kept_session,
            )
            .optional()
            .map_err(fault("reading a session"))
            .map_err(WriteError::Database)
    }

    /// Sessions matching `search`, the most recently seen first.
    pub fn find_sessions(
        &self,
        search: &SessionSearch<'_>,
    ) -> Result<Vec<KeptSession>, WriteError> {
        let doing = "searching the session ledger";
        let text = search.text.map(str::trim).filter(|text| !text.is_empty());
        // A pull request or a slot is spelled `#12` or `slot-3` as often as `12`.
        let bare = text.map(|text| {
            text.trim_start_matches('#')
                .trim_start_matches("slot-")
                .to_string()
        });
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {SESSION_COLUMNS} FROM sessions
                 WHERE (?1 IS NULL OR manifest_id = ?1)
                   AND (?2 IS NULL OR state = ?2)
                   AND (?3 IS NULL
                        OR id = ?3
                        OR title LIKE ?4 ESCAPE '\\'
                        OR EXISTS (
                            SELECT 1 FROM ledger_attachments a
                            WHERE a.holder_kind = 'session' AND a.holder_id = sessions.id
                              AND ((a.kind IN ('pr', 'slot', 'job') AND a.target = ?5)
                                   OR (a.kind = 'job' AND a.target LIKE ?6 ESCAPE '\\')
                                   OR (a.kind = 'branch' AND a.target LIKE ?4 ESCAPE '\\'))))
                 ORDER BY last_seen_at DESC, id"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(
                params![
                    search.manifest_id,
                    search.state.map(SessionState::as_text),
                    text,
                    text.map(|text| format!("%{}%", literally(text))),
                    bare,
                    text.map(|text| format!("{}%", literally(text))),
                ],
                kept_session,
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Everything `holder` has taken or done, oldest first.
    pub fn attachments_of(&self, holder: &Holder) -> Result<Vec<KeptAttachment>, WriteError> {
        self.attachments_where(
            "holder_kind = ?1 AND holder_id = ?2",
            params![holder.kind.as_text(), holder.id],
        )
    }

    /// Every holder of `kind` at `target`, standing ones first, then the most
    /// recently changed. `manifest_id` narrows to one repository's.
    pub fn attachments_at(
        &self,
        kind: &str,
        target: &str,
        manifest_id: Option<&str>,
    ) -> Result<Vec<KeptAttachment>, WriteError> {
        self.attachments_where(
            "kind = ?1 AND target = ?2 AND (?3 IS NULL OR manifest_id = ?3)",
            params![kind, target, manifest_id],
        )
    }

    fn attachments_where(
        &self,
        clause: &str,
        arguments: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<KeptAttachment>, WriteError> {
        let doing = "reading the session ledger";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {ATTACHMENT_COLUMNS} FROM ledger_attachments WHERE {clause}
                 ORDER BY state = 'standing' DESC, changed_at DESC, since"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(arguments, kept_attachment)
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut kept = Vec::new();
        for row in rows {
            if let Some(row) = row.map_err(fault(doing)).map_err(WriteError::Database)? {
                kept.push(row);
            }
        }
        Ok(kept)
    }

    /// The standing rows of `kind` in one repository, in the order they were
    /// taken: `since`, then the holder's id. `target` narrows to one. **The
    /// order a need is served in**, which `attachments_at` does not give: that
    /// one answers who holds a thing now, this one who is waiting for it.
    pub fn standing_in_order(
        &self,
        kind: &str,
        manifest_id: &str,
        target: Option<&str>,
    ) -> Result<Vec<KeptAttachment>, WriteError> {
        let doing = "reading the session ledger";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {ATTACHMENT_COLUMNS} FROM ledger_attachments
                 WHERE kind = ?1 AND manifest_id = ?2 AND state = 'standing'
                   AND (?3 IS NULL OR target = ?3)
                 ORDER BY since, holder_id, target"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![kind, manifest_id, target], kept_attachment)
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut kept = Vec::new();
        for row in rows {
            if let Some(row) = row.map_err(fault(doing)).map_err(WriteError::Database)? {
                kept.push(row);
            }
        }
        Ok(kept)
    }

    /// Move everything `holder` still holds, of `kind` or of every kind, to
    /// `state`: a need spent where its Job landed, given back where it was
    /// dropped. `true` where anything changed.
    pub fn settle_held(
        &mut self,
        holder: &Holder,
        kind: Option<&str>,
        state: AttachmentState,
        at: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE ledger_attachments SET state = ?5, changed_at = ?4
                 WHERE holder_kind = ?1 AND holder_id = ?2 AND state = 'standing'
                   AND (?3 IS NULL OR kind = ?3)",
                params![holder.kind.as_text(), holder.id, kind, at, state.as_text()],
            )
            .map(|rows| rows > 0)
            .map_err(fault("settling what a holder held"))
            .map_err(WriteError::Database)
    }

    /// Take `attachment`: a new row, or the existing one made standing again
    /// with its detail replaced. **`exclusive` gives back every other standing
    /// row of the same holder, kind and repository** — a session is on one
    /// branch and in one slot at a time. `true` where anything changed.
    pub fn attach(
        &mut self,
        attachment: &KeptAttachment,
        exclusive: bool,
    ) -> Result<bool, WriteError> {
        let doing = "taking an attachment";
        let wrap = |why| WriteError::Database(fault(doing)(why));
        let holder = (attachment.holder.kind.as_text(), &attachment.holder.id);
        let transaction = self.conn.transaction().map_err(wrap)?;
        let mut changed = 0;
        if exclusive {
            changed += transaction
                .execute(
                    "UPDATE ledger_attachments SET state = 'given_back', changed_at = ?6
                     WHERE holder_kind = ?1 AND holder_id = ?2 AND kind = ?3
                       AND manifest_id = ?4 AND target <> ?5 AND state = 'standing'",
                    params![
                        holder.0,
                        holder.1,
                        attachment.kind,
                        attachment.manifest_id,
                        attachment.target,
                        attachment.changed_at
                    ],
                )
                .map_err(wrap)?;
        }
        let detail = serde_json::to_string(&attachment.detail).unwrap_or_else(|_| "{}".into());
        changed += transaction
            .execute(
                "INSERT INTO ledger_attachments (holder_kind, holder_id, kind, manifest_id, \
                 target, state, detail, since, changed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT (holder_kind, holder_id, kind, manifest_id, target)
                 DO UPDATE SET since = CASE WHEN state <> excluded.state
                                            THEN excluded.since ELSE since END,
                               state = excluded.state, detail = excluded.detail,
                               changed_at = excluded.changed_at
                 WHERE state <> excluded.state OR detail <> excluded.detail",
                params![
                    holder.0,
                    holder.1,
                    attachment.kind,
                    attachment.manifest_id,
                    attachment.target,
                    attachment.state.as_text(),
                    detail,
                    attachment.changed_at,
                ],
            )
            .map_err(wrap)?;
        transaction.commit().map_err(wrap)?;
        Ok(changed > 0)
    }

    /// Move one attachment to `state`. `true` where the row existed and was not
    /// there already.
    pub fn settle(
        &mut self,
        holder: &Holder,
        kind: &str,
        manifest_id: &str,
        target: &str,
        state: AttachmentState,
        at: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE ledger_attachments SET state = ?6, changed_at = ?7
                 WHERE holder_kind = ?1 AND holder_id = ?2 AND kind = ?3
                   AND manifest_id = ?4 AND target = ?5 AND state <> ?6",
                params![
                    holder.kind.as_text(),
                    holder.id,
                    kind,
                    manifest_id,
                    target,
                    state.as_text(),
                    at
                ],
            )
            .map(|rows| rows > 0)
            .map_err(fault("settling an attachment"))
            .map_err(WriteError::Database)
    }

    /// Give back everything `holder` still holds, of `kind` or of every kind.
    /// What a session ending does to its slot.
    pub fn give_back(
        &mut self,
        holder: &Holder,
        kind: Option<&str>,
        at: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE ledger_attachments SET state = 'given_back', changed_at = ?4
                 WHERE holder_kind = ?1 AND holder_id = ?2 AND state = 'standing'
                   AND (?3 IS NULL OR kind = ?3)",
                params![holder.kind.as_text(), holder.id, kind, at],
            )
            .map(|rows| rows > 0)
            .map_err(fault("giving back what a holder held"))
            .map_err(WriteError::Database)
    }
}
