//! The drafts the owner and an agent edit before they are published: a workflow's, or a lifecycle's, rule's or autonomy
//! default's. `docs/concepts/workflow.md`, *Drafts and Publish*.
//!
//! **A draft is one place's, one thing's.** It is keyed by kind (`workflow` or a piece's kind), the place (`source`, and
//! `project` where that is a project's), the repository where the place is `repository` and empty everywhere else (Kit's, a
//! project's and Armada's are the same in every repository), a piece's scope, and the id. **Its text is its last change's**,
//! each change holding the whole text after it, so stepping back is putting an earlier change's text again as a new one.
//! `base` is the published text the draft began from, which a publish is held against.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The change was made by the owner.
pub const BY_YOU: &str = "you";
/// The change was made by an agent.
pub const BY_AGENT: &str = "agent";

/// Which draft: the key a draft is kept under.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftKey {
    /// `workflow`, or a piece's kind.
    pub kind: String,
    /// `armada`, `kit`, `repository` or `project` for a workflow; `kit` for a piece.
    pub source: String,
    /// The project, where `source` is `project`; empty otherwise.
    pub project: String,
    /// The repository, only where `source` is `repository`; empty otherwise.
    pub manifest_id: String,
    /// A piece's scope as a word (`everywhere` or `within:<id>`); empty for a workflow.
    pub scope: String,
    /// The workflow's or the piece's id.
    pub item_id: String,
}

/// One change to a draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDraftChange {
    pub id: i64,
    /// [`BY_YOU`] or [`BY_AGENT`].
    pub made_by: String,
    pub at: String,
    /// The whole draft after this change.
    pub text: String,
    pub summary: Option<String>,
    /// The workflow change an agent's change came from.
    pub change: Option<i64>,
}

/// One draft, with its changes oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDraft {
    pub key: DraftKey,
    /// The published text the draft began from; for Armada's, the carried text. Empty where nothing was published.
    pub base: String,
    pub started_at: String,
    pub updated_at: String,
    pub changes: Vec<KeptDraftChange>,
}

impl KeptDraft {
    /// The draft's text: its last change's.
    pub fn text(&self) -> &str {
        self.changes.last().map_or("", |change| change.text.as_str())
    }
}

/// A change about to be kept.
pub struct NewDraftChange<'a> {
    pub made_by: &'a str,
    pub text: &'a str,
    pub summary: Option<&'a str>,
    pub change: Option<i64>,
}

fn change_of(row: &Row<'_>) -> rusqlite::Result<KeptDraftChange> {
    Ok(KeptDraftChange { id: row.get(0)?, made_by: row.get(1)?, at: row.get(2)?, text: row.get(3)?, summary: row.get(4)?, change: row.get(5)? })
}

impl Store {
    /// The draft under `key`, with its changes, or `None` where there is none.
    pub fn workflow_draft(&self, key: &DraftKey) -> Result<Option<KeptDraft>, WriteError> {
        let doing = "reading a draft";
        let head = self
            .conn
            .query_row(
                "SELECT id, base, started_at, updated_at FROM workflow_drafts
                 WHERE kind = ?1 AND source = ?2 AND project = ?3 AND manifest_id = ?4 AND scope = ?5 AND item_id = ?6",
                params![key.kind, key.source, key.project, key.manifest_id, key.scope, key.item_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
            )
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let Some((id, base, started_at, updated_at)) = head else { return Ok(None) };
        let mut statement = self
            .conn
            .prepare("SELECT id, made_by, at, text, summary, change_id FROM workflow_draft_changes WHERE draft_id = ?1 ORDER BY id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let changes = statement
            .query_map([id], change_of)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(Some(KeptDraft { key: key.clone(), base, started_at, updated_at, changes }))
    }

    /// Add a change to the draft under `key`, **beginning the draft from `base` where there is none**, at `at`. The
    /// draft as it stands after the change.
    pub fn put_draft_change(&mut self, key: &DraftKey, base: &str, new: &NewDraftChange<'_>, at: &str) -> Result<KeptDraft, WriteError> {
        let doing = "keeping a change to a draft";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.execute(
            "INSERT OR IGNORE INTO workflow_drafts (kind, source, project, manifest_id, scope, item_id, base, started_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![key.kind, key.source, key.project, key.manifest_id, key.scope, key.item_id, base, at],
        )
        .map_err(fault(doing))
        .map_err(WriteError::Database)?;
        let draft: i64 = tx
            .query_row(
                "SELECT id FROM workflow_drafts
                 WHERE kind = ?1 AND source = ?2 AND project = ?3 AND manifest_id = ?4 AND scope = ?5 AND item_id = ?6",
                params![key.kind, key.source, key.project, key.manifest_id, key.scope, key.item_id],
                |row| row.get(0),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        tx.execute(
            "INSERT INTO workflow_draft_changes (draft_id, made_by, at, text, summary, change_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![draft, new.made_by, at, new.text, new.summary, new.change],
        )
        .map_err(fault(doing))
        .map_err(WriteError::Database)?;
        tx.execute("UPDATE workflow_drafts SET updated_at = ?2 WHERE id = ?1", params![draft, at])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)?;
        self.workflow_draft(key)?.ok_or_else(|| WriteError::Database(fault(doing)(rusqlite::Error::QueryReturnedNoRows)))
    }

    /// Discard the draft under `key` and its changes. Whether there was one.
    pub fn discard_draft(&mut self, key: &DraftKey) -> Result<bool, WriteError> {
        let doing = "discarding a draft";
        let tx = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        let found: Option<i64> = tx
            .query_row(
                "SELECT id FROM workflow_drafts
                 WHERE kind = ?1 AND source = ?2 AND project = ?3 AND manifest_id = ?4 AND scope = ?5 AND item_id = ?6",
                params![key.kind, key.source, key.project, key.manifest_id, key.scope, key.item_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let Some(draft) = found else { return Ok(false) };
        tx.execute("DELETE FROM workflow_draft_changes WHERE draft_id = ?1", [draft]).map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.execute("DELETE FROM workflow_drafts WHERE id = ?1", [draft]).map_err(fault(doing)).map_err(WriteError::Database)?;
        tx.commit().map_err(fault(doing)).map_err(WriteError::Database)?;
        Ok(true)
    }
}
