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
                "SELECT kind, manifest_id, target, state, detail, at
                 FROM ledger_attachment_versions
                 WHERE holder_kind = ?1 AND holder_id = ?2
                 ORDER BY seq",
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
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
