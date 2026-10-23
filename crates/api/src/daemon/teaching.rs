//! Teaching: words a person selected and said what to do with, read into a proposal, and the link
//! rules they keep from it.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own, and a handler
//! that takes only this cannot reach for anything else a command does.

use std::future::Future;
use std::sync::Arc;

use ipc::{ForgottenTaughtLink, ReadTeaching, SaveTeaching, SavedTeaching, TaughtLinks, TeachingProposal};

use crate::daemon::Refusal;

pub trait Teaching: Send + Sync + 'static {
    /// `read_teaching` — one model call into a proposal, keeping nothing. [`Refusal::Unacceptable`]
    /// for words or a reply that is empty or too long, a link Bridge could not run, a note that does
    /// not fit, or an answer with neither; [`Refusal::NotFound`] for a root Fleet does not serve;
    /// [`Refusal::Fault`] where the call failed.
    fn read_teaching(&self, teach: ReadTeaching) -> impl Future<Output = Result<TeachingProposal, Refusal>> + Send;

    /// `save_teaching` — keep the link rule, held to what `read_teaching` holds a proposal to, and
    /// tell the Session named, where one is, the note it carries. **Takes the `Arc`** because
    /// telling a Session starts a turn on a task. [`Refusal::Unacceptable`] for a body that carries
    /// nothing to keep or a link that cannot work.
    fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> impl Future<Output = Result<SavedTeaching, Refusal>> + Send;

    /// `list_taught_links` — the rules for `repository` and for every repository. With none named,
    /// the every-repository ones alone. [`Refusal::NotFound`] for a root Fleet does not serve.
    fn list_taught_links(&self, repository: Option<String>) -> impl Future<Output = Result<TaughtLinks, Refusal>> + Send;

    /// `forget_taught_link` — [`Refusal::NotFound`] for an id that names none, the second time too.
    fn forget_taught_link(&self, link_id: String) -> impl Future<Output = Result<ForgottenTaughtLink, Refusal>> + Send;
}
