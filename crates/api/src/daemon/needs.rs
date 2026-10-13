//! `armada need`: what a checkout says it needs on a path, kept on the session
//! ledger. Since 23.46. `docs/capabilities/needs.md`.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its
//! own, and a handler that takes only this cannot reach for a Job.

use std::future::Future;

use ipc::{ManifestId, NeedAnswer, NeedCall, NeedList, WriteClashes};

use crate::daemon::Refusal;

pub trait Needs: Send + Sync + 'static {
    /// `act_on_need` — declare, say what was taken, or give back.
    /// [`Refusal::Unacceptable`] for a blank branch or path, a declaration with
    /// no `what`, a `took` with no `value`, and a `took` on a need the holder
    /// never declared.
    fn act_on_need(
        &self,
        call: NeedCall,
    ) -> impl Future<Output = Result<NeedAnswer, Refusal>> + Send;

    /// `list_needs` — every standing need of one repository, by path and in
    /// order. `manifest_id` absent is the first served.
    fn list_needs(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<NeedList, Refusal>> + Send;

    /// `list_write_clashes` — the clashes standing in one repository, or those
    /// `holder` (a Job's or a Session's id) is in. `manifest_id` absent is the
    /// first served. Read from what Fleet last checked; asks git for nothing.
    fn list_write_clashes(
        &self,
        manifest_id: Option<ManifestId>,
        holder: Option<String>,
    ) -> impl Future<Output = Result<WriteClashes, Refusal>> + Send;
}
