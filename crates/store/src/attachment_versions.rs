//! Every version a ledger attachment has stood in: the row is replaced in place, so
//! a trigger on `ledger_attachments` writes each insert and each change of state or
//! detail here, and nothing in Rust has to remember to. A Job's artifacts read back
//! as a timeline from it.

use std::collections::BTreeMap;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::session_ledger::{AttachmentState, Holder};

/// One attachment as it stood from `at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAttachmentVersion {
    pub kind: String,
    pub manifest_id: String,
    pub target: String,
    pub state: AttachmentState,
    pub detail: BTreeMap<String, String>,
    pub at: String,
    /// The thread row id of the person's latest message when it was written.
    pub from_message: Option<String>,
    /// The plan task in progress when it was written.
    pub from_task: Option<String>,
}

impl Store {
    /// Every version of everything `holder` has taken or done, oldest first.
    pub fn attachment_versions(
        &self,
        holder: &Holder,
    ) -> Result<Vec<KeptAttachmentVersion>, WriteError> {
        let doing = "reading the job ledger's history";
        let mut statement = self
            .conn
            .prepare(
                "SELECT v.kind, v.manifest_id, v.target, v.state, v.detail, v.at, o.from_message, o.from_task
                 FROM ledger_attachment_versions v
                 LEFT JOIN ledger_attachment_version_origins o ON o.seq = v.seq
                 WHERE v.holder_kind = ?1 AND v.holder_id = ?2
                 ORDER BY v.seq",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(rusqlite::params![holder.kind.as_text(), holder.id], |row| {
                let detail: String = row.get(4)?;
                Ok(KeptAttachmentVersion {
                    kind: row.get(0)?,
                    manifest_id: row.get(1)?,
                    target: row.get(2)?,
                    state: AttachmentState::of(&row.get::<_, String>(3)?),
                    detail: serde_json::from_str(&detail).unwrap_or_default(),
                    at: row.get(5)?,
                    from_message: row.get(6)?,
                    from_task: row.get(7)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
