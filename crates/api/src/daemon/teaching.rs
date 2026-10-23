//! Teaching: something a person pressed Teach on, read into a proposal, and the Logbook entry and
//! mods they keep from it.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own, and a handler
//! that takes only this cannot reach for anything else a command does.

use std::future::Future;
use std::sync::Arc;

use ipc::{ReadTeaching, SaveTeaching, SavedTeaching, TeachingProposal, WordRules};

use crate::daemon::Refusal;

pub trait Teaching: Send + Sync + 'static {
    /// `read_teaching` — one model call into a proposal, keeping nothing. [`Refusal::Unacceptable`]
    /// for words or a reply that is empty or too long, a part that could not work (a pattern Bridge
    /// could not run, a note that does not fit, a rule for a notice Fleet does not send, a tweak or
    /// a layout the checks refuse) or an answer with nothing the source can make;
    /// [`Refusal::NotFound`] for a root Fleet does not serve; [`Refusal::Fault`] where the call failed.
    fn read_teaching(&self, teach: ReadTeaching) -> impl Future<Output = Result<TeachingProposal, Refusal>> + Send;

    /// `save_teaching` — make the Logbook entry for the note and write the mods, held to what
    /// `read_teaching` holds a proposal to, and tell the Session named, where one is, the note.
    /// **Takes the `Arc`** because telling a Session starts a turn on a task, and so does looking for
    /// an address. [`Refusal::Unacceptable`] for a body that carries nothing to keep or a part that
    /// could not work.
    fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> impl Future<Output = Result<SavedTeaching, Refusal>> + Send;

    /// `list_word_rules` — the patterns that have an address for `repository`. **Takes the `Arc`**
    /// because a pattern with none starts a search for one on a task of its own, off this call.
    /// [`Refusal::NotFound`] for a root Fleet does not serve.
    fn list_word_rules(
        self: Arc<Self>,
        repository: Option<String>,
        session_id: Option<String>,
    ) -> impl Future<Output = Result<WordRules, Refusal>> + Send;
}
