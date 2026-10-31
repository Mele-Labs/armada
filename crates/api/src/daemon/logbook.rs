//! The Logbook: what Armada learned, read, and the acts a person has on an entry.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own, and a handler
//! that takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    EditLogbookEntry, Logbook as LogbookRead, LogbookEntry, ForgottenLogbookEntry, LogbookSection, LogbookState,
    LockLogbookEntry, TeachLogbookEntry, TidiedLogbook, TidyLogbook, AppliedOrganizeLogbook, ApplyOrganizeLogbook,
    OrganizeLogbook, OrganizedLogbook,
};

use crate::daemon::Refusal;

pub trait Logbook: Send + Sync + 'static {
    /// `get_logbook` — the entries of `repository` where one is named, with the habits and this
    /// Mac's, narrowed by `section` and `state`. [`Refusal::NotFound`] where Fleet does not serve
    /// the root.
    fn get_logbook(
        &self,
        repository: Option<String>,
        section: Option<LogbookSection>,
        state: Option<LogbookState>,
    ) -> impl Future<Output = Result<LogbookRead, Refusal>> + Send;

    /// `edit_logbook_entry` — replace the text, keeping the one it had. [`Refusal::NotFound`] for
    /// no such entry; [`Refusal::Unacceptable`] for one run memory or the machine note keeps, or
    /// for text that is empty, too long or forgotten.
    fn edit_logbook_entry(
        &self,
        entry_id: String,
        edit: EditLogbookEntry,
    ) -> impl Future<Output = Result<LogbookEntry, Refusal>> + Send;

    /// `lock_logbook_entry` — lock it or let it go. Refused as `edit_logbook_entry` is.
    fn lock_logbook_entry(
        &self,
        entry_id: String,
        lock: LockLogbookEntry,
    ) -> impl Future<Output = Result<LogbookEntry, Refusal>> + Send;

    /// `forget_logbook_entry` — onto the deny list, for good. [`Refusal::NotFound`] the second time.
    fn forget_logbook_entry(&self, entry_id: String) -> impl Future<Output = Result<ForgottenLogbookEntry, Refusal>> + Send;

    /// `tidy_logbook` — start folding near-duplicate entries, of one repository or of every one, on
    /// a task of its own. **Answers at once** that a pass is running, and starts none where one
    /// is; what it merged is in `get_logbook`. [`Refusal::Fault`] where the call cannot be put
    /// together on this machine.
    fn tidy_logbook(self: std::sync::Arc<Self>, tidy: TidyLogbook) -> impl Future<Output = Result<TidiedLogbook, Refusal>> + Send;

    /// `organize_logbook` — propose merges, stale entries or Armada faults; writes nothing. Waits
    /// for the model calls. [`Refusal::Fault`] where a call cannot be put together on this machine.
    fn organize_logbook(&self, organize: OrganizeLogbook) -> impl Future<Output = Result<OrganizedLogbook, Refusal>> + Send;

    /// `apply_organize_logbook` — carry out proposals as `organize_logbook` gave them.
    fn apply_organize_logbook(
        &self,
        apply: ApplyOrganizeLogbook,
    ) -> impl Future<Output = Result<AppliedOrganizeLogbook, Refusal>> + Send;

    /// `teach_logbook_entry` — an entry a person wrote, standing at once. [`Refusal::Unacceptable`]
    /// for a `repo` entry naming no repository, or for text that is empty or too long;
    /// [`Refusal::NotFound`] for a root Fleet does not serve.
    fn teach_logbook_entry(
        &self,
        teach: TeachLogbookEntry,
    ) -> impl Future<Output = Result<LogbookEntry, Refusal>> + Send;
}
