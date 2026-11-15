//! A side question put to a Job, and the owner's act on its answer. Its own surface, for
//! [`SharedHome`](super::SharedHome)'s reason: a handler taking only this cannot reach for a Job's record, and an
//! aside is kept out of it.

use std::future::Future;

use ipc::{Aside, AsideList, AskAside, SessionId, TellAside};

use crate::daemon::Refusal;

pub trait Asides: Send + Sync + 'static {
    /// `ask_aside` — one read-only turn on a thrown-away copy of the Job's conversation. **Answers at once**, with
    /// the aside `asking`; the turn runs in a task and its answer arrives on `session.aside`.
    /// [`Refusal::NotFound`] (`fleet.no_such_session`) for no Job Fleet hosts, [`Refusal::Unacceptable`]
    /// (`fleet.aside_blank`) for a blank question, (`fleet.fork_point_unwritten`) for a Job with no finished turn.
    fn ask_aside(self: std::sync::Arc<Self>, ask: AskAside) -> impl Future<Output = Result<Aside, Refusal>> + Send;

    /// `list_asides` — the Job's asides, oldest first. [`Refusal::NotFound`] (`fleet.no_such_session`).
    fn list_asides(&self, session_id: SessionId) -> impl Future<Output = Result<AsideList, Refusal>> + Send;

    /// `tell_aside` — an answered aside goes to the Job as the owner's message, once. [`Refusal::NotFound`]
    /// (`fleet.no_such_session`, `fleet.no_such_aside`), [`Refusal::IllegalMove`] (`fleet.aside_not_answered`,
    /// `fleet.aside_told`).
    fn tell_aside(self: std::sync::Arc<Self>, tell: TellAside) -> impl Future<Output = Result<Aside, Refusal>> + Send;
}
