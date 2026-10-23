//! The Logbook: what Armada learned, read, and the acts a person has on an entry.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own, and a handler
//! that takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    EditLogbookEntry, Logbook as LogbookRead, LogbookEntry, ForgottenLogbookEntry, LogbookSection, LogbookState,
    LockLogbookEntry, TeachLogbookEntry, TidiedLogbook, TidyLogbook,
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

    /// `tidy_logbook` — fold near-duplicate entries now, of one repository or of every one. The
    /// answer counts the entries folded into another. [`Refusal::Fault`] where the model call
    /// fails.
    fn tidy_logbook(&self, tidy: TidyLogbook) -> impl Future<Output = Result<TidiedLogbook, Refusal>> + Send;

    /// `teach_logbook_entry` — an entry a person wrote, standing at once. [`Refusal::Unacceptable`]
    /// for a `repo` entry naming no repository, or for text that is empty or too long;
    /// [`Refusal::NotFound`] for a root Fleet does not serve.
    fn teach_logbook_entry(
        &self,
        teach: TeachLogbookEntry,
    ) -> impl Future<Output = Result<LogbookEntry, Refusal>> + Send;
}
