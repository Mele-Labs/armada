//! One finished run as its own directory keeps it, and the shape it is
//! answered in. Fields an older record carries that are no longer here are
//! ignored on read.

use ipc::{ChangedFile, CheckoutRunRecord, Instant};
use serde::{Deserialize, Serialize};

/// A run's `run.json`. Not an `ipc` type: it is Fleet's own format for its own
/// directory, and the wire shapes below are projections of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) required: Vec<String>,
    pub(crate) started_at: Instant,
    pub(crate) ended_at: Instant,
    pub(crate) duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i32>,
    pub(crate) expect_exit_code: i64,
    pub(crate) ended: String,
    pub(crate) stopped: bool,
    pub(crate) changed: Vec<ChangedFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) changed_unreadable: Option<String>,
    /// Where the tree before the run is kept. Absent: nothing to undo from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) undone_at: Option<Instant>,
    pub(crate) log: String,
    /// The workspace whose own file declared it. Absent is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) workspace: Option<String>,
    /// Who asked for it. **Absent on a record written before 23.40 reads as
    /// outside**, which is what every such run was.
    #[serde(default)]
    pub(crate) requester: ipc::Requester,
}

impl Record {
    /// This record as the Manifest surface reads it.
    ///
    /// **`undoable` is the snapshot, answered rather than handed over.** What
    /// a surface in the main checkout has to know is whether Undo is there to
    /// offer, because this tree holds a person's own uncommitted work — the
    /// reference it would be restored from is Fleet's business.
    pub(crate) fn of_checkout(&self) -> CheckoutRunRecord {
        CheckoutRunRecord {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            command: self.command.clone(),
            required: self.required.clone(),
            started_at: self.started_at.clone(),
            ended_at: self.ended_at.clone(),
            duration_ms: self.duration_ms,
            exit_code: self.exit_code,
            expect_exit_code: self.expect_exit_code,
            ended: self.ended.clone(),
            stopped: self.stopped,
            changed: self.changed.clone(),
            changed_unreadable: self.changed_unreadable.clone(),
            undoable: self.snapshot.is_some() && self.undone_at.is_none(),
            undone_at: self.undone_at.clone(),
            log: self.log.clone(),
            requester: self.requester.clone(),
        }
    }
}

/// A run that has started and not finished, as [`Rehearsals`] holds it.
///
/// [`Record`]'s reasoning one moment earlier: one shape while the run is
/// Fleet's own, projected when it is answered.
///
/// [`Rehearsals`]: super::in_flight::Rehearsals
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Underway {
    pub(crate) id: String,
    pub(crate) name: String,
    /// The command line that is running.
    pub(crate) command: String,
    pub(crate) started_at: Instant,
    pub(crate) workspace: Option<String>,
    pub(crate) requester: ipc::Requester,
}

impl Underway {

    pub(crate) fn of_checkout(&self) -> ipc::CheckoutRunUnderway {
        ipc::CheckoutRunUnderway {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            started_at: self.started_at.clone(),
            workspace: self.workspace.clone(),
            requester: self.requester.clone(),
        }
    }
}
